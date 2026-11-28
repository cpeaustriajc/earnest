use crate::checks::is_a_name;
use crate::confirm::read_tlc;
use crate::confirm_reach::where_backends_differ;
use crate::quint::{QuintCheck, segments_end_alike};
use crate::run::only_runs_not_played;
use crate::run::{Checked, RuleResult, whole_of};
use earnest_quint::Runs;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const FOLDER: &str = ".earnest/confirm/quint";
const PROGRAM_VARIABLE: &str = "EARNEST_QUINT";
const PROGRAM: &str = "quint";
const LOCAL_PROGRAMS: &str = "node_modules/.bin";
const NOT_FOUND: &str = "`quint` was not found: install it with `npm install --save-dev \
                         @informalsystems/quint`, or set `EARNEST_QUINT`";
const BROKEN: &str = "broken";
const HOLDS: &str = "holds";
const UNKNOWN: &str = "unknown";
const WITNESS: &str = "witness";
const FAULT: &str = "fault";
const OUT_OF_RANGE: &str = "out-of-range";
const NO_VIOLATION: &str = "[ok]";
const VIOLATION: &str = "[violation]";
const PASSED: &str = " passed ";
const FAILED_AFTER: &str = " failed after ";
const TESTS: &str = " test(s)";
const PASSED_MARK: &str = "ok ";
const RUNS_THAT_END_IN_TEST: &str = "Test$";
const JVM_NOTICE: &str = "Picked up ";
const QUINT_ERROR: &str = "error: ";
const GENERAL_ERRORS: [&str; 3] = [
    "TLC error (see output above)",
    "found a counterexample",
    "Tests failed",
];
const NOT_QUINT: &str = "the model is not read from a `.qnt` file, so Quint has nothing to run";
const NO_JAVA: &str = "Java was not found on PATH, and `quint verify --backend tlc` needs it \
                       to run TLC, so the state count and the properties are not compared";
const SAMPLED: &str = "`quint test` tries random choices of a run, and the checker tries every \
                       choice";
const TLC_REFUSES_REMAINDER: &str = "The second argument of % should be a positive number";
const REMAINDER_BELOW_ONE: &str = "TLC refuses `%` by a divisor below 1 (\"The second argument \
                                  of % should be a positive number\"), where the checker takes \
                                  `a % b` as `a - b * (a / b)` for every divisor but zero";
const UNSURE_WHICH: &str = "the checker can't tell which of its results is this property";

#[cfg(windows)]
const PROGRAM_FILES: [&str; 3] = ["quint.cmd", "quint.exe", "quint"];
#[cfg(not(windows))]
const PROGRAM_FILES: [&str; 1] = [PROGRAM];

