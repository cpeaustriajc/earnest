#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{binary, rule};

const RECORDED: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
main=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "--main" ]; then main="$argument"; fi
  if [ "$previous" = "--out" ]; then out="$argument"; fi
  if [ "$previous" = "--source-map" ]; then map="$argument"; fi
  previous="$argument"
done
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  parse)
    folder="$(cd "$(dirname "$2")" && pwd)"
    cp "$here/parse.json" "$out"
    sed "s#\"0\":\"#\"0\":\"$folder/#" "$here/sourcemap.json" > "$map"
    ;;
  typecheck)
    if [ ! -f "$here/typecheck.json" ]; then exit 1; fi
    cp "$here/typecheck.json" "$out"
    ;;
  compile)
    if [ ! -f "$here/compile.$main.json" ]; then echo "no compile for $main" >&2; exit 1; fi
    cat "$here/compile.$main.json"
    ;;
esac
"##;

fn case_folder(case: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("models/quint_sixth")
        .join(case)
}

fn is_recorded_output(name: &str) -> bool {
    name == "parse.json"
        || name == "sourcemap.json"
        || name == "typecheck.json"
        || (name.starts_with("compile.")
            && Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "json"))
}

fn ran(case: &str, arguments: &[&str]) -> (String, String) {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    let program = fake.join("quint");
    fs::write(&program, RECORDED).expect("the fake program can be written");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
    let source = case_folder(case);
    let entries = fs::read_dir(&source)
        .unwrap_or_else(|error| panic!("{} can't be listed: {error}", source.display()));
    for entry in entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let target = if is_recorded_output(&name) {
            fake.join(&name)
        } else {
            folder.path().join(&name)
        };
        fs::copy(entry.path(), &target)
            .unwrap_or_else(|error| panic!("{name} can't be copied: {error}"));
    }
    let output = Command::new(binary())
        .args(arguments)
        .env("EARNEST_QUINT", &program)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn verified(case: &str, arguments: &[&str]) -> Value {
    let mut all = vec!["verify", "--json", "--no-cache", "."];
    all.extend_from_slice(arguments);
    let (stdout, stderr) = ran(case, &all);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("earnest verify prints JSON ({error}):\n{stdout}\n{stderr}"))
}

fn model<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == name))
        .unwrap_or(&Value::Null)
}

#[test]
fn a_check_whose_only_mark_is_reachable_stops_once_the_state_is_reached() {
    let report = verified("reachable_only", &[]);
    let checked = model(&report, "reachableOnly");
    assert_eq!(
        (
            &rule(checked, "two")["expected"],
            &rule(checked, "two")["verdict"]
        ),
        (&Value::from("reached"), &Value::from("broken")),
        "the mark is read and the state where `x` is 2 is reached\n{report:#}"
    );
    assert_eq!(
        (&checked["status"], &checked["states"], &checked["reason"]),
        (
            &Value::from("pass"),
            &Value::from(4),
            &Value::from("every expected counterexample was found")
        ),
        "`x` counts from 0 to 10, so the model has 11 states. A state marked `reachable` is \
         looked for as a counterexample of its negation, so a check that judges nothing else \
         stops at the end of the level that reaches it, as the docs say under \"Stopping once \
         every counterexample is found\", and a model with no end still passes\n{report:#}"
    );
}

