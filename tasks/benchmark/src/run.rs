//! Runs `earnest verify` once on some files and measures its wall time and peak memory, and reads
//! the time of the search that the checker reports for each model.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::models::expected_states;

const POLL: Duration = Duration::from_millis(1);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Seen {
    pub(crate) name: String,
    pub(crate) states: u64,
    pub(crate) search: Duration,
}

#[derive(Debug)]
pub(crate) struct Sample {
    pub(crate) wall: Duration,
    pub(crate) peak_kib: Option<u64>,
    pub(crate) models: Vec<Seen>,
}

impl Sample {
    pub(crate) fn search(&self) -> Duration {
        self.models.iter().map(|seen| seen.search).sum()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Job<'a> {
    pub(crate) binary: &'a Path,
    pub(crate) files: &'a [PathBuf],
    pub(crate) workers: Option<usize>,
    pub(crate) max_states: Option<u64>,
    pub(crate) scratch: &'a Path,
}

pub(crate) fn peak_from_status(status: &str) -> Option<u64> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|number| number.parse().ok())
}

fn peak_of(process: u32) -> Option<u64> {
    let status = fs::read_to_string(format!("/proc/{process}/status")).ok()?;
    peak_from_status(&status)
}

pub(crate) fn processor_name() -> Option<String> {
    if cfg!(target_os = "linux") {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo").ok()?;
        model_name_from_cpuinfo(&cpuinfo)
    } else if cfg!(target_os = "macos") {
        let output = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()?;
        non_empty(String::from_utf8_lossy(&output.stdout).trim())
    } else {
        None
    }
}

fn model_name_from_cpuinfo(cpuinfo: &str) -> Option<String> {
    cpuinfo
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(key, _)| key.trim() == "model name")
        .and_then(|(_, value)| non_empty(value.trim()))
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

pub(crate) fn parse_results(json: &str) -> Result<Vec<Seen>, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| format!("the output is not JSON: {error}"))?;
    let models = value["models"]
        .as_array()
        .ok_or("the JSON has no list of models")?;
    if value["unreadable"]
        .as_array()
        .is_some_and(|list| !list.is_empty())
    {
        return Err("a benchmark model is unreadable".to_owned());
    }
    models
        .iter()
        .map(|model| {
            let name = model["name"].as_str().ok_or("a model has no name")?;
            let status = model["status"].as_str().unwrap_or("missing");
            if status != "pass" {
                return Err(format!("{name} is {status}, not pass"));
            }
            let counted = model["states"]
                .as_u64()
                .ok_or_else(|| format!("{name} has no state count"))?;
            let milliseconds = model["milliseconds"]
                .as_u64()
                .ok_or_else(|| format!("{name} has no search time"))?;
            Ok(Seen {
                name: name.to_owned(),
                states: counted,
                search: Duration::from_millis(milliseconds),
            })
        })
        .collect()
}

pub(crate) fn check_counts(models: &[Seen]) -> Result<(), String> {
    for seen in models {
        let expected = expected_states(&seen.name)
            .ok_or_else(|| format!("{} is not a benchmark model", seen.name))?;
        if seen.states != expected {
            return Err(format!(
                "{}: the checker counted {} states, but README.md derives {expected}",
                seen.name, seen.states
            ));
        }
    }
    Ok(())
}