#[cfg(windows)]
const JAVA: [&str; 2] = ["java.exe", "java"];
#[cfg(not(windows))]
const JAVA: [&str; 1] = ["java"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Difference {
    pub(crate) said: String,
    pub(crate) cannot: Option<String>,
}

fn differs(said: String) -> Difference {
    Difference { said, cannot: None }
}

fn cannot(said: String, why: &str) -> Difference {
    Difference {
        said,
        cannot: Some(why.to_owned()),
    }
}

fn unless(said: String, reasons: &[String]) -> Difference {
    match reasons.first() {
        Some(why) => cannot(said, why),
        None => differs(said),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verified {
    Holds(Option<u64>),
    Violated,
    Stopped(String),
}

fn first_said(text: &str) -> Option<&str> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with(JVM_NOTICE))
}

fn names_an_error(line: &str) -> bool {
    line.split_once([':', ' '])
        .is_some_and(|(word, _)| word.ends_with("Error") && word.chars().all(char::is_alphabetic))
}

fn quint_error(stdout: &str, stderr: &str) -> Option<String> {
    let lines = || stderr.lines().chain(stdout.lines()).map(str::trim);
    let tlc = read_tlc(stdout).errors.into_iter().next();
    let thrown = lines().find(|line| names_an_error(line)).map(str::to_owned);
    let said = lines()
        .filter_map(|line| line.strip_prefix(QUINT_ERROR))
        .find(|line| !GENERAL_ERRORS.contains(line))
        .map(str::to_owned);
    tlc.or(thrown)
        .or(said)
        .or_else(|| first_said(stderr).map(str::to_owned))
}

pub(crate) fn read_verify(stdout: &str, stderr: &str, exit: Option<i32>) -> Verified {
    let has = |mark: &str| stdout.lines().any(|line| line.trim().starts_with(mark));
    if has(VIOLATION) {
        Verified::Violated
    } else if exit == Some(0) && has(NO_VIOLATION) {
        Verified::Holds(read_tlc(stdout).distinct_states)
    } else if stdout.contains(TLC_REFUSES_REMAINDER) || stderr.contains(TLC_REFUSES_REMAINDER) {
        Verified::Stopped(REMAINDER_BELOW_ONE.to_owned())
    } else {
        Verified::Stopped(
            quint_error(stdout, stderr)
                .unwrap_or_else(|| "`quint verify` stopped without saying why".to_owned()),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Tested {
    pub(crate) name: String,
    pub(crate) passed: bool,
    pub(crate) samples: u64,
}

fn tested(line: &str) -> Option<Tested> {
    let line = line.trim();
    let (name, samples, passed) = if let Some(rest) = line.strip_prefix(PASSED_MARK) {
        let (name, samples) = rest.split_once(PASSED)?;
        (name, samples, true)
    } else {
        let (number, rest) = line.split_once(") ")?;
        number.parse::<u64>().ok()?;
        let (name, samples) = rest.split_once(FAILED_AFTER)?;
        (name, samples, false)
    };
    Some(Tested {
        name: name.to_owned(),
        passed,
        samples: samples.strip_suffix(TESTS)?.parse().ok()?,
    })
}

pub(crate) fn read_test(stdout: &str) -> Vec<Tested> {
    stdout.lines().filter_map(tested).collect()
}

fn java_on_path() -> bool {
    env::var_os("PATH").is_some_and(|listed| {
        env::split_paths(&listed).any(|folder| JAVA.iter().any(|name| folder.join(name).is_file()))
    })
}

#[derive(Debug)]
struct Property {
    id: String,
    argument: String,
    temporal: bool,
    verdict: &'static str,
    expected: bool,
}

fn joined(verdicts: &[&str]) -> &'static str {
    if verdicts.contains(&BROKEN) {
        BROKEN
    } else if verdicts.iter().all(|verdict| *verdict == HOLDS) {
        HOLDS
    } else {
        UNKNOWN
    }
}

fn grouped<'a>(rules: impl Iterator<Item = &'a RuleResult>) -> Vec<(String, &'static str)> {
    let mut ids: Vec<String> = Vec::new();
    let rules: Vec<&RuleResult> = rules.collect();
    for rule in &rules {
        let id = whole_of(&rule.id);
        if !ids.iter().any(|known| known == id) {
            ids.push(id.to_owned());
        }
    }
    ids.into_iter()
        .map(|id| {
            let verdicts: Vec<&str> = rules
                .iter()
                .filter(|rule| whole_of(&rule.id) == id)
                .map(|rule| rule.verdict.as_str())
                .collect();
            let verdict = joined(&verdicts);
            (id, verdict)
        })
        .collect()
}

fn matching<'a>(
    id: &str,
    among: &'a [(String, &'static str)],
) -> Option<&'a (String, &'static str)> {
    if let Some(exact) = among.iter().find(|(known, _)| known == id) {
        return Some(exact);
    }
    if !is_a_name(id) {
        return None;
    }
    let mut alike = among
        .iter()
        .filter(|(known, _)| is_a_name(known) && segments_end_alike(known, id));
    let first = alike.next()?;
    alike.next().is_none().then_some(first)
}

