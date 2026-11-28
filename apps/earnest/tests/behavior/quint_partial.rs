use std::process::Command;

use serde_json::Value;

use crate::support::{Run, Sandbox, fields, model, rule};

const INVARIANTS: &str = r"module Spec {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  /// @check
  val small = x <= 3
  /// @check
  val picked = Set(1, 2).chooseSome() <= 2
}

/// @check
module S {
  import Spec.*
}
";

const BROKEN_BESIDE: &str = r"module Spec {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  /// @check
  val tiny = x <= 1
  /// @check
  val picked = Set(1, 2).chooseSome() <= 2
}

/// @check
module S {
  import Spec.*
}
";

const EXPECTED_BESIDE: &str = r"module Spec {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  /// @check broken
  val tiny = x <= 1
  /// @check
  val picked = Set(1, 2).chooseSome() <= 2
}

/// @check
module S {
  import Spec.*
}
";

const TEMPORAL: &str = r"/// @check
module Fair {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else x
  /// @check
  temporal weak = weakFair(step, x) implies eventually(x == 3)
  /// @check
  temporal strong = strongFair(step, x) implies eventually(x == 3)
}
";

const RUNS: &str = r"/// @check
module Played {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  action pick = x' = Set(1, 2).chooseSome()
  /// @check
  val small = x <= 3
  run stepTest = init.then(step).expect(x == 1)
  run pickTest = init.then(pick).expect(x <= 2)
}
";

const BROKEN_RUN: &str = r"/// @check
module Played {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  action pick = x' = Set(1, 2).chooseSome()
  /// @check
  val small = x <= 3
  run stepTest = init.then(step).expect(x == 2)
  run pickTest = init.then(pick).expect(x <= 2)
}
";

const INIT: &str = r"/// @check
module Start {
  var x: int
  action init = x' = Set(1, 2).chooseSome()
  action step = x' = x
  action other = x' = 5
  /// @check
  val small = x <= 3
  run otherTest = other.expect(x == 5)
  run initTest = init.then(step)
}
";

const INIT_AND_A_BROKEN_RUN: &str = r"/// @check
module Start {
  var x: int
  action init = x' = Set(1, 2).chooseSome()
  action step = x' = x
  action other = x' = 5
  /// @check
  val small = x <= 3
  run otherTest = other.expect(x == 4)
  run initTest = init.then(step)
}
";

fn checked(source: &str, arguments: &[&str]) -> (Sandbox, Run) {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/spec.qnt", source);
    let run = sandbox.earnest(&[&["verify", "specs/spec.qnt"], arguments].concat());
    (sandbox, run)
}

fn line_of(run: &Run, name: &str) -> Vec<String> {
    run.stdout
        .lines()
        .filter(|line| !line.starts_with(' '))
        .map(fields)
        .find(|line| line.get(1).is_some_and(|each| each == name))
        .unwrap_or_else(|| panic!("no result line for {name}\n{run}"))
}

fn under(run: &Run, name: &str) -> Vec<String> {
    run.stdout
        .lines()
        .skip_while(|line| line.starts_with(' ') || fields(line).get(1).is_none_or(|n| n != name))
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .map(|line| line.trim().to_owned())
        .collect()
}