pub(crate) fn run_once(job: &Job<'_>) -> Result<Sample, String> {
    let mut command = Command::new(job.binary);
    command.args(["verify", "--no-cache", "--json"]);
    if let Some(workers) = job.workers {
        command.args(["--jobs", &workers.to_string()]);
    }
    if let Some(limit) = job.max_states {
        command.args(["--max-states", &limit.to_string()]);
    }
    let started = Instant::now();
    let child = command
        .args(job.files)
        .current_dir(job.scratch)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{} did not start: {error}", job.binary.display()))?;
    let process = child.id();
    let (wall, peak_kib, waited) = thread::scope(|scope| {
        let waiting = earnest_threads::start(scope, None, move || child.wait_with_output())
            .map_err(|error| {
                format!("the thread that waits for the checker did not start: {error}")
            })?;
        let mut peak_kib = None;
        while !waiting.is_finished() {
            peak_kib = peak_kib.max(peak_of(process));
            thread::sleep(POLL);
        }
        let wall = started.elapsed();
        let waited = waiting
            .join()
            .map_err(|_| "the thread that waits for the checker panicked".to_owned())?;
        Ok::<_, String>((wall, peak_kib, waited))
    })?;
    let output = waited.map_err(|error| format!("waiting for the checker failed: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        return Err(format!(
            "the checker exited with {}: {stdout}{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let models = parse_results(&stdout)?;
    check_counts(&models)?;
    Ok(Sample {
        wall,
        peak_kib,
        models,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPYC: &str = "AMD EPYC 7763 64-Core Processor";

    #[test]
    fn processor_name_is_the_first_model_name_of_a_two_processor_file() {
        let cpuinfo = format!(
            "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t: {EPYC}\ncpu MHz\t\t: 2445.4\n\nprocessor\t: 1\nvendor_id\t: AuthenticAMD\nmodel name\t: Other Processor\n"
        );
        assert_eq!(
            model_name_from_cpuinfo(&cpuinfo),
            Some(EPYC.to_owned()),
            "the first of two"
        );
    }

    #[test]
    fn processor_name_ignores_the_tabs_before_the_colon() {
        assert_eq!(
            model_name_from_cpuinfo(&format!("model name\t: {EPYC}")),
            Some(EPYC.to_owned()),
            "a tab before the colon, no newline at the end"
        );
    }

    #[test]
    fn processor_name_is_unknown_in_arm_style_text_without_a_model_name() {
        let cpuinfo =
            "processor\t: 0\nBogoMIPS\t: 50.00\nCPU implementer\t: 0x41\nCPU part\t: 0xd0c\n";
        assert_eq!(model_name_from_cpuinfo(cpuinfo), None, "no model name line");
    }

    #[test]
    fn processor_name_is_unknown_in_empty_text() {
        assert_eq!(model_name_from_cpuinfo(""), None, "empty text");
    }

    #[test]
    fn processor_name_is_unknown_when_the_model_name_has_no_value() {
        assert_eq!(
            model_name_from_cpuinfo("model name\t:  \nmodel name\t: Later\n"),
            None,
            "an empty value is not skipped for a later line"
        );
    }

    #[test]
    fn peak_is_read_from_the_status_file() {
        let status = "Name:\tearnest\nVmPeak:\t  900 kB\nVmHWM:\t  123456 kB\nVmRSS:\t  100 kB\n";
        assert_eq!(
            peak_from_status(status),
            Some(123_456),
            "the high-water mark"
        );
        assert_eq!(peak_from_status("Name:\tearnest\n"), None, "no mark");
    }

    fn counter(states: u64) -> Seen {
        Seen {
            name: "Counter".to_owned(),
            states,
            search: Duration::from_millis(7),
        }
    }

    #[test]
    fn results_give_each_models_name_states_and_search_time() {
        let json = r#"{"models":[{"name":"Counter","status":"pass","states":100,"milliseconds":7}],"unreadable":[]}"#;
        assert_eq!(
            parse_results(json),
            Ok(vec![counter(100)]),
            "one passing model"
        );
    }

    #[test]
    fn a_model_without_a_search_time_is_an_error() {
        let json =
            r#"{"models":[{"name":"Counter","status":"pass","states":100}],"unreadable":[]}"#;
        assert_eq!(
            parse_results(json),
            Err("Counter has no search time".to_owned()),
            "no milliseconds"
        );
    }

    #[test]
    fn the_search_time_of_a_run_is_the_sum_over_its_models() {
        let sample = Sample {
            wall: Duration::from_millis(50),
            peak_kib: None,
            models: vec![counter(100), counter(100)],
        };
        assert_eq!(
            sample.search(),
            Duration::from_millis(14),
            "two models of 7 ms"
        );
    }

    #[test]
    fn a_model_that_does_not_pass_is_an_error() {
        let json =
            r#"{"models":[{"name":"Counter","status":"fail","states":100}],"unreadable":[]}"#;
        assert_eq!(
            parse_results(json),
            Err("Counter is fail, not pass".to_owned()),
            "a failing model"
        );
        assert!(parse_results("not json").is_err(), "text is not JSON");
        let unreadable = r#"{"models":[],"unreadable":[{"file":"a"}]}"#;
        assert!(parse_results(unreadable).is_err(), "an unreadable file");
    }

    #[test]
    fn counts_must_equal_the_derived_ones() {
        assert_eq!(check_counts(&[counter(100)]), Ok(()), "the derived count");
        let wrong = [counter(101)];
        assert_eq!(
            check_counts(&wrong),
            Err("Counter: the checker counted 101 states, but README.md derives 100".to_owned()),
            "a different count fails"
        );
        let stranger = [Seen {
            name: "Stranger".to_owned(),
            ..counter(1)
        }];
        assert!(check_counts(&stranger).is_err(), "an unknown model fails");
    }
}
