#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Report, Sandbox, binary};

const FAKE_QUINT: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
previous=""
for argument in "$@"; do
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
    cp "$here/typecheck.json" "$out"
    echo "{\"sourceIndex\":{\"0\":\"$folder/$(basename "$2")\"},\"map\":{}}" > "$map"
    ;;
  typecheck)
    cp "$here/typecheck.json" "$out"
    ;;
  compile)
    cat "$here/compiled.json"
    ;;
esac
"#;

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

fn program(folder: &Path, name: &str, text: &str) {
    let path = folder.join(name);
    fs::write(&path, text).expect("the fake program can be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
}

struct QuintFixtures<'a> {
    file: &'a str,
    source: &'a str,
    typecheck: &'a str,
    compiled: &'a str,
    checks: &'a str,
}

struct RecordedQuint {
    folder: TempDir,
    quint: PathBuf,
}

impl RecordedQuint {
    fn new(fixtures: &QuintFixtures<'_>) -> Self {
        let folder = TempDir::new().expect("a temporary folder can be made");
        let fake = folder.path().join("fake");
        fs::create_dir_all(&fake).expect("the fake's folder can be made");
        program(&fake, "quint", FAKE_QUINT);
        for (name, text) in [
            ("typecheck.json", fixtures.typecheck),
            ("compiled.json", fixtures.compiled),
        ] {
            fs::write(fake.join(name), text).expect("a fixture can be written");
        }
        let recorded = Self {
            folder,
            quint: fake.join("quint"),
        };
        recorded.write(fixtures.file, fixtures.source);
        recorded.write("earnest.quint.json", fixtures.checks);
        recorded
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.folder.path().join(name), text).expect("a file can be written");
    }

    fn verify(&self, arguments: &[&str]) -> Value {
        let output = Command::new(binary())
            .args(["verify", "--json"])
            .args(arguments)
            .arg(".")
            .env("EARNEST_QUINT", &self.quint)
            .current_dir(self.folder.path())
            .output()
            .expect("earnest starts");
        serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON")
    }
}

fn model_named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == name))
        .unwrap_or(&Value::Null)
}

#[test]
fn a_temporal_property_that_is_a_forall_over_an_empty_set_is_still_a_rule() {
    let report = compiled(
        "empty_forall",
        include_str!("../../models/quint_third/empty_forall.json"),
    );
    if report.is_unreadable() {
        return;
    }
    let rules = report.model("empty_forall")["rules"].clone();
    let listed = rules
        .as_array()
        .into_iter()
        .flatten()
        .find(|found| found["id"] == "everyonePays");
    assert_eq!(
        listed.map(|found| found["verdict"].clone()),
        Some(Value::from("holds")),
        "`everyonePays` is `Buyers.forall(b => eventually(...))` over the empty set: an `and` \
         of no parts, true in every run. The check lists it, so it is a rule that holds (or \
         the file is refused naming it); it is never left out of the result, where `--rule \
         everyonePays` then says the model has no such rule and `expectCounterexample` that the \
         module has no such property\n{report}"
    );
}

#[test]
fn the_note_of_runs_not_played_names_only_runs_quint_test_plays() {
    let recorded = RecordedQuint::new(&QuintFixtures {
        file: "one_name.qnt",
        source: include_str!("../../models/quint_third/one_name/one_name.qnt"),
        typecheck: include_str!("../../models/quint_third/one_name/typecheck.json"),
        compiled: include_str!("../../models/quint_third/one_name/compiled.json"),
        checks: r#"{ "checks": [ { "file": "one_name.qnt", "main": "OneName", "runs": true } ] }"#,
    });
    let report = recorded.verify(&["--no-cache"]);
    let model = model_named(&report, "OneName");
    let named: Vec<&str> = model["runsNotPlayed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        !named.contains(&"specOverTest"),
        "`OneName` imports `Spec.specTest` alone (with `init`, `step` and `v`), so `quint test \
         --main OneName` plays `specTest` and `ownTest` and never `specOverTest`; the note says \
         it did not play `specOverTest`, which no Quint command plays here\n{report:#}"
    );
}

const SEEDED: [(&str, &str); 10] = [
    (
        "R6",
        include_str!("../../models/quint_third/random/R6.json"),
    ),
    (
        "R9",
        include_str!("../../models/quint_third/random/R9.json"),
    ),
    (
        "R13",
        include_str!("../../models/quint_third/random/R13.json"),
    ),
    (
        "R14",
        include_str!("../../models/quint_third/random/R14.json"),
    ),
    (
        "R15",
        include_str!("../../models/quint_third/random/R15.json"),
    ),
    (
        "R16",
        include_str!("../../models/quint_third/random/R16.json"),
    ),
    (
        "R25",
        include_str!("../../models/quint_third/random/R25.json"),
    ),
    (
        "R30",
        include_str!("../../models/quint_third/random/R30.json"),
    ),
    (
        "R35",
        include_str!("../../models/quint_third/random/R35.json"),
    ),
    (
        "R36",
        include_str!("../../models/quint_third/random/R36.json"),
    ),
];

fn whole_verdicts(model: &Value) -> serde_json::Map<String, Value> {
    let mut verdicts = serde_json::Map::new();
    for found in model["rules"].as_array().into_iter().flatten() {
        let id = found["id"].as_str().unwrap_or_default();
        let base = id.split('#').next().unwrap_or(id).to_owned();
        let verdict = found["verdict"].as_str().unwrap_or_default();
        let earlier = verdicts.get(&base).and_then(Value::as_str);
        let joined = [Some("broken"), Some("unknown")]
            .into_iter()
            .flatten()
            .find(|worst| earlier == Some(*worst) || verdict == *worst)
            .unwrap_or(verdict)
            .to_owned();
        verdicts.insert(base, Value::from(joined));
    }
    verdicts
}

#[test]
fn every_seeded_model_of_buyers_has_the_states_and_verdicts_of_quint_s_tlc_run_and_quint_test() {
    let expected: Value = serde_json::from_str(include_str!(
        "../../models/quint_third/random/expected.json"
    ))
    .expect("the expected answers are JSON");
    for (name, json) in SEEDED {
        for jobs in ["1", "4"] {
            let file = format!("{name}.json");
            let report = Sandbox::with(&[(file.as_str(), json)]).verify(&[&file, "--jobs", jobs]);
            let model = report.model(name);
            let wanted = &expected[name];
            assert_eq!(
                (&model["states"], Value::Object(whole_verdicts(model))),
                (&wanted["states"], wanted["verdicts"].clone()),
                "`{name}` with {jobs} jobs: the states `quint verify --backend tlc` counts, \
                 each invariant and temporal property as TLC judges it (a property in parts \
                 broken when one part is), and each run as `quint test` judges it\n{report}"
            );
        }
    }
}
