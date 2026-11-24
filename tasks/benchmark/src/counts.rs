//! Counts the instructions that the checker's own binary executes on a few fixed cases, under
//! callgrind, and compares each count with the one recorded in `tasks/benchmark/instructions.txt`.
//!
//! A count repeats where a clock does not: the clock of a shared machine moves 5% to 10% between
//! runs and as much again when the compiler merely places a loop elsewhere, while the count of one
//! case moves by less than 0.2%. Only the instructions inside the checker's binary are counted,
//! not those of the C library or the loader, whose code differs between machines.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::models::FOLDER;
use crate::run::{check_counts, parse_results};

/// Where the recorded counts live, from the root of the repository.
pub(crate) const RECORD: &str = "tasks/benchmark/instructions.txt";

const BASIS_POINTS: u64 = 10_000;
const CALLGRIND_OUT: &str = "callgrind.out";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Case {
    pub(crate) model: &'static str,
    pub(crate) workers: usize,
    pub(crate) most_above: u64,
}

pub(crate) const CASES: [Case; 7] = [
    Case {
        model: "grid_1m",
        workers: 1,
        most_above: 100,
    },
    Case {
        model: "ledger",
        workers: 1,
        most_above: 100,
    },
    Case {
        model: "wide",
        workers: 1,
        most_above: 100,
    },
    Case {
        model: "narrow_levels",
        workers: 1,
        most_above: 100,
    },
    Case {
        model: "mixed/shelf",
        workers: 1,
        most_above: 200,
    },
    Case {
        model: "wide",
        workers: 4,
        most_above: 100,
    },
    Case {
        model: "grid_1m",
        workers: 4,
        most_above: 100,
    },
];

impl Case {
    fn key(&self) -> String {
        format!("{} {}", self.model, self.workers)
    }

    fn file_name(&self) -> String {
        let stem = self.model.rsplit('/').next().unwrap_or(self.model);
        format!("{stem}.model.ts")
    }

    fn above_limit(&self, recorded: u64, counted: u64) -> bool {
        u128::from(counted) * u128::from(BASIS_POINTS)
            > u128::from(recorded) * u128::from(BASIS_POINTS + self.most_above)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Tally {
    pub(crate) case: Case,
    pub(crate) recorded: Option<u64>,
    pub(crate) counted: u64,
    pub(crate) again: Option<u64>,
}

impl Tally {
    fn best(&self) -> u64 {
        self.again
            .map_or(self.counted, |again| again.min(self.counted))
    }

    pub(crate) fn missed(&self) -> bool {
        self.recorded
            .is_none_or(|recorded| self.case.above_limit(recorded, self.best()))
    }
}

/// Reads the recorded counts: one line for each case, its model, its number of workers and its
/// count, apart by spaces.
pub(crate) fn parse_record(text: &str) -> Result<Vec<(String, u64)>, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                [model, workers, count] => count
                    .parse()
                    .map(|count| (format!("{model} {workers}"), count))
                    .map_err(|_| format!("{RECORD}: `{line}` has no whole count")),
                _ => Err(format!(
                    "{RECORD}: `{line}` is not a model, a number of workers and a count"
                )),
            }
        })
        .collect()
}

pub(crate) fn record_text(counted: &[(Case, u64)]) -> String {
    counted
        .iter()
        .fold(String::new(), |mut text, (case, count)| {
            let _ = writeln!(text, "{} {count}", case.key());
            text
        })
}

fn recorded_for(record: &[(String, u64)], case: &Case) -> Option<u64> {
    let key = case.key();
    record
        .iter()
        .find(|(recorded, _)| *recorded == key)
        .map(|(_, count)| *count)
}

/// Sums the instructions that callgrind's output gives to `object`, the path of one file.
///
/// The output names each file once in full, as `ob=(n) path`, and by its number after that. A cost
/// line after a `calls=` line is the cost of the call, which the callee's own lines count already,
/// so it is skipped.
pub(crate) fn instructions_in(out: &str, object: &str) -> Result<u64, String> {
    let mut names: Vec<(String, String)> = Vec::new();
    let mut inside = false;
    let mut call_cost_next = false;
    let mut total: u64 = 0;
    let mut seen = false;
    for line in out.lines() {
        if let Some(named) = line.strip_prefix("ob=") {
            let id = remember(&mut names, named);
            inside = names
                .iter()
                .any(|(number, path)| *number == id && path == object);
            seen |= inside;
        } else if let Some(named) = line.strip_prefix("cob=") {
            remember(&mut names, named);
        } else if line.starts_with("calls=") {
            call_cost_next = true;
        } else if line.starts_with(|first: char| first.is_ascii_digit() || "+-*".contains(first)) {
            if std::mem::take(&mut call_cost_next) || !inside {
                continue;
            }
            let cost = line
                .split_whitespace()
                .nth(1)
                .ok_or_else(|| format!("a cost line without a cost: `{line}`"))?;
            let cost: u64 = cost
                .parse()
                .map_err(|_| format!("a cost that is not a whole number: `{line}`"))?;
            total = total.saturating_add(cost);
        }
    }
    if seen {
        Ok(total)
    } else {
        Err(format!("callgrind's output names no file {object}"))
    }
}

