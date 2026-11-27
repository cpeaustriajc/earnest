#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

use crate::support::{Run, Sandbox, model, repository};

const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
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
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  parse)
    cp "$here/fixtures/parsed/$name.parse.json" "$out"
    sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" \
      "$here/fixtures/parsed/$name.sourcemap.json" > "$map"
    ;;
  typecheck)
    if [ ! -f "$here/fixtures/typechecked/$name.json" ]; then exit 1; fi
    cp "$here/fixtures/typechecked/$name.json" "$out"
    ;;
  compile)
    fixture="$here/fixtures/compiled/$main@$invariants@$temporal.json"
    if [ ! -f "$fixture" ]; then
      echo "no fixture for $main@$invariants@$temporal" >&2
      exit 1
    fi
    cat "$fixture"
    ;;
esac
"##;

const FAILS_AS_MARKED: [&str; 1] = ["tallyfails"];

fn samples() -> PathBuf {
    repository().join("tasks/quint/samples")
}

fn copy_folder(sandbox: &Sandbox, from: &PathBuf, to: &str) {
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

fn sample_names() -> BTreeSet<String> {
    fs::read_dir(samples())
        .expect("the samples folder can be listed")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".qnt")
                .map(str::to_owned)
        })
        .collect()
}

fn recorded() -> Value {
    let path = samples().join("fixtures/expected.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
    serde_json::from_str(&text).expect("expected.json is JSON")
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
        copy_folder(&sandbox, &samples().join("fixtures"), "fake/fixtures");
        for name in sample_names() {
            let text = fs::read_to_string(samples().join(format!("{name}.qnt")))
                .unwrap_or_else(|error| panic!("{name}.qnt can't be read: {error}"));
            sandbox.write(&format!("samples/{name}.qnt"), &text);
        }
        Self { sandbox, program }
    }

    fn verify(&self) -> Run {
        let empty = self.sandbox.path().join("no-programs");
        fs::create_dir_all(&empty).expect("an empty folder can be made");
        Run::of(
            self.sandbox
                .command()
                .env("EARNEST_QUINT", &self.program)
                .env("PATH", &empty)
                .args(["verify", "samples", "--json"]),
        )
    }
}

fn judged(model: &Value) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = model["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|rule| {
            (
                rule["id"]
                    .as_str()
                    .unwrap_or_default()
                    .rsplit("::")
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
                rule["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    found.sort();
    found
}

fn wanted(entry: &Value) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = ["rules", "runs"]
        .iter()
        .flat_map(|kind| entry[kind].as_object().cloned().unwrap_or_default())
        .map(|(id, verdict)| (id, verdict.as_str().unwrap_or_default().to_owned()))
        .collect();
    found.sort();
    found
}

#[test]
fn every_sample_has_its_fixtures_recorded_from_quint_and_every_recording_has_its_sample() {
    let recorded = recorded();
    let listed: BTreeSet<String> = recorded
        .as_object()
        .expect("expected.json maps each sample")
        .keys()
        .cloned()
        .collect();
    assert_eq!(listed, sample_names(), "record.py was run on these samples");
}

#[test]
fn each_sample_that_reads_has_the_state_count_and_the_verdicts_quint_gave() {
    let run = Quint::new().verify();
    let report = run.json();
    for (name, entry) in recorded()
        .as_object()
        .expect("expected.json maps each sample")
    {
        if FAILS_AS_MARKED.contains(&name.as_str()) {
            continue;
        }
        let module = entry["module"]
            .as_str()
            .expect("each entry names its module");
        let checked = model(&report, module);
        assert_eq!(
            checked["states"].as_u64().unwrap_or(0),
            entry["states"].as_u64().unwrap_or(0),
            "{name}: TLC's count of distinct states\n{run}"
        );
        assert_eq!(
            judged(checked),
            wanted(entry),
            "{name}: Quint's verdicts, from `quint verify --backend tlc` and `quint test`\n{run}"
        );
    }
}

#[test]
fn a_pure_run_that_fails_by_an_assert_is_broken_as_quint_test_fails_it() {
    let run = Quint::new().verify();
    let report = run.json();
    assert_eq!(
        report["unreadable"].as_array().map(Vec::len),
        Some(0),
        "every sample reads\n{run}"
    );
    let checked = model(&report, "tallyfails");
    let wrong = checked["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|rule| rule["id"] == "wrongTest")
        .unwrap_or(&Value::Null);
    assert_eq!(
        (&wrong["verdict"], &wrong["expected"]),
        (&Value::from("broken"), &Value::from("broken")),
        "`quint test --main tallyfails` prints `wrongTest failed after 1 test(s)` (QNT508, \
         Assertion failed), and the run is marked `@check broken`\n{run}"
    );
    assert_eq!(
        checked["status"], "pass",
        "the one run breaks as marked\n{run}"
    );
}