fn problem_heads(run: &Run, name: &str) -> Vec<String> {
    under(run, name)
        .iter()
        .filter(|line| line.contains(".qnt:"))
        .map(|line| {
            fields(line)
                .into_iter()
                .take(2)
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

fn not_judged(report: &Value, name: &str) -> Vec<String> {
    model(report, name)["notJudged"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

#[test]
fn an_unreadable_invariant_beside_a_readable_one_leaves_the_readable_one_judged_and_no_pass() {
    let (_sandbox, run) = checked(INVARIANTS, &[]);
    run.exits(1);
    assert_eq!(line_of(&run, "S")[0], "unreadable", "{run}");
    assert!(
        under(&run, "S")
            .first()
            .is_some_and(|line| line.starts_with("note  not judged: picked ")),
        "the note names what was not judged\n{run}"
    );
    assert_eq!(
        problem_heads(&run, "S"),
        ["specs/spec.qnt:8:16  `chooseSome`"],
        "{run}"
    );
    assert_eq!(
        run.summary_line(),
        "1 model: 0 pass, 0 fail, 0 unfinished, 1 unreadable"
    );
    let (_sandbox, run) = checked(INVARIANTS, &["--json"]);
    run.exits(1);
    let report = run.json();
    let check = model(&report, "S");
    assert_eq!(check["status"], "unreadable", "{run}");
    assert_eq!(rule(check, "small")["verdict"], "holds", "{run}");
    assert!(
        check["rules"]
            .as_array()
            .is_some_and(|rules| rules.iter().all(|each| each["id"] != "picked")),
        "{run}"
    );
    assert_eq!(not_judged(&report, "S"), ["picked"], "{run}");
    assert_eq!(report["unreadable"][0]["line"], 8, "{run}");
    assert_eq!(report["unreadable"][0]["column"], 16, "{run}");
    assert_eq!(report["summary"]["unreadable"], 1, "{run}");
    assert_eq!(report["summary"]["models"], 0, "{run}");
    assert_eq!(report["summary"]["pass"], 0, "{run}");
}

#[test]
fn a_broken_readable_invariant_beside_an_unreadable_one_fails_the_check() {
    let (_sandbox, run) = checked(BROKEN_BESIDE, &[]);
    run.exits(1);
    assert_eq!(line_of(&run, "S")[0], "fail", "{run}");
    assert!(
        under(&run, "S").iter().any(|line| line == "broken  tiny"),
        "{run}"
    );
    assert_eq!(
        problem_heads(&run, "S"),
        ["specs/spec.qnt:8:16  `chooseSome`"],
        "{run}"
    );
    assert_eq!(run.summary_line(), "1 model: 0 pass, 1 fail, 0 unfinished");
}

#[test]
fn an_expected_counterexample_beside_an_unreadable_part_is_found_and_the_check_does_not_pass() {
    let (_sandbox, run) = checked(EXPECTED_BESIDE, &[]);
    run.exits(1);
    assert_eq!(line_of(&run, "S")[0], "unreadable", "{run}");
    assert!(
        under(&run, "S")
            .iter()
            .any(|line| line == "broken  tiny  (expected)"),
        "{run}"
    );
    let (_sandbox, run) = checked(EXPECTED_BESIDE, &["--json"]);
    let report = run.json();
    let tiny = rule(model(&report, "S"), "tiny");
    assert_eq!(tiny["verdict"], "broken", "{run}");
    assert_eq!(tiny["expected"], "broken", "{run}");
    assert_eq!(not_judged(&report, "S"), ["picked"], "{run}");
}

#[test]
fn an_unreadable_temporal_property_is_not_judged_without_the_fairness_it_names() {
    let (_sandbox, run) = checked(TEMPORAL, &["--json"]);
    run.exits(1);
    let report = run.json();
    let check = model(&report, "Fair");
    assert_eq!(check["status"], "unreadable", "{run}");
    assert_eq!(rule(check, "weak")["verdict"], "holds", "{run}");
    assert!(
        check["rules"]
            .as_array()
            .is_some_and(|rules| rules.iter().all(|each| each["id"] != "strong")),
        "the property whose strong fairness is not read is not judged with no fairness\n{run}"
    );
    assert_eq!(not_judged(&report, "Fair"), ["strong"], "{run}");
    assert_eq!(report["unreadable"][0]["feature"], "`strongFair`", "{run}");
}

#[test]
fn an_unreadable_run_is_listed_and_the_other_runs_and_properties_are_judged() {
    let (_sandbox, run) = checked(RUNS, &["--json"]);
    run.exits(1);
    let report = run.json();
    let check = model(&report, "Played");
    assert_eq!(check["status"], "unreadable", "{run}");
    assert_eq!(rule(check, "small")["verdict"], "holds", "{run}");
    assert_eq!(rule(check, "stepTest")["verdict"], "holds", "{run}");
    assert_eq!(not_judged(&report, "Played"), ["pickTest"], "{run}");
    let (_sandbox, run) = checked(BROKEN_RUN, &[]);
    run.exits(1);
    assert_eq!(
        line_of(&run, "Played")[0],
        "fail",
        "a broken run that was read fails the check\n{run}"
    );
}

#[test]
fn an_unreadable_init_searches_nothing_and_plays_only_the_runs_that_do_not_reach_it() {
    let (_sandbox, run) = checked(INIT, &["--json"]);
    run.exits(1);
    let report = run.json();
    let check = model(&report, "Start");
    assert_eq!(check["status"], "unreadable", "{run}");
    assert_eq!(check["states"], 0, "nothing is searched\n{run}");
    assert_eq!(rule(check, "otherTest")["verdict"], "holds", "{run}");
    assert_eq!(
        not_judged(&report, "Start"),
        ["small", "initTest"],
        "the invariant, which needs the search, and the run that starts with `init`\n{run}"
    );
    assert_eq!(
        report["unreadable"].as_array().map(Vec::len),
        Some(1),
        "the problem in `init` is listed once, though `initTest` reaches it too\n{run}"
    );
    let (_sandbox, run) = checked(INIT_AND_A_BROKEN_RUN, &[]);
    run.exits(1);
    assert_eq!(line_of(&run, "Start")[0], "fail", "{run}");
}

#[test]
fn a_second_run_served_from_the_cache_is_still_unreadable() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/spec.qnt", INVARIANTS);
    let first = sandbox.earnest(&["verify", "specs/spec.qnt"]);
    first.exits(1);
    let second = sandbox.earnest(&["verify", "specs/spec.qnt"]);
    second.exits(1);
    let line = line_of(&second, "S");
    assert_eq!(line[0], "unreadable", "{second}");
    assert_eq!(
        line.last().map(String::as_str),
        Some("(cached)"),
        "{second}"
    );
    assert_eq!(under(&first, "S"), under(&second, "S"), "{second}");
    let json = sandbox.earnest(&["verify", "specs/spec.qnt", "--json"]);
    json.exits(1);
    let report = json.json();
    assert_eq!(model(&report, "S")["cached"], true, "{json}");
    assert_eq!(model(&report, "S")["status"], "unreadable", "{json}");
    assert_eq!(not_judged(&report, "S"), ["picked"], "{json}");
}

#[test]
fn a_rule_asked_for_by_name_in_a_check_read_in_part_is_never_a_pass() {
    for asked in ["picked", "small"] {
        let (_sandbox, run) = checked(INVARIANTS, &["--rule", asked]);
        run.exits(1);
        assert_eq!(line_of(&run, "S")[0], "unreadable", "--rule {asked}\n{run}");
        assert_eq!(
            problem_heads(&run, "S"),
            ["specs/spec.qnt:8:16  `chooseSome`"],
            "--rule {asked}\n{run}"
        );
    }
}

#[test]
fn the_report_of_a_check_read_in_part_says_unreadable_and_lists_what_was_not_read() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/spec.qnt", INVARIANTS);
    let run = sandbox.earnest(&["report", "specs/spec.qnt"]);
    run.exits(1);
    assert!(
        run.stdout.contains("**unreadable** · S · 4 states"),
        "{run}"
    );
    assert!(run.stdout.contains("note  not judged: picked"), "{run}");
    assert!(
        run.stdout.contains("specs/spec.qnt:8:16  `chooseSome`"),
        "{run}"
    );
}

fn git(sandbox: &Sandbox, arguments: &[&str]) {
    let output = Command::new("git")
        .current_dir(sandbox.path())
        .args(["-c", "user.name=Conformance"])
        .args(["-c", "user.email=conformance@earnest.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(["-c", "init.defaultBranch=main"])
        .args(arguments)
        .output()
        .expect("`git` is on the `PATH`");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_rule_changes_of_a_check_read_in_part_are_listed_as_for_any_other_check() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/spec.qnt", INVARIANTS);
    git(&sandbox, &["init", "-q"]);
    git(&sandbox, &["add", "."]);
    git(&sandbox, &["commit", "-qm", "base"]);
    sandbox.write(
        "specs/spec.qnt",
        &INVARIANTS.replace(
            "  val small = x <= 3\n",
            "  val small = x <= 3\n  /// @check\n  val positive = x >= 0\n",
        ),
    );
    let run = sandbox.earnest(&["verify", "specs/spec.qnt", "--base", "HEAD"]);
    run.exits(1);
    assert_eq!(line_of(&run, "S")[0], "unreadable", "{run}");
    assert!(
        run.stdout
            .lines()
            .any(|line| fields(line) == ["added", "S.positive", "specs/spec.qnt"]),
        "{run}"
    );
    assert!(!run.stdout.contains("not compared"), "{run}");
}

#[test]
fn confirming_a_check_read_in_part_compares_nothing_and_exits_one() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/spec.qnt", INVARIANTS);
    let run = Run::of(sandbox.command().env("PATH", sandbox.path()).args([
        "confirm",
        "--quint",
        "specs/spec.qnt",
    ]));
    run.exits(1);
    assert!(
        run.stdout
            .contains("the checker can't read part of the check (not judged: picked)"),
        "{run}"
    );
    assert!(
        run.stderr.contains("unreadable  specs/spec.qnt  S"),
        "{run}"
    );
}