fn remember(names: &mut Vec<(String, String)>, entry: &str) -> String {
    let Some((id, path)) = entry
        .strip_prefix('(')
        .and_then(|rest| rest.split_once(')'))
    else {
        return entry.to_owned();
    };
    let path = path.trim();
    if !path.is_empty() && !names.iter().any(|(number, _)| number == id) {
        names.push((id.to_owned(), path.to_owned()));
    }
    id.to_owned()
}

fn count_once(binary: &Path, repository: &Path, case: &Case) -> Result<u64, String> {
    let scratch = tempfile::tempdir().map_err(|error| error.to_string())?;
    let source = repository
        .join(FOLDER)
        .join(format!("{}.model.ts", case.model));
    let file = case.file_name();
    fs::copy(&source, scratch.path().join(&file))
        .map_err(|error| format!("{} cannot be copied: {error}", source.display()))?;
    let out = scratch.path().join(CALLGRIND_OUT);
    let output = Command::new("valgrind")
        .arg("--tool=callgrind")
        .arg(format!("--callgrind-out-file={}", out.display()))
        .arg(binary)
        .args(["verify", "--no-cache", "--json", "--jobs"])
        .arg(case.workers.to_string())
        .arg(&file)
        .current_dir(scratch.path())
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("valgrind did not start ({error}); install valgrind"))?;
    if !output.status.success() {
        return Err(format!(
            "{} under valgrind exited with {}: {}",
            case.key(),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let models = parse_results(&String::from_utf8_lossy(&output.stdout))?;
    check_counts(&models)?;
    let text = fs::read_to_string(&out).map_err(|error| error.to_string())?;
    instructions_in(&text, &binary.to_string_lossy())
}

fn canonical(binary: &Path) -> Result<PathBuf, String> {
    binary.canonicalize().map_err(|error| {
        format!(
            "{} cannot be found ({error}); build it with `cargo build --release -p earnest-cli`",
            binary.display()
        )
    })
}

fn progress(case: &Case) -> Result<(), String> {
    crate::write_progress(&format!(
        "counting the instructions of {} with {} worker(s)",
        case.model, case.workers
    ))
    .map_err(|error| error.to_string())
}

/// Counts every case and compares it with its record; a case above its limit is counted once more
/// and judged by the smaller count.
pub(crate) fn compare(binary: &Path, repository: &Path) -> Result<Vec<Tally>, String> {
    let binary = canonical(binary)?;
    let text = fs::read_to_string(repository.join(RECORD))
        .map_err(|error| format!("{RECORD} cannot be read: {error}"))?;
    let record = parse_record(&text)?;
    let mut counted = Vec::new();
    for case in CASES {
        progress(&case)?;
        let mut each = Tally {
            case,
            recorded: recorded_for(&record, &case),
            counted: count_once(&binary, repository, &case)?,
            again: None,
        };
        if each.missed() && each.recorded.is_some() {
            progress(&case)?;
            each.again = Some(count_once(&binary, repository, &case)?);
        }
        counted.push(each);
    }
    Ok(counted)
}

/// Counts every case and writes the counts to the record.
pub(crate) fn record(binary: &Path, repository: &Path) -> Result<String, String> {
    let binary = canonical(binary)?;
    let mut counted = Vec::new();
    for case in CASES {
        progress(&case)?;
        counted.push((case, count_once(&binary, repository, &case)?));
    }
    let text = record_text(&counted);
    fs::write(repository.join(RECORD), &text)
        .map_err(|error| format!("{RECORD} cannot be written: {error}"))?;
    Ok(text)
}

fn with_separators(number: u64) -> String {
    crate::report::with_separators(number)
}

fn change(recorded: u64, counted: u64) -> String {
    let difference = u128::from(counted.abs_diff(recorded)) * u128::from(BASIS_POINTS);
    let basis_points = difference / u128::from(recorded.max(1));
    let sign = match (basis_points, counted < recorded) {
        (0, _) => "",
        (_, true) => "-",
        (_, false) => "+",
    };
    format!("{sign}{}.{:02}%", basis_points / 100, basis_points % 100)
}

fn percent(basis_points: u64) -> String {
    format!("{}.{:02}%", basis_points / 100, basis_points % 100)
}

/// The Markdown table of the counts, one row for each case.
pub(crate) fn table(counted: &[Tally]) -> String {
    let mut text = String::from(
        "## Instructions in the checker's binary, under callgrind\n\n\
         | Model | Workers | Recorded | Counted | Change | Most above the record | Verdict |\n\
         |---|---:|---:|---:|---:|---:|---|\n",
    );
    for each in counted {
        let counted_text = each.again.map_or_else(
            || with_separators(each.counted),
            |again| {
                format!(
                    "{}, again {}",
                    with_separators(each.counted),
                    with_separators(again)
                )
            },
        );
        let (recorded, change_text) = each.recorded.map_or_else(
            || ("none".to_owned(), "n/a".to_owned()),
            |recorded| (with_separators(recorded), change(recorded, each.best())),
        );
        let verdict = if each.missed() { "missed" } else { "met" };
        let _ = writeln!(
            text,
            "| {} | {} | {recorded} | {counted_text} | {change_text} | {} | {verdict} |",
            each.case.model,
            each.case.workers,
            percent(each.case.most_above)
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUT: &str = "\
version: 1
events: Ir
ob=(1) /bin/earnest
fl=(1) a.rs
fn=(1) main
0 100
+2 20
cob=(2) /lib/libc.so.6
cfn=(2) memcpy
calls=1 0
0 999
ob=(2)
fl=(2) memcpy.S
fn=(2)
0 7
ob=(1)
fn=(1)
* 3
summary: 130
";

    #[test]
    fn the_count_of_one_file_skips_the_cost_of_its_calls_and_the_other_files() {
        assert_eq!(
            instructions_in(OUT, "/bin/earnest"),
            Ok(123),
            "100, 20 and 3, without the call of 999"
        );
        assert_eq!(
            instructions_in(OUT, "/lib/libc.so.6"),
            Ok(7),
            "a file first named in a call"
        );
        assert!(
            instructions_in(OUT, "/bin/other").is_err(),
            "a file that is not in the output"
        );
    }

    #[test]
    fn a_cost_that_is_not_a_number_is_an_error() {
        assert!(
            instructions_in("ob=(1) /bin/earnest\n0 many\n", "/bin/earnest").is_err(),
            "a cost of words"
        );
    }

    #[test]
    fn a_count_above_its_share_misses_and_one_at_it_meets() {
        let case = Case {
            model: "wide",
            workers: 1,
            most_above: 100,
        };
        let counted = |counted, again| Tally {
            case,
            recorded: Some(10_000),
            counted,
            again,
        };
        assert!(!counted(10_100, None).missed(), "1% above meets");
        assert!(counted(10_101, None).missed(), "more than 1% above misses");
        assert!(!counted(9_000, None).missed(), "fewer meets");
        assert!(
            !counted(10_500, Some(10_050)).missed(),
            "the smaller of two counts is judged"
        );
        let unrecorded = Tally {
            recorded: None,
            ..counted(1, None)
        };
        assert!(unrecorded.missed(), "a case with no record misses");
    }

    #[test]
    fn the_record_reads_back_what_it_writes() {
        let written = record_text(&[(CASES[0], 12), (CASES[6], 34)]);
        assert_eq!(
            written, "grid_1m 1 12\ngrid_1m 4 34\n",
            "one line for each case"
        );
        assert_eq!(
            parse_record(&written),
            Ok(vec![
                ("grid_1m 1".to_owned(), 12),
                ("grid_1m 4".to_owned(), 34)
            ]),
            "the same counts"
        );
        assert!(
            parse_record("grid_1m 1\n").is_err(),
            "a line without a count"
        );
        assert!(
            parse_record("grid_1m 1 many\n").is_err(),
            "a count of words"
        );
    }

    #[test]
    fn the_change_is_given_in_hundredths_of_a_percent() {
        assert_eq!(change(10_000, 10_123), "+1.23%", "a rise");
        assert_eq!(change(10_000, 9_950), "-0.50%", "a fall");
        assert_eq!(change(100_000, 99_999), "0.00%", "too small to show");
        assert_eq!(percent(200), "2.00%", "a limit");
    }

    #[test]
    fn every_case_has_a_model_file_and_a_recorded_count() {
        let repository = crate::repository();
        let text = fs::read_to_string(repository.join(RECORD)).expect("the record exists");
        let record = parse_record(&text).expect("the record reads");
        assert_eq!(record.len(), CASES.len(), "one line for each case");
        for case in CASES {
            let file = repository
                .join(FOLDER)
                .join(format!("{}.model.ts", case.model));
            assert!(file.is_file(), "{} exists", file.display());
            assert!(
                recorded_for(&record, &case).is_some(),
                "{} has a count",
                case.key()
            );
        }
    }
}
