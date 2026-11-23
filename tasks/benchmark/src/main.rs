//! Measures how fast the release `earnest verify` is on a fixed set of models, and prints the
//! result as Markdown. It stands in for the speed targets of the spec until the first app's
//! models are available.

mod counts;
mod models;
mod report;
mod run;

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use models::{LARGEST, Language, NARROW_LEVELS, R12_LARGE, SCALED, WIDE};
use report::{Judged, Kind, Limit, Row, Unit, Verdict};
use run::{Job, processor_name, run_once};

type Run = (PathBuf, Option<usize>);

const DEFAULT_RUNS: usize = 5;
const USAGE: &str = "usage: benchmark [--runs <n>] [--binary <path>] [--counts [--record]]";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Time,
    Counts,
    Record,
}

#[derive(Debug)]
struct Options {
    runs: usize,
    binary: PathBuf,
    mode: Mode,
}

#[derive(Clone, Debug)]
struct Measured {
    median: Duration,
    search: Duration,
    peak_kib: Option<u64>,
    states: u64,
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_options(arguments: &[String]) -> Result<Options, String> {
    let mut runs = DEFAULT_RUNS;
    let mut binary = repository().join("target/release/earnest");
    let mut mode = Mode::Time;
    let mut remaining = arguments.iter();
    while let Some(flag) = remaining.next() {
        match (flag.as_str(), mode) {
            ("--counts", Mode::Time) => {
                mode = Mode::Counts;
                continue;
            }
            ("--record", Mode::Counts) => {
                mode = Mode::Record;
                continue;
            }
            _ => {}
        }
        let value = remaining.next();
        match (flag.as_str(), value) {
            ("--runs", Some(count)) => {
                runs = count
                    .parse()
                    .ok()
                    .filter(|count| *count > 0)
                    .ok_or_else(|| format!("--runs needs a whole number above 0. {USAGE}"))?;
            }
            ("--binary", Some(path)) => binary = PathBuf::from(path),
            _ => return Err(format!("{flag} is not understood. {USAGE}")),
        }
    }
    Ok(Options { runs, binary, mode })
}

struct Bench<'a> {
    options: &'a Options,
    scratch: &'a Path,
    cache: HashMap<(Vec<PathBuf>, Option<usize>), Measured>,
}

impl Bench<'_> {
    fn measure(&mut self, files: &[PathBuf], workers: Option<usize>) -> Result<Measured, String> {
        let key = (files.to_vec(), workers);
        if let Some(known) = self.cache.get(&key) {
            return Ok(known.clone());
        }
        let job = Job {
            binary: &self.options.binary,
            files,
            workers,
            max_states: models::state_limit(files),
            scratch: self.scratch,
        };
        let names: Vec<String> = files
            .iter()
            .filter_map(|file| file.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        write_progress(&format!(
            "measuring {} with {} worker(s)",
            names.join(" "),
            workers.map_or_else(|| "all".to_owned(), |count| count.to_string())
        ))
        .map_err(|error| error.to_string())?;
        run_once(&job)?;
        let mut walls = Vec::new();
        let mut searches = Vec::new();
        let mut peak_kib = None;
        let mut states = 0;
        for _ in 0..self.options.runs {
            let sample = run_once(&job)?;
            walls.push(sample.wall);
            searches.push(sample.search());
            peak_kib = peak_kib.max(sample.peak_kib);
            states = sample.models.iter().map(|seen| seen.states).sum();
        }
        let measured = Measured {
            median: report::median(&walls),
            search: report::median(&searches),
            peak_kib,
            states,
        };
        self.cache.insert(key, measured.clone());
        Ok(measured)
    }
}

fn write_progress(message: &str) -> io::Result<()> {
    writeln!(io::stderr().lock(), "{message}")
}

fn row(label: &str, language: &str, measured: &Measured) -> Row {
    Row {
        model: label.to_owned(),
        language: language.to_owned(),
        states: measured.states,
        median: measured.median,
        peak_kib: measured.peak_kib,
    }
}

struct Benchmarked {
    text: String,
    limits: Vec<Limit>,
    cores: usize,
}

fn wall_time(what: String, (measured, run): (&Measured, &Run), allowed: f64, kind: Kind) -> Limit {
    Limit {
        what,
        measured: Some(measured.median.as_secs_f64()),
        again: None,
        runs: vec![run.clone()],
        allowed,
        unit: Unit::Seconds,
        kind,
        judged: Judged::OnTargetCores,
    }
}

