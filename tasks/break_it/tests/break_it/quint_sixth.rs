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

fn verified(case: &str, arguments: &[&str]) -> Value {
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
        .args(["verify", "--json", "--no-cache", "."])
        .args(arguments)
        .env("EARNEST_QUINT", &program)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "earnest verify prints JSON ({error}):\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
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
        message.contains("import Spec(N = 2) as V") && message.contains("action init = V::start"),
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
