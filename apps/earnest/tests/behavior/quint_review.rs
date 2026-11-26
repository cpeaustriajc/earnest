#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::support::{Run, Sandbox, find_model, model, repository};

const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
main=""
invariants=""
temporal=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "--main" ]; then main="$argument"; fi
  if [ "$previous" = "--invariant" ]; then invariants="$argument"; fi
  if [ "$previous" = "--temporal" ]; then temporal="$argument"; fi
  if [ "$previous" = "--out" ]; then out="$argument"; fi
  if [ "$previous" = "--source-map" ]; then map="$argument"; fi
  previous="$argument"
  file="$argument"
done
case "$1" in
  parse|typecheck) file="$2" ;;
esac
name="$(basename "$file" .qnt)"
folder="$(cd "$(dirname "$file")" && pwd)"
set="$(basename "$folder")"
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  parse)
    cp "$here/fixtures/parsed/$set/$name.parse.json" "$out"
    sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" \
      "$here/fixtures/parsed/$set/$name.sourcemap.json" > "$map"
    ;;
  typecheck)
    if [ ! -f "$here/fixtures/typechecked/$set/$name.json" ]; then exit 1; fi
    cp "$here/fixtures/typechecked/$set/$name.json" "$out"
    ;;
  compile)
    fixture="$here/fixtures/compiled/$set/$main@$invariants@$temporal.json"
    if [ ! -f "$fixture" ]; then
      echo "no fixture for $set/$main@$invariants@$temporal" >&2
      exit 1
    fi
    cat "$fixture"
    ;;
esac
"##;

const SETS: [&str; 7] = [
    "samename",
    "exported",
    "nospace",
    "aliasruns",
    "noinit",
    "migrated",
    "leftover",
];

const TIGHT_DEFINES: [&str; 5] = ["neverFull", "Half", "belowHalf", "reset", "stepOrReset"];

fn review() -> PathBuf {
    repository().join("tasks/quint/review")
}

fn copy_folder(sandbox: &Sandbox, from: &Path, to: &str) {
    let entries = fs::read_dir(from)
        .unwrap_or_else(|error| panic!("{} can't be listed: {error}", from.display()));
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            copy_folder(sandbox, &path, &format!("{to}/{name}"));
        } else {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
            sandbox.write(&format!("{to}/{name}"), &text);
        }
    }
}

struct Quint {
    sandbox: Sandbox,
    program: PathBuf,
}

impl Quint {
    fn new() -> Self {
        let sandbox = Sandbox::empty();
        let program = sandbox.path().join("fake/quint");
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the fake can be made a program");
        copy_folder(&sandbox, &review().join("fixtures"), "fake/fixtures");
        for set in SETS {
            copy_folder(&sandbox, &review().join(set), set);
        }
        sandbox.write("fake/calls", "");
        Self { sandbox, program }
    }

    fn report(&self, set: &str) -> Value {
        let empty = self.sandbox.path().join("no-programs");
        fs::create_dir_all(&empty).expect("an empty folder can be made");
        Run::of(
            self.sandbox
                .command()
                .env("EARNEST_QUINT", &self.program)
                .env("PATH", &empty)
                .args(["verify", set, "--json", "--no-cache"]),
        )
        .json()
    }
}