fn search_time_all_cores_over_one_worker(
    model: &str,
    file: &Path,
    (all, one): (&Measured, &Measured),
    allowed: f64,
) -> Limit {
    Limit {
        what: format!(
            "{model}, search time with all cores ({:.3} s) over search time with 1 worker ({:.3} s)",
            all.search.as_secs_f64(),
            one.search.as_secs_f64()
        ),
        measured: Some(report::ratio(all.search, one.search)),
        again: None,
        runs: vec![(file.to_path_buf(), None), (file.to_path_buf(), Some(1))],
        allowed,
        unit: Unit::Times,
        kind: Kind::Budget,
        judged: Judged::OnTargetCores,
    }
}

impl Bench<'_> {
    fn all_cores_over_one_worker(
        &mut self,
        repository: &Path,
        model: &str,
        allowed: f64,
    ) -> Result<Limit, String> {
        let file = models::scaled_path(repository, model, Language::TypeScript);
        let all = self.measure(std::slice::from_ref(&file), None)?;
        let one = self.measure(std::slice::from_ref(&file), Some(1))?;
        Ok(search_time_all_cores_over_one_worker(
            model,
            &file,
            (&all, &one),
            allowed,
        ))
    }

    fn limits(&mut self, repository: &Path) -> Result<Vec<Limit>, String> {
        let workers = report::TARGET_WORKERS;
        let largest = models::scaled_path(repository, LARGEST, Language::TypeScript);
        let target_run = (largest.clone(), Some(workers));
        let one_run = (largest, Some(1));
        let target = &self.measure(std::slice::from_ref(&target_run.0), target_run.1)?;
        let one = &self.measure(std::slice::from_ref(&one_run.0), one_run.1)?;
        let (larger, larger_run) = if one.peak_kib > target.peak_kib {
            (one, "1 worker".to_owned())
        } else {
            (target, format!("{workers} workers"))
        };
        Ok(vec![
            wall_time(
                format!("{LARGEST} with {workers} workers, wall time"),
                (target, &target_run),
                f64::from(report::LARGE_TARGET_SECONDS),
                Kind::Target,
            ),
            wall_time(
                format!("{LARGEST} with 1 worker, wall time"),
                (one, &one_run),
                report::LARGEST_ONE_WORKER_BUDGET_SECONDS,
                Kind::Budget,
            ),
            Limit {
                what: format!(
                    "{LARGEST}, peak memory, the larger of {workers} workers and 1 worker ({larger_run})"
                ),
                measured: report::mib(larger.peak_kib),
                again: None,
                runs: vec![target_run.clone(), one_run.clone()],
                allowed: report::LARGEST_PEAK_MEMORY_BUDGET_MIB,
                unit: Unit::Mib,
                kind: Kind::Budget,
                judged: Judged::Anywhere,
            },
            self.all_cores_over_one_worker(
                repository,
                NARROW_LEVELS,
                report::NARROW_LEVELS_ALL_CORES_OVER_ONE_WORKER_BUDGET,
            )?,
            self.all_cores_over_one_worker(
                repository,
                WIDE,
                report::WIDE_ALL_CORES_OVER_ONE_WORKER_BUDGET,
            )?,
        ])
    }

    fn limits_measured_again_where_missed(
        &mut self,
        repository: &Path,
        cores: usize,
    ) -> Result<Vec<Limit>, String> {
        let mut limits = self.limits(repository)?;
        let missed = |limit: &Limit| limit.verdict(cores) == Verdict::Missed;
        let again: Vec<Run> = limits
            .iter()
            .filter(|limit| missed(limit))
            .flat_map(|limit| limit.runs.clone())
            .collect();
        if again.is_empty() {
            return Ok(limits);
        }
        for (file, workers) in again {
            self.cache.remove(&(vec![file], workers));
        }
        let second = self.limits(repository)?;
        for (limit, measured_again) in limits.iter_mut().zip(second) {
            if missed(limit) {
                limit.again = measured_again.measured;
            }
        }
        Ok(limits)
    }
}

