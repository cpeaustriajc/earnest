#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::support::{Run, Sandbox, repository};

const FIXTURES: &str = "crates/earnest_quint/fixtures";

const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  compile)
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--main" ]; then main="$argument"; fi
      previous="$argument"
    done
    cat "$here/compiled/$main.json"
    ;;
  parse)
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--out" ]; then out="$argument"; fi
      if [ "$previous" = "--source-map" ]; then map="$argument"; fi
      previous="$argument"
    done
    folder="$(cd "$(dirname "$2")" && pwd)"
    cp "$here/parse.json" "$out"
    sed "s#\"0\":\"problems.qnt\"#\"0\":\"$folder/problems.qnt\"#" "$here/sourcemap.json" > "$map"
    ;;
esac
"##;

const CHECKS: &str = r#"{ "checks": [
  { "file": "problems.qnt", "main": "problems", "invariants": ["small", "chosen"], "runs": true },
  { "file": "problems.qnt", "main": "repeated", "invariants": ["small", "listed"],
    "temporal": ["strong", "fairPick"], "runs": true }
] }"#;

fn fixture(name: &str) -> String {
    let path = repository().join(FIXTURES).join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

fn checked(arguments: &[&str]) -> Run {
    let sandbox = Sandbox::empty();
    sandbox.write("fake/quint", FAKE);
    let program = sandbox.path().join("fake/quint");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the fake can be made a program");
    sandbox.write("fake/compiled/problems.json", &fixture("problems.json"));
    sandbox.write("fake/compiled/repeated.json", &fixture("repeated.json"));
    sandbox.write("fake/parse.json", &fixture("problems.parse.json"));
    sandbox.write("fake/sourcemap.json", &fixture("problems.sourcemap.json"));
    sandbox.write("models/problems.qnt", &fixture("problems.qnt"));
    sandbox.write("models/earnest.quint.json", CHECKS);
    let empty = sandbox.path().join("no-programs");
    fs::create_dir_all(&empty).expect("an empty folder can be made");
    Run::of(
        sandbox
            .command()
            .env("EARNEST_QUINT", &program)
            .env("PATH", &empty)
            .args(arguments),
    )
}

fn places(report: &serde_json::Value) -> Vec<(u64, u64, String)> {
    report["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|problem| {
            (
                problem["line"].as_u64().unwrap_or_default(),
                problem["column"].as_u64().unwrap_or_default(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn under(run: &Run, main: &str) -> Vec<String> {
    let head = format!("unreadable  models/problems.qnt  {main}");
    run.stdout
        .lines()
        .skip_while(|line| *line != head)
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .map(|line| {
            line.split("  ")
                .filter(|part| !part.is_empty())
                .take(2)
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect()
}

#[test]
fn two_unread_runs_and_an_unread_invariant_print_three_problems_under_one_unreadable_line() {
    let run = checked(&["verify", "models/problems.qnt"]);
    run.exits(1);
    assert_eq!(
        under(&run, "problems"),
        [
            "models/problems.qnt:9:16  `chooseSome`",
            "models/problems.qnt:11:25  a run",
            "models/problems.qnt:13:22  `fail`",
        ],
        "the invariant `chosen` on line 9, the run `fromNowhereTest` on line 11 and the run \
         `lateFailTest` on line 13, in the order of the file\n{run}"
    );
    assert_eq!(
        run.stdout
            .lines()
            .filter(|line| line.starts_with("unreadable  models/problems.qnt  problems"))
            .count(),
        1,
        "{run}"
    );
    assert!(
        run.stdout.contains("in the run `fromNowhereTest`")
            && run.stdout.contains("in the run `lateFailTest`")
            && run.stdout.contains("in the definition `chosen`"),
        "each problem names the piece it is in\n{run}"
    );
}

#[test]
fn a_run_that_plays_a_refused_action_does_not_repeat_its_problem_and_the_json_carries_them_all() {
    let run = checked(&["verify", "models/problems.qnt", "--json"]);
    run.exits(1);
    let report = run.json();
    let found = places(&report);
    let expected: Vec<(u64, u64, String)> = [
        (9, 16, "`chooseSome`"),
        (11, 25, "a run"),
        (13, 22, "`fail`"),
        (21, 22, "`chooseSome`"),
        (25, 16, "`allListsUpTo`"),
        (26, 12, "`strongFair`"),
    ]
    .into_iter()
    .map(|(line, column, feature)| (line, column, feature.to_owned()))
    .collect();
    assert_eq!(
        found, expected,
        "`repeated`: the step's `pick` on line 21 once, though `pickTest` plays it and `fairPick` \
         asks for fairness of it, then the invariant `listed` and the temporal property \
         `strong`\n{run}"
    );
    assert_eq!(report["summary"]["unreadable"], 2, "{run}");
    let pick = report["unreadable"][3]["message"]
        .as_str()
        .unwrap_or_default();
    assert!(
        pick.ends_with("in the definition `pick`, which `step` uses"),
        "the problem keeps the trail from the step it was first reached from: {pick}"
    );
}