fn unreadable(report: &Value) -> Vec<(String, String, String)> {
    report["unreadable"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|problem| {
            (
                problem["file"].as_str().unwrap_or_default().to_owned(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn verdicts(found: &Value) -> Vec<(String, String)> {
    found["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|rule| {
            (
                rule["id"].as_str().unwrap_or_default().to_owned(),
                rule["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn speaks_of(report: &Value, file: &str, name: &str) -> bool {
    unreadable(report).iter().any(|(each, feature, message)| {
        each.ends_with(file) && (feature.contains(name) || message.contains(name))
    })
}

#[test]
fn a_mark_no_check_sees_is_reported_though_another_file_has_a_module_of_the_same_name() {
    let report = Quint::new().report("samename");
    assert!(
        speaks_of(&report, "a.qnt", "small"),
        "`Lib::small` of a.qnt is marked and no check sees it; the `Lib` of b.qnt is another \
         module: {report}"
    );
}

#[test]
fn a_mark_exported_by_a_middle_module_is_judged_in_the_check_that_imports_that_module() {
    let report = Quint::new().report("exported");
    let top = find_model(&report, "Top")
        .unwrap_or_else(|| panic!("`Top` sees `init`, `step` and `small` through `Mid`'s `export Spec.*`, as `quint verify --main Top --invariant small` does: {report}"));
    assert!(
        verdicts(top).contains(&("small".to_owned(), "broken".to_owned())),
        "{top}"
    );
}

#[test]
fn a_mark_written_with_no_space_after_the_three_slashes_is_not_dropped_without_a_word() {
    let report = Quint::new().report("nospace");
    assert!(
        find_model(&report, "A").is_some() || speaks_of(&report, "nospace.qnt", "A"),
        "`///@check` above `module A` reaches earnest as the doc `check`, and `A`'s failing \
         `upTest` is never played, with nothing said: {report}"
    );
    let judged = verdicts(model(&report, "B"));
    assert!(
        judged.iter().any(|(id, _)| id == "wraps") || speaks_of(&report, "nospace.qnt", "wraps"),
        "`///@check broken` above `wraps` is dropped: {judged:?}"
    );
}

#[test]
fn a_run_reached_through_an_instance_under_a_name_is_played_when_a_marked_run_is_beside_it() {
    let report = Quint::new().report("aliasruns");
    let found = find_model(&report, "A").unwrap_or_else(|| {
        panic!("`quint test --main A` plays `A::S::upTest` and `A::S::badTest`: {report}")
    });
    assert_eq!(found["status"], "fail", "`S::badTest` fails: {found}");
    let judged = verdicts(found);
    assert!(
        judged
            .iter()
            .any(|(id, verdict)| id.ends_with("badTest") && verdict == "broken"),
        "{judged:?}"
    );
}

#[test]
fn a_marked_property_of_a_check_with_no_step_is_not_left_unjudged_without_a_word() {
    let report = Quint::new().report("noinit");
    let judged = find_model(&report, "N").map(verdicts).unwrap_or_default();
    assert!(
        judged.iter().any(|(id, _)| id == "small") || speaks_of(&report, "noinit.qnt", "small"),
        "`small` is marked `@check` and false, and `N` passes on its run alone: {report}"
    );
}

#[test]
fn the_module_the_leftover_checks_file_asks_for_defines_each_name_of_its_setup_once() {
    let report = Quint::new().report("leftover");
    let (_, _, message) = unreadable(&report)
        .into_iter()
        .find(|(file, ..)| file.ends_with("earnest.quint.json"))
        .unwrap_or_else(|| panic!("{report}"));
    for name in TIGHT_DEFINES {
        for keyword in ["val", "action", "pure val"] {
            let added = format!("{keyword} {name} =");
            assert!(
                !message.lines().any(|line| line.trim().starts_with(&added)),
                "the message says to copy `Tight` and add `{added} …` to the copy, which \
                 already defines `{name}`: Quint rejects the file with QNT101: {message}"
            );
        }
    }
}

#[test]
fn following_the_message_for_the_setups_checks_file_keeps_the_verdicts_of_two_and_large() {
    let report = Quint::new().report("migrated");
    let large = find_model(&report, "Large")
        .unwrap_or_else(|| panic!("`Large` passed with its checks file: {report}"));
    assert_eq!(large["status"], "pass", "{large}");
    let two = model(&report, "TwoChecked");
    assert_eq!(
        two["status"], "pass",
        "`Two` judged `ok` and `fine` and passed; the message's marks on `Base` for `Exported` \
         and `Plain` also land in `TwoChecked`, and no line of it says so: {two}"
    );
}