fn benchmark(options: &Options) -> Result<Benchmarked, String> {
    if !options.binary.is_file() {
        return Err(format!(
            "{} does not exist; build it with `cargo build --release -p earnest-cli`",
            options.binary.display()
        ));
    }
    let scratch = tempfile::tempdir().map_err(|error| error.to_string())?;
    let repository = repository();
    let mut bench = Bench {
        options,
        scratch: scratch.path(),
        cache: HashMap::new(),
    };

    let mut scaled = Vec::new();
    for entry in SCALED {
        for language in [Language::TypeScript, Language::Rust] {
            let file = models::scaled_path(&repository, entry.file, language);
            let measured = bench.measure(&[file], None)?;
            scaled.push(row(entry.model, language.label(), &measured));
        }
    }

    let mixed = models::mixed_files(&repository);
    let together = bench.measure(&mixed, None)?;
    let mut alone = Vec::new();
    for file in &mixed {
        let measured = bench.measure(std::slice::from_ref(file), None)?;
        let label = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map_or("?", |stem| stem.trim_end_matches(".model"));
        alone.push(row(label, Language::TypeScript.label(), &measured));
    }
    let slowest = alone
        .iter()
        .map(|each| each.median)
        .max()
        .unwrap_or(Duration::ZERO);

    let largest = models::scaled_path(&repository, LARGEST, Language::TypeScript);
    let cores = thread::available_parallelism().map_or(1, usize::from);
    let processor = processor_name();
    let all = bench.measure(std::slice::from_ref(&largest), None)?;
    let one = bench.measure(std::slice::from_ref(&largest), Some(1))?;
    let target = bench.measure(std::slice::from_ref(&largest), Some(report::TARGET_WORKERS))?;

    let r12_large = models::scaled_path(&repository, R12_LARGE, Language::TypeScript);
    let r12_alone = bench.measure(std::slice::from_ref(&r12_large), None)?;
    let mut with_r12_large = mixed.clone();
    with_r12_large.push(r12_large);
    let everything = bench.measure(&with_r12_large, None)?;
    let slowest_row = alone.iter().max_by_key(|each| each.median);
    let r12 = report::r12_lines(
        everything.median,
        with_r12_large.len(),
        report::Alone {
            model: R12_LARGE,
            median: r12_alone.median,
        },
        report::Alone {
            model: slowest_row.map_or("?", |each| each.model.as_str()),
            median: slowest,
        },
    );

    let spec = report::spec_lines(together.median, slowest);
    let together_line = format!(
        "the 12 models in that one run: {} states, peak memory {} MiB",
        report::with_separators(together.states),
        report::memory(together.peak_kib)
    );
    let target_line = report::target_line(LARGEST, target.median, target.peak_kib, cores);
    let speedup = report::speedup_line(LARGEST, one.median, all.median, cores);
    let limits = bench.limits_measured_again_where_missed(&repository, cores)?;
    let text = [
        report::first_line(cores, processor.as_deref(), options.runs),
        "## Scaled models\n".to_owned(),
        report::table(&scaled),
        "## The 12 mixed models, each alone\n".to_owned(),
        report::table(&alone),
        "## Spec targets\n".to_owned(),
        format!(
            "- {}\n- {}\n- {together_line}\n- {target_line}\n",
            spec[0], spec[1]
        ),
        "## One large model on one worker and on all cores\n".to_owned(),
        format!("- {speedup}\n"),
        "## R12: many models and one large model in one run\n".to_owned(),
        format!(
            "- {}\n- {}\n- peak memory {} MiB\n",
            r12[0],
            r12[1],
            report::memory(everything.peak_kib)
        ),
        report::budget_section(&limits, cores),
    ]
    .join("\n");
    Ok(Benchmarked {
        text,
        limits,
        cores,
    })
}

fn printed(benchmarked: &Benchmarked) -> Result<(), String> {
    io::stdout()
        .lock()
        .write_all(benchmarked.text.as_bytes())
        .map_err(|error| error.to_string())?;
    let missed: Vec<&Limit> = benchmarked
        .limits
        .iter()
        .filter(|limit| limit.verdict(benchmarked.cores) == Verdict::Missed)
        .collect();
    let mut errors = io::stderr().lock();
    for limit in &missed {
        writeln!(errors, "benchmark: {}", limit.line(benchmarked.cores))
            .map_err(|error| error.to_string())?;
    }
    if missed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} of {} limits missed",
            missed.len(),
            benchmarked.limits.len()
        ))
    }
}

