#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::support::{Marks, Run, Sandbox, finding, model, repository, rule, with_marks};

const FIXTURES: &str = "crates/earnest_quint/fixtures";

const FAKE: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
case "$1" in
  --version) echo "0.33.1" ;;
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
    cp "$here/parse.json" "$out"
    echo '{"sourceIndex":{},"map":{}}' > "$map"
    ;;
esac
"#;

const MARKS: Marks<'static> = &[(
    "reads",
    &[
        ("natural", "val", "@check"),
        ("booleans", "val", "@check"),
        ("names", "val", "@check"),
        ("only", "val", "@check"),
        ("operators", "val", "@check"),
        ("folded", "val", "@check"),
        ("small", "val", "@check"),
    ],
)];

const HOLDING: [&str; 6] = [
    "natural",
    "booleans",
    "names",
    "only",
    "operators",
    "folded",
];

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
    sandbox.write("fake/compiled/reads.json", &fixture("reads.json"));
    sandbox.write(
        "fake/parse.json",
        &with_marks(r#"{"modules":[]}"#, MARKS),
    );
    sandbox.write(
        "specs/reads.qnt",
        &format!("{}// marked: reads\n", fixture("reads.qnt")),
    );
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

#[test]
fn the_sample_of_every_construct_read_since_issue_64_prints_one_line_with_tlcs_eight_states() {
    let run = checked(&["verify", "specs/reads.qnt"]);
    run.exits(1);
    let lines = run.model_lines();
    assert_eq!(lines.len(), 1, "one check, one result line\n{run}");
    assert_eq!(
        run.model_line("reads")[..3],
        ["fail", "reads", "8 states"],
        "Quint's TLC run finds 8 distinct states and breaks `small`\n{run}"
    );
}

#[test]
fn the_sample_has_the_verdicts_of_quints_tlc_and_typescript_runs() {
    let run = checked(&["verify", "specs/reads.qnt", "--json"]);
    run.exits(1);
    let report = run.json();
    let reads = model(&report, "reads");
    assert_eq!(reads["states"], 8, "{run}");
    for invariant in HOLDING {
        assert_eq!(
            rule(reads, invariant)["verdict"],
            "holds",
            "TLC and the TypeScript simulator find no violation of {invariant}\n{run}"
        );
    }
    assert_eq!(
        rule(reads, "small")["verdict"],
        "broken",
        "TLC breaks `small` once the log is [0, 1, 2]\n{run}"
    );
}

#[test]
fn get_only_element_of_a_set_of_two_is_a_fault_with_its_path() {
    let sandbox = Sandbox::empty();
    sandbox.write("models/onlyfault.json", &fixture("onlyfault.json"));
    let run = sandbox.earnest(&["verify", "models/onlyfault.json", "--json"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, "onlyfault");
    assert_eq!(found["states"], 3, "{run}");
    let fault = finding(found, "fault");
    assert!(
        fault["message"]
            .as_str()
            .is_some_and(|message| message.contains("that is not there")),
        "`Set(1, 2).getOnlyElement()` stops the step, as QNT505 stops Quint\n{run}"
    );
    assert_eq!(
        rule(found, "boolTest")["verdict"],
        "holds",
        "the run picks from `Bool` and passes, as `quint test` says\n{run}"
    );
}