fn properties(
    checked: &Checked,
    quint: &QuintCheck,
    expected: &[String],
) -> (Vec<Property>, Vec<Difference>) {
    let groups = grouped(
        checked
            .result
            .rules
            .iter()
            .filter(|rule| rule.kind.as_deref() != Some(WITNESS)),
    );
    let listed = quint
        .invariants
        .iter()
        .map(|item| (item, false))
        .chain(quint.temporal.iter().map(|item| (item, true)));
    let mut found = Vec::new();
    let mut differences = Vec::new();
    let mut matched: Vec<&str> = Vec::new();
    for (item, temporal) in listed {
        let id = item.id.clone();
        let Some((known, verdict)) = matching(&id, &groups) else {
            differences.push(cannot(
                format!("property {id}: the checker has no result for it"),
                UNSURE_WHICH,
            ));
            continue;
        };
        matched.push(known);
        found.push(Property {
            id: known.clone(),
            argument: item.text.clone(),
            temporal,
            verdict,
            expected: expected.iter().any(|name| name == known),
        });
    }
    for (id, _) in &groups {
        if !matched.contains(&id.as_str()) {
            differences.push(cannot(
                format!("property {id}: the check does not list it, and the checker judged it"),
                UNSURE_WHICH,
            ));
        }
    }
    (found, differences)
}

fn ours_cut_short(checked: &Checked) -> Vec<String> {
    checked
        .result
        .findings
        .iter()
        .filter(|finding| finding.kind == FAULT || finding.kind == OUT_OF_RANGE)
        .map(|finding| {
            format!(
                "the checker stopped on a state it could not finish: {}",
                finding.message
            )
        })
        .collect()
}