fn counted(options: &Options) -> Result<(), String> {
    let counted = counts::compare(&options.binary, &repository())?;
    io::stdout()
        .lock()
        .write_all(counts::table(&counted).as_bytes())
        .map_err(|error| error.to_string())?;
    let missed: Vec<&counts::Tally> = counted.iter().filter(|each| each.missed()).collect();
    if missed.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = missed
        .iter()
        .map(|each| format!("{} with {} worker(s)", each.case.model, each.case.workers))
        .collect();
    Err(format!(
        "{} of {} counts of instructions above their limit: {}. Make the change cheaper or take \
         it back; re-record (`just record-counts`) only when the compiler changes or a change is \
         meant to cost more",
        missed.len(),
        counted.len(),
        names.join(", ")
    ))
}

fn recorded(options: &Options) -> Result<(), String> {
    let text = counts::record(&options.binary, &repository())?;
    io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .map_err(|error| error.to_string())
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let outcome = parse_options(&arguments).and_then(|options| match options.mode {
        Mode::Time => benchmark(&options).and_then(|benchmarked| printed(&benchmarked)),
        Mode::Counts => counted(&options),
        Mode::Record => recorded(&options),
    });
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr().lock(), "benchmark: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn readme() -> String {
        fs::read_to_string(repository().join("tasks/benchmark/README.md"))
            .expect("the benchmark README exists")
    }

    #[test]
    fn options_default_and_parse() {
        let defaults = parse_options(&[]).expect("no arguments is valid");
        assert_eq!(defaults.runs, DEFAULT_RUNS, "the default number of runs");
        let given = parse_options(&[
            "--runs".to_owned(),
            "3".to_owned(),
            "--binary".to_owned(),
            "x".to_owned(),
        ])
        .expect("both flags are valid");
        assert_eq!(
            (given.runs, given.binary),
            (3, PathBuf::from("x")),
            "both flags are read"
        );
        assert_eq!(defaults.mode, Mode::Time, "timing by default");
        let modes = |words: &[&str]| {
            let arguments: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            parse_options(&arguments).map(|options| options.mode)
        };
        assert_eq!(modes(&["--counts"]), Ok(Mode::Counts), "counting");
        assert_eq!(
            modes(&["--counts", "--record"]),
            Ok(Mode::Record),
            "recording"
        );
        assert!(modes(&["--record"]).is_err(), "recording needs --counts");
    }

    #[test]
    fn options_refuse_bad_flags() {
        for bad in [
            vec!["--runs"],
            vec!["--runs", "0"],
            vec!["--runs", "many"],
            vec!["--fast", "1"],
        ] {
            let arguments: Vec<String> = bad.iter().map(|word| (*word).to_owned()).collect();
            assert!(parse_options(&arguments).is_err(), "{bad:?} is refused");
        }
    }

    #[test]
    fn the_ratio_lines_divide_the_search_times_and_not_the_wall_times() {
        let measured = |wall: u64, search: u64| Measured {
            median: Duration::from_millis(wall),
            search: Duration::from_millis(search),
            peak_kib: None,
            states: 10_000,
        };
        let file = PathBuf::from("wide.model.ts");
        let limit = search_time_all_cores_over_one_worker(
            "wide",
            &file,
            (&measured(120, 30), &measured(240, 120)),
            0.75,
        );
        assert_eq!(
            limit.measured,
            Some(0.25),
            "30 ms over 120 ms, not 120 ms over 240 ms"
        );
        assert_eq!(
            limit.line(4),
            "wide, search time with all cores (0.030 s) over search time with 1 worker (0.120 s): 0.25x, budget at most 0.75x: met",
            "the line names the search times"
        );
        assert_eq!(
            limit.runs,
            vec![(file.clone(), None), (file, Some(1))],
            "a miss measures both runs again"
        );
    }

    #[test]
    fn every_benchmark_file_exists_in_both_languages_where_scaled() {
        let repository = repository();
        for entry in SCALED {
            for language in [Language::TypeScript, Language::Rust] {
                let path = models::scaled_path(&repository, entry.file, language);
                assert!(path.is_file(), "{} exists", path.display());
            }
        }
        let mixed = models::mixed_files(&repository);
        assert_eq!(mixed.len(), 12, "the stand-in set has 12 models");
        for path in mixed {
            assert!(path.is_file(), "{} exists", path.display());
        }
    }

    #[test]
    fn the_readme_derives_every_expected_count() {
        let readme = readme();
        for (name, states) in models::EXPECTED_STATES {
            let line = readme
                .lines()
                .find(|line| line.contains(&format!("`{name}`")))
                .unwrap_or_else(|| panic!("README.md has a row for {name}"));
            assert!(
                line.contains(&report::with_separators(states)),
                "the row for {name} states {states} states: {line}"
            );
        }
    }
}