fn leftover_message(report: &Value) -> String {
    report["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|problem| problem["file"] == "earnest.quint.json")
        .and_then(|problem| problem["message"].as_str())
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn the_leftover_file_message_writes_an_expression_of_a_copied_setup_in_the_copy_s_names() {
    let report = verified("leftover", &[]);
    let message = leftover_message(&report);
    assert!(
        message.contains("import Spec(N = 2) as V")
            && message.contains("/// @check init = V::start,"),
        "the entry `SmallR` is turned into a copy of `Small` that imports `Spec` as `V`\n{message}"
    );
    assert!(
        !message.contains("val atMostTwo = x <= 2"),
        "in the copy `Spec` is imported as `V`, so `x` is `V::x` there: written as the message \
         says, `val atMostTwo = x <= 2` makes `quint parse` stop with `[QNT404] Name 'x' not \
         found`, where the checks file gave a check of 2 states that passes on 6dceb43. The docs: \
         \"the message lists the lines to add to a copy of the module\"\n{message}"
    );
}

#[test]
fn the_leftover_file_message_does_not_add_a_second_step_to_a_copy_that_has_one() {
    let report = verified("leftover", &[]);
    let message = leftover_message(&report);
    let copy = message
        .split_once("the check of `Big` named `BigR`")
        .map_or("", |(_, rest)| rest);
    assert!(
        !copy.contains("add these lines to the copy:\n      action step = stepR"),
        "`Big` declares `action step = S::step`, so a copy of it already has a `step`: adding \
         `action step = stepR` beside it, as the message says, makes `quint parse` stop with \
         `[QNT101] Conflicting definitions found for name 'step' in module 'BigR'`, where the \
         checks file gave a check of 4 states that passes on 6dceb43. The copy's `step` must be \
         replaced, not added to\n{message}"
    );
}

#[test]
fn the_report_of_a_check_failed_by_its_marks_says_why_and_does_not_call_a_met_mark_broken() {
    let (verify, _) = ran("report_marks", &["verify", "--no-cache", "."]);
    assert!(
        verify.contains("holds  belowNine  a counterexample was expected")
            && verify.contains("never reached  atFive"),
        "`earnest verify` says why the check fails\n{verify}"
    );
    let (report, stderr) = ran("report_marks", &["report", "."]);
    assert!(
        report.contains("**fail**"),
        "the report gives the check's status\n{report}\n{stderr}"
    );
    assert!(
        report.contains("belowNine") && report.contains("atFive"),
        "the check fails only because `belowNine` (marked `@check broken`) holds, as `quint verify \
         --backend tlc --invariant belowNine` says (`[ok]`), and `atFive` (marked `@check \
         reachable`) is never reached; the report must name them, as `earnest verify` does, or a \
         pull request body says `fail` with no reason\n{report}"
    );
    assert!(
        !report.contains("broken  atOne\n") && !report.contains("broken  belowTwo  \""),
        "`atOne` was reached and `belowTwo` broke as marked: both are what the marks ask for, which \
         `earnest verify` prints as `reached  atOne` and `broken  belowTwo  (expected)`. Printed \
         as a plain `broken` under a `fail`, they read as the rules that failed the check\n{report}"
    );
}

#[test]
fn a_run_marked_broken_that_a_compiled_check_can_t_play_leaves_it_unfinished_with_its_properties_judged()
 {
    let report = verified("broken_run_one_name", &[]);
    let checked = model(&report, "One");
    assert_eq!(
        (&checked["status"], &rule(checked, "small")["verdict"]),
        (&Value::from("unfinished"), &Value::from("holds")),
        "`One` imports `Spec.badRun`, `Spec.fineTest`, `init`, `step` and `small` one name at a \
         time, so it is compiled and holds none of `Spec`'s runs. `quint verify --backend tlc \
         --main One --invariant small` says `[ok]`; with `badRun` marked `@check` the check is \
         `unfinished` with `runs not played: fineTest, badRun`. The docs: \"A marked or named run \
         the compiled module does not hold is one of those runs, and the check's properties are \
         still judged\". Marked `@check broken`, the run is read as a property `One` lacks (`the \
         module `One` has no property `badRun` to judge`, at line 1) and nothing is judged\n\
         {report:#}"
    );
    let not_played: Vec<&str> = checked["runsNotPlayed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        not_played.contains(&"badRun") && not_played.contains(&"fineTest"),
        "both runs are named as not played, as with a plain `@check` on `badRun`\n{report:#}"
    );
}

#[test]
fn a_run_that_is_false_or_fails_an_assert_is_broken_in_a_check_of_runs_only() {
    let report = verified("false_runs", &[]);
    let checked = model(&report, "falseRuns");
    let verdicts: Vec<(&str, &str)> = [
        "plainFalseTest",
        "plainTrueTest",
        "bareAssertTest",
        "firstAssertTest",
        "pickAssertTest",
        "passingAssertTest",
    ]
    .into_iter()
    .map(|run| (run, rule(checked, run)["verdict"].as_str().unwrap_or("")))
    .collect();
    assert_eq!(
        verdicts,
        [
            ("plainFalseTest", "broken"),
            ("plainTrueTest", "holds"),
            ("bareAssertTest", "broken"),
            ("firstAssertTest", "broken"),
            ("pickAssertTest", "broken"),
            ("passingAssertTest", "holds"),
        ],
        "`quint test --backend typescript --main falseRuns` fails `plainFalseTest`, \
         `bareAssertTest`, `firstAssertTest` and `pickAssertTest` (QNT508, Assertion failed, for \
         the three with an `assert`) and passes the other two. The module marks no property, so \
         the check plays its runs and searches nothing: `plainFalseTest` passed there, and the \
         three runs whose first action holds a false `assert` made the check unreadable\n\
         {report:#}"
    );
    assert_eq!(checked["status"], "fail", "{report:#}");
}

fn nondet_verdicts(runs: &[&'static str]) -> (Vec<(&'static str, String)>, Value) {
    let report = verified("nondet_runs", &[]);
    let checked = model(&report, "nondetRuns");
    let verdicts = runs
        .iter()
        .map(|run| {
            let verdict = rule(checked, run)["verdict"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            (*run, verdict)
        })
        .collect();
    (verdicts, checked.clone())
}

fn expected(pairs: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
    pairs
        .iter()
        .map(|(run, verdict)| (*run, (*verdict).to_owned()))
        .collect()
}

#[test]
fn a_run_with_one_nondet_at_its_top_skips_a_value_it_is_false_for() {
    let runs = [
        "expectOneOfThreeTest",
        "thenGuardTest",
        "firstGuardThenTest",
        "firstGuardOnlyTest",
        "allFalseTest",
    ];
    let (verdicts, checked) = nondet_verdicts(&runs);
    assert_eq!(
        verdicts,
        expected(&[
            ("expectOneOfThreeTest", "broken"),
            ("thenGuardTest", "holds"),
            ("firstGuardThenTest", "broken"),
            ("firstGuardOnlyTest", "holds"),
            ("allFalseTest", "broken"),
        ]),
        "`quint test --backend typescript --main nondetRuns` (Quint 0.33.1) prints `ok \
         thenGuardTest passed 10000 test(s)` and `ok firstGuardOnlyTest passed 10000 test(s)`: \
         where the last step, or the only action, can't happen for `n` = 3, the run is false and \
         Quint tries the next value. It prints `expectOneOfThreeTest failed after 1 test(s)` (a \
         false `expect` is an error), `firstGuardThenTest failed after 2 test(s)` (the first \
         part of a `then` is false, QNT513) and `allFalseTest failed after 1 test(s)` (false for \
         every value). Issue #86\n{checked:#}"
    );
    let why = rule(&checked, "firstGuardThenTest")["reason"]
        .as_str()
        .unwrap_or("");
    assert!(
        why.contains("with `n` = 3"),
        "the broken run names the value that ends in an error\n{checked:#}"
    );
}

#[test]
fn a_step_that_can_t_happen_in_the_middle_of_a_chain_is_an_error_and_at_its_end_is_false() {
    let (verdicts, checked) = nondet_verdicts(&["threeStepTest", "expectLastTest"]);
    assert_eq!(
        verdicts,
        expected(&[("threeStepTest", "holds"), ("expectLastTest", "broken")]),
        "`quint test` prints `ok threeStepTest passed 10000 test(s)`: the guard `n == 2` is on \
         the last step of three. It prints `expectLastTest failed after 2 test(s)`: the guarded \
         step is the action of an `expect`, and Quint stops with QNT508 (`Cannot continue to \
         \"expect\"`) when it is false\n{checked:#}"
    );
}

#[test]
fn a_run_with_two_nondets_one_inside_the_other_skips_a_combination_it_is_false_for() {
    let runs = [
        "twoNestedTest",
        "twoNestedAllFalseTest",
        "twoNestedErrTest",
        "nestedEmptyTest",
    ];
    let (verdicts, checked) = nondet_verdicts(&runs);
    assert_eq!(
        verdicts,
        expected(&[
            ("twoNestedTest", "holds"),
            ("twoNestedAllFalseTest", "broken"),
            ("twoNestedErrTest", "broken"),
            ("nestedEmptyTest", "holds"),
        ]),
        "`quint test` prints `ok twoNestedTest passed 10000 test(s)` and `ok nestedEmptyTest \
         passed 10000 test(s)` (the inner `nondet` over an empty set is false, so the outer one \
         tries `n` = 2), and `twoNestedAllFalseTest failed after 1 test(s)` and \
         `twoNestedErrTest failed after 1 test(s)`\n{checked:#}"
    );
}

#[test]
fn a_nondet_followed_by_a_val_skips_a_value_it_is_false_for() {
    let (verdicts, checked) = nondet_verdicts(&["valTest"]);
    assert_eq!(
        verdicts,
        expected(&[("valTest", "holds")]),
        "`quint test` prints `ok valTest passed 10000 test(s)`\n{checked:#}"
    );
}

#[test]
fn a_step_of_reps_that_can_t_happen_is_an_error_after_a_nondet() {
    let (verdicts, checked) =
        nondet_verdicts(&["repsLastTest", "repsThenTest", "repsBodyThenTest"]);
    assert_eq!(
        verdicts,
        expected(&[
            ("repsLastTest", "broken"),
            ("repsThenTest", "holds"),
            ("repsBodyThenTest", "broken"),
        ]),
        "`quint test` prints `repsLastTest failed after 2 test(s)` and `repsBodyThenTest failed \
         after 2 test(s)`: a false round of `reps` stops it with QNT513 (`Reps loop could not \
         continue`), even the last round. It prints `ok repsThenTest passed 10000 test(s)`, \
         where the false step comes after the `reps`\n{checked:#}"
    );
}

#[test]
fn a_run_ending_in_fail_after_a_nondet_is_false_where_its_action_gets_through() {
    let runs = [
        "failTest",
        "failAllTrueTest",
        "failExpectTest",
        "failOnlyTest",
        "failOnlyAllTest",
    ];
    let (verdicts, checked) = nondet_verdicts(&runs);
    assert_eq!(
        verdicts,
        expected(&[
            ("failTest", "holds"),
            ("failAllTrueTest", "broken"),
            ("failExpectTest", "broken"),
            ("failOnlyTest", "holds"),
            ("failOnlyAllTest", "broken"),
        ]),
        "`A.fail()` is `not(A)`: where `A` gets through the run is false and Quint tries the \
         next value. `quint test` prints `ok failTest passed 10000 test(s)` and `ok failOnlyTest \
         passed 10000 test(s)` (`n` = 3 can't take the step), and `failAllTrueTest failed after \
         1 test(s)`, `failExpectTest failed after 1 test(s)` and `failOnlyAllTest failed after 1 \
         test(s)`, where every value gets through\n{checked:#}"
    );
}

#[test]
fn a_helper_that_chains_steps_after_a_nondet_is_false_only_at_the_run_s_last_step() {
    let (verdicts, checked) = nondet_verdicts(&["helperTest", "helperMidTest"]);
    assert_eq!(
        verdicts,
        expected(&[("helperTest", "holds"), ("helperMidTest", "broken")]),
        "`chain2(k)` is `step.then(guarded(k))`. `quint test` prints `ok helperTest passed 10000 \
         test(s)`, where `guarded(3)` is the run's last step, and `helperMidTest failed after 1 \
         test(s)`, where a `then(step)` follows it\n{checked:#}"
    );
}

#[test]
fn a_run_that_calls_a_run_with_a_nondet_skips_a_value_as_that_run_does() {
    let (verdicts, checked) = nondet_verdicts(&["calledTest"]);
    assert_eq!(
        verdicts,
        expected(&[("calledTest", "holds")]),
        "`run calledTest = thenGuardTest`, and `quint test` prints `ok calledTest passed 10000 \
         test(s)`\n{checked:#}"
    );
}

#[test]
fn a_nondet_inside_a_step_with_one_at_the_top_holds_only_when_some_value_always_gets_through() {
    let runs = [
        "innerTopTest",
        "innerTopFalseTest",
        "startMixTest",
        "startMixOneTest",
    ];
    let (verdicts, checked) = nondet_verdicts(&runs);
    assert_eq!(
        verdicts,
        expected(&[
            ("innerTopTest", "broken"),
            ("innerTopFalseTest", "holds"),
            ("startMixTest", "broken"),
            ("startMixOneTest", "holds"),
        ]),
        "`quint test` prints `innerTopTest failed after 9 test(s)` and `startMixTest failed \
         after 2 test(s)`: for each `n` one value picked inside a step gets the run through and \
         the others leave it false, so a sample can be false for every `n`. It prints `ok \
         innerTopFalseTest passed 10000 test(s)` and `ok startMixOneTest passed 10000 test(s)`, \
         where `n` = 1 and `n` = 2 get through whatever is picked inside\n{checked:#}"
    );
}

#[test]
fn a_false_assert_an_empty_set_and_a_false_value_after_a_nondet_are_judged_as_quint_does() {
    let runs = [
        "assertFirstTest",
        "emptyTest",
        "pureTest",
        "pureFalseTest",
        "boolTest",
    ];
    let (verdicts, checked) = nondet_verdicts(&runs);
    assert_eq!(
        verdicts,
        expected(&[
            ("assertFirstTest", "broken"),
            ("emptyTest", "broken"),
            ("pureTest", "holds"),
            ("pureFalseTest", "broken"),
            ("boolTest", "holds"),
        ]),
        "`quint test` prints `assertFirstTest failed after 9 test(s)` (QNT508, Assertion failed, \
         an error and not a false run), `emptyTest failed after 1 test(s)`, `ok pureTest passed \
         10000 test(s)`, `pureFalseTest failed after 1 test(s)` and `ok boolTest passed 10000 \
         test(s)`\n{checked:#}"
    );
}

#[test]
fn a_nondet_over_a_set_of_100_values_or_more_tries_no_other_value() {
    let (verdicts, checked) = nondet_verdicts(&["bigTest", "bigNestTest"]);
    assert_eq!(
        verdicts,
        expected(&[("bigTest", "broken"), ("bigNestTest", "holds")]),
        "Quint retries a `nondet` only over a set of fewer than 100 values \
         (`QUINT_RETRY_NONDET_SMALLER_THAN`). `quint test` prints `bigTest failed after 14 \
         test(s)`, where `1.to(150)` gives a false run for `n` of 140 or more, and `ok \
         bigNestTest passed 10000 test(s)`, where the inner `nondet k` over two values is \
         retried for every `n`\n{checked:#}"
    );
}