fn local_program(folder: &Path) -> Option<PathBuf> {
    let real = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    real.ancestors().find_map(|above| {
        PROGRAM_FILES
            .iter()
            .map(|name| above.join(LOCAL_PROGRAMS).join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn program_on_path() -> Option<PathBuf> {
    let listed = env::var_os("PATH")?;
    env::split_paths(&listed).find_map(|folder| {
        PROGRAM_FILES
            .iter()
            .map(|name| folder.join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn find_program(folder: &Path) -> Result<PathBuf, String> {
    if let Some(named) = env::var_os(PROGRAM_VARIABLE).filter(|named| !named.is_empty()) {
        let program = PathBuf::from(&named);
        return if program.is_file() {
            Ok(program)
        } else {
            Err(format!(
                "`{PROGRAM}` was not found: `{PROGRAM_VARIABLE}` names `{}`, which is not a \
                 file",
                program.to_string_lossy()
            ))
        };
    }
    local_program(folder)
        .or_else(program_on_path)
        .ok_or_else(|| NOT_FOUND.to_owned())
}

fn program_for(file: &Path) -> Result<PathBuf, String> {
    let program = find_program(file.parent().unwrap_or_else(|| Path::new("")))?;
    Ok(std::path::absolute(&program).unwrap_or(program))
}

#[derive(Debug)]
struct Runner<'a> {
    quint: &'a QuintCheck,
    program: PathBuf,
    folder: PathBuf,
}

impl Runner<'_> {
    fn run(&self, arguments: Vec<OsString>) -> Result<Output, String> {
        fs::create_dir_all(&self.folder).map_err(|error| {
            format!(
                "the folder {} can't be made: {error}",
                self.folder.to_string_lossy()
            )
        })?;
        Command::new(&self.program)
            .args(arguments)
            .current_dir(&self.folder)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| {
                format!(
                    "`{}` did not start: {error}",
                    self.program.to_string_lossy()
                )
            })
    }

    fn opening(&self, command: &str) -> Vec<OsString> {
        let mut arguments: Vec<OsString> = vec![command.into(), self.quint.file.clone().into()];
        arguments.extend(["--main", &self.quint.main].map(OsString::from));
        arguments
    }

    fn verify(&self, properties: &[&Property]) -> Verified {
        let check = self.quint;
        let mut arguments = self.opening("verify");
        arguments.extend(
            [
                "--backend",
                "tlc",
                "--init",
                &check.init,
                "--step",
                &check.step,
            ]
            .map(OsString::from),
        );
        let temporal: Vec<&str> = properties
            .iter()
            .filter(|property| property.temporal)
            .map(|property| property.argument.as_str())
            .collect();
        if !temporal.is_empty() {
            arguments.push("--temporal".into());
            arguments.push(temporal.join(",").into());
        }
        let invariants: Vec<&str> = properties
            .iter()
            .filter(|property| !property.temporal)
            .map(|property| property.argument.as_str())
            .collect();
        if !invariants.is_empty() {
            arguments.push("--invariants".into());
            arguments.extend(invariants.into_iter().map(OsString::from));
        }
        match self.run(arguments) {
            Ok(output) => read_verify(
                &String::from_utf8_lossy(&output.stdout),
                &String::from_utf8_lossy(&output.stderr),
                output.status.code(),
            ),
            Err(error) => Verified::Stopped(error),
        }
    }

    fn test(&self) -> Result<Vec<Tested>, String> {
        let pattern = match &self.quint.runs {
            Runs::Named(names) | Runs::TestsAnd(names) => {
                let mut last: Vec<String> = names
                    .iter()
                    .map(|name| name.rsplit("::").next().unwrap_or(name).to_owned())
                    .collect();
                if matches!(self.quint.runs, Runs::TestsAnd(_)) {
                    last.insert(0, ".*Test".to_owned());
                }
                format!("^(.*::)?({})$", last.join("|"))
            }
            _ => RUNS_THAT_END_IN_TEST.to_owned(),
        };
        let mut arguments = self.opening("test");
        arguments.extend(["--backend", "typescript", "--match", &pattern].map(OsString::from));
        let output = self.run(arguments)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let found = read_test(&stdout);
        if found.is_empty() && !output.status.success() {
            return Err(
                quint_error(&stdout, &String::from_utf8_lossy(&output.stderr))
                    .unwrap_or_else(|| format!("`quint test` stopped with {}", output.status)),
            );
        }
        Ok(found)
    }
}

fn verdict_of(verified: &Verified) -> &'static str {
    match verified {
        Verified::Holds(_) => HOLDS,
        Verified::Violated => BROKEN,
        Verified::Stopped(_) => UNKNOWN,
    }
}

fn compared_property(
    property: &Property,
    verified: &Verified,
    reasons: &[String],
) -> Option<Difference> {
    let theirs = verdict_of(verified);
    if property.verdict == theirs {
        return None;
    }
    let said = format!(
        "property {}: the checker says {}, `quint verify` says {theirs}",
        property.id, property.verdict
    );
    Some(match verified {
        Verified::Stopped(error) => cannot(said, &format!("Quint stopped: {error}")),
        _ if property.verdict == UNKNOWN => cannot(said, "the checker could not judge it"),
        _ => unless(said, reasons),
    })
}

fn searched(
    checked: &Checked,
    runner: &Runner<'_>,
    properties: &[Property],
    reasons: &[String],
) -> Vec<Difference> {
    let (expected, rest): (Vec<&Property>, Vec<&Property>) =
        properties.iter().partition(|property| property.expected);
    let together = runner.verify(&rest);
    let mut runs: Vec<(Vec<&Property>, Verified)> = Vec::new();
    if matches!(together, Verified::Violated | Verified::Stopped(_)) && rest.len() > 1 {
        for property in &rest {
            runs.push((vec![property], runner.verify(&[property])));
        }
    } else {
        runs.push((rest, together));
    }
    for property in expected {
        runs.push((vec![property], runner.verify(&[property])));
    }
    let mut counts: Vec<u64> = runs
        .iter()
        .filter_map(|(_, verified)| match verified {
            Verified::Holds(Some(states)) => Some(*states),
            _ => None,
        })
        .collect();
    let stopped_in = |verified: &Verified| match verified {
        Verified::Stopped(error) => Some(error.clone()),
        _ => None,
    };
    let mut stopped = runs.iter().find_map(|(_, verified)| stopped_in(verified));
    if counts.is_empty() && stopped.is_none() {
        let counting = runner.verify(&[]);
        if let Verified::Holds(Some(states)) = counting {
            counts.push(states);
        }
        stopped = stopped_in(&counting);
    }
    counts.sort_unstable();
    counts.dedup();
    let ours = checked.result.states;
    let mut found = Vec::new();
    if counts.is_empty() {
        let why = stopped.map_or_else(
            || "Quint printed no state count".to_owned(),
            |error| format!("Quint stopped: {error}"),
        );
        found.push(cannot(
            format!("states: the checker found {ours}, `quint verify` gave no count"),
            &why,
        ));
    }
    for states in counts {
        if u64::try_from(ours).is_ok_and(|ours| ours == states) {
            continue;
        }
        found.push(unless(
            format!("states: the checker found {ours}, `quint verify` found {states}"),
            reasons,
        ));
    }
    for (judged, verified) in &runs {
        for property in judged {
            found.extend(compared_property(property, verified, reasons));
        }
    }
    found
}

fn played(
    checked: &Checked,
    runner: &Runner<'_>,
    reasons_of: &dyn Fn(&str) -> Vec<String>,
) -> Vec<Difference> {
    let ours: Vec<&RuleResult> = checked
        .result
        .rules
        .iter()
        .filter(|rule| rule.kind.as_deref() == Some(WITNESS))
        .collect();
    let theirs = match runner.test() {
        Ok(theirs) => theirs,
        Err(error) => {
            return ours
                .iter()
                .map(|rule| {
                    cannot(
                        format!("run {}: `quint test` did not play it", rule.id),
                        &format!("Quint stopped: {error}"),
                    )
                })
                .collect();
        }
    };
    let mut claimed: Vec<&str> = Vec::new();
    let mut found = Vec::new();
    let exact: Vec<&Tested> = theirs
        .iter()
        .filter(|run| ours.iter().any(|rule| rule.id == run.name))
        .collect();
    for rule in &ours {
        let pick = exact
            .iter()
            .find(|run| run.name == rule.id)
            .copied()
            .or_else(|| {
                let mut alike = theirs.iter().filter(|run| {
                    !exact.iter().any(|taken| taken.name == run.name)
                        && run.name.ends_with(&format!("::{}", rule.id))
                });
                let first = alike.next()?;
                alike.next().is_none().then_some(first)
            });
        let Some(run) = pick else {
            found.push(differs(format!(
                "run {}: `quint test` did not play it",
                rule.id
            )));
            continue;
        };
        claimed.push(&run.name);
        let verdict = if run.passed { HOLDS } else { BROKEN };
        if verdict == rule.verdict {
            continue;
        }
        let said = format!(
            "run {}: the checker says {}, `quint test` says {verdict} after {} sample(s)",
            rule.id, rule.verdict, run.samples
        );
        found.push(if rule.verdict == UNKNOWN {
            cannot(said, "the checker could not judge it")
        } else if rule.verdict == BROKEN && run.samples > 1 {
            cannot(said, SAMPLED)
        } else {
            unless(said, &reasons_of(&rule.id))
        });
    }
    found.extend(left_over(checked, runner, &ours, &theirs, &claimed));
    found
}

fn left_over(
    checked: &Checked,
    runner: &Runner<'_>,
    ours: &[&RuleResult],
    theirs: &[Tested],
    claimed: &[&str],
) -> Vec<Difference> {
    let mut found = Vec::new();
    for run in theirs {
        if claimed.contains(&run.name.as_str()) {
            continue;
        }
        if checked.result.runs_not_played.contains(&run.name) {
            found.push(if run.passed {
                cannot(
                    format!("run {}: not compared", run.name),
                    "the checker did not play it (runs not played), and `quint test` passes it",
                )
            } else {
                differs(format!(
                    "run {}: `quint test` says broken after {} sample(s), and the checker did \
                     not play it, so it passes a check whose run Quint breaks",
                    run.name, run.samples
                ))
            });
            continue;
        }
        if !quint_check_plays_by_ending(runner.quint) {
            continue;
        }
        let said = format!(
            "run {}: `quint test` played it, and the checker did not",
            run.name
        );
        let second_name = ours
            .iter()
            .find(|rule| run.name.ends_with(&format!("::{}", rule.id)));
        found.push(match second_name {
            Some(rule) => cannot(
                said,
                &format!(
                    "Quint also names a run that the checker played as `{}`, which an \
                     `export` makes visible under two names, and the checker can't tell \
                     whether it is that run",
                    rule.id
                ),
            ),
            None => differs(said),
        });
    }
    found
}

fn quint_check_plays_by_ending(quint: &QuintCheck) -> bool {
    matches!(quint.runs, Runs::Tests | Runs::TestsAnd(_))
}

#[derive(Debug)]
pub(crate) struct Asked<'a> {
    pub(crate) checked: &'a Checked,
    pub(crate) quint: Option<&'a QuintCheck>,
    pub(crate) expected: &'a [String],
    pub(crate) beyond_tlc: Vec<String>,
    pub(crate) compiled: &'a str,
    pub(crate) folder: &'a Path,
}

pub(crate) fn quint_differences(asked: &Asked<'_>) -> Vec<Difference> {
    let checked = asked.checked;
    let Some(quint) = asked.quint else {
        return vec![cannot("not compared".to_owned(), NOT_QUINT)];
    };
    if !checked.result.not_judged.is_empty() {
        return vec![cannot(
            "not compared".to_owned(),
            &format!(
                "the checker can't read part of the check (not judged: {}), so there is no full \
                 result to compare",
                checked.result.not_judged.join(", ")
            ),
        )];
    }
    if let Some(reason) = checked
        .result
        .reason
        .as_ref()
        .filter(|reason| !only_runs_not_played(reason, &checked.result.runs_not_played))
    {
        return vec![cannot(
            "not compared".to_owned(),
            &format!("the checker stopped early ({reason}), so there is no full result to compare"),
        )];
    }
    let program = match program_for(&quint.file) {
        Ok(program) => program,
        Err(message) => return vec![cannot("not compared".to_owned(), &message)],
    };
    let runner = Runner {
        quint,
        program,
        folder: asked.folder.to_path_buf(),
    };
    let differ = where_backends_differ(asked.compiled, quint);
    let mut reasons = asked.beyond_tlc.clone();
    reasons.extend(differ.search.iter().cloned());
    reasons.extend(ours_cut_short(checked));
    let cut_short = ours_cut_short(checked);
    let run_reasons = |run: &str| -> Vec<String> {
        let mut reasons = differ.runs.clone();
        reasons.extend(differ.of_run(run));
        reasons.extend(cut_short.iter().cloned());
        reasons
    };
    let mut found = Vec::new();
    if quint.searches {
        let (properties, unmatched) = properties(checked, quint, asked.expected);
        found.extend(unmatched);
        if java_on_path() {
            found.extend(searched(checked, &runner, &properties, &reasons));
        } else {
            found.push(cannot(
                "states and properties: not compared".to_owned(),
                NO_JAVA,
            ));
        }
    }
    let plays = checked
        .result
        .rules
        .iter()
        .any(|rule| rule.kind.as_deref() == Some(WITNESS));
    if quint.runs != Runs::None && (plays || quint_check_plays_by_ending(quint)) {
        found.extend(played(checked, &runner, &run_reasons));
    }
    found
}

pub(crate) fn quint_folder() -> &'static Path {
    Path::new(FOLDER)
}

pub(crate) fn quint_lines(checked: &Checked, differences: &[Difference]) -> Vec<String> {
    let result = &checked.result;
    let word = if differences.is_empty() {
        "same"
    } else if differences.iter().any(|each| each.cannot.is_none()) {
        "differs"
    } else {
        "cannot"
    };
    let mut lines = vec![format!(
        "{word:<7}  {}  {} states  {}  (Quint)",
        result.run_name(),
        result.states,
        result.status.word()
    )];
    for each in differences {
        lines.push(format!("  {}", each.said));
        if let Some(why) = &each.cannot {
            lines.push(format!("    Quint can't confirm this: {why}"));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{Tested, Verified, names_an_error, read_test, read_verify};

    const NO_VIOLATION: &str = "\
[TLC] Running TLC model checker...
Model checking completed. No error has been found.
29 states generated, 16 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 6.

[ok] No violation found (1008ms).
";

    const A_VIOLATION: &str = "\
Error: Temporal property q_temporalProps was violated.
29 states generated, 16 distinct states found, 0 states left on queue.

[violation] Found an issue (855ms).
error: found a counterexample
";

    const PLAYED: &str = "
  Small
    ok Small::Clock::tickTwice passed 1 test(s)
    1) Small::Clock::overTick failed after 1 test(s)
    ok Small::Clock::hearAny passed 10000 test(s)

  2 passing (544ms)
  1 failed

  1) Small::Clock::overTick:
       Error [QNT513]: Reps loop could not continue after iteration #4 evaluated to false
error: Tests failed
";

    #[test]
    fn a_run_without_a_violation_gives_tlc_s_count() {
        assert_eq!(
            read_verify(NO_VIOLATION, "", Some(0)),
            Verified::Holds(Some(16))
        );
    }

    #[test]
    fn a_violation_is_read_whatever_count_tlc_printed() {
        assert_eq!(read_verify(A_VIOLATION, "", Some(1)), Verified::Violated);
    }

    #[test]
    fn a_run_that_stops_gives_quint_s_own_error() {
        assert_eq!(
            read_verify(
                "[TLC] Compiling to TLA+ (via Apalache)...\n",
                "/quint/dist/src/flattening/fullFlattener.js:90\n    throw e;\n\nTypeError: \
                 Cannot read properties of undefined\n    at flatten (x.js:1:1)\n",
                Some(1)
            ),
            Verified::Stopped("TypeError: Cannot read properties of undefined".to_owned())
        );
        assert_eq!(
            read_verify(
                "Error: TLC threw an unexpected exception.\nerror: TLC error (see output above)\n",
                "",
                Some(1)
            ),
            Verified::Stopped("TLC threw an unexpected exception.".to_owned()),
            "TLC's error before Quint's general one"
        );
        assert_eq!(
            read_verify(NO_VIOLATION, "", Some(1)),
            Verified::Stopped("`quint verify` stopped without saying why".to_owned()),
            "an exit that is not a success is not a pass"
        );
    }

    #[test]
    fn a_parse_error_of_quint_names_an_error() {
        assert!(names_an_error("Error [QNT404]: Name 'y' not found"));
        assert!(names_an_error("Error: Module A not found"));
        assert!(!names_an_error(
            "Nov 26, 2026 9:53:32 AM com.google.protobuf"
        ));
        assert!(!names_an_error("error: found a counterexample"));
    }

    #[test]
    fn each_run_quint_test_played_is_read_with_its_verdict_and_its_samples() {
        let run = |name: &str, passed: bool, samples: u64| Tested {
            name: name.to_owned(),
            passed,
            samples,
        };
        assert_eq!(
            read_test(PLAYED),
            [
                run("Small::Clock::tickTwice", true, 1),
                run("Small::Clock::overTick", false, 1),
                run("Small::Clock::hearAny", true, 10000),
            ],
            "the lines of the failure's detail are not runs"
        );
    }
}
