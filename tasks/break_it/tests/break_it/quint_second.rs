#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Report, Sandbox, binary, rule};

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

const FAKE_JAVA: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
previous=""
for argument in "$@"; do
  if [ "$previous" = "-config" ]; then config="$argument"; fi
  previous="$argument"
done
cat "$here/$(basename "$config" .cfg).tlc.txt"
"#;

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

fn faults(report: &Report, model: &str) -> Vec<String> {
    report.model(model)["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding["kind"] == "fault")
        .filter_map(|finding| finding["message"].as_str().map(str::to_owned))
        .collect()
}

fn program(folder: &Path, name: &str, text: &str) {
    let path = folder.join(name);
    fs::write(&path, text).expect("the fake program can be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
}

#[test]
fn a_lambda_inside_a_temporal_property_is_judged_as_tlc_judges_it() {
    let report = compiled(
        "temporal_lambda",
        include_str!("../../models/quint_second/temporal_lambda.json"),
    );
    let model = report.model("temporal_lambda");
    assert!(
        faults(&report, "temporal_lambda").is_empty() && rule(model, "small")["verdict"] == "holds",
        "`always(s.forall(e => e < 9))` holds in both states (`Set()` and `Set(5)`): `quint \
         verify --backend tlc --temporal small` prints `[ok] No violation found`; the same \
         condition as an invariant holds in earnest, so a lambda in a temporal property is no \
         fault\n{report}"
    );
}

#[test]
fn a_nondet_inside_a_match_branch_is_picked_only_where_the_branch_matches() {
    let report = compiled(
        "match_nondet",
        include_str!("../../models/quint_second/match_nondet.json"),
    );
    let model = report.model("match_nondet");
    assert!(
        faults(&report, "match_nondet").is_empty()
            && model["states"] == 4
            && rule(model, "small")["verdict"] == "holds",
        "`| B(s) => all {{ nondet w = s.oneOf() ... }}` picks from `s` only where `t` is a `B`: \
         from `t = A(1)` only the `A` branch can happen; `quint verify --backend tlc` finds 4 \
         distinct states and `[ok]` for `small`, and `quint run --backend typescript` finds no \
         violation\n{report}"
    );
}

#[test]
fn a_fold_over_tuples_of_ten_items_takes_them_in_the_order_tlc_takes_them() {
    let report = compiled(
        "fold_tuple10",
        include_str!("../../models/quint_second/fold_tuple10.json"),
    );
    assert_eq!(
        rule(report.model("fold_tuple10"), "lastFirst")["verdict"],
        "holds",
        "TLC compares two tuples item by item from the first, so `(0, ..., 0, 1)` comes before \
         `(0, 1, 0, ..., 0)`: `quint verify --backend tlc --invariant lastFirst` and `quint \
         run --backend typescript` both find no violation\n{report}"
    );
}

#[test]
fn the_runs_of_a_spec_imported_under_a_name_are_played_as_quint_test_plays_them() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    program(&fake, "quint", FAKE_QUINT);
    for (name, text) in [
        (
            "typecheck.json",
            include_str!("../../models/quint_second/named_runs/typecheck.json"),
        ),
        (
            "compiled.json",
            include_str!("../../models/quint_second/named_runs/Named.json"),
        ),
    ] {
        fs::write(fake.join(name), text).expect("a fixture can be written");
    }
    fs::write(
        folder.path().join("named_runs.qnt"),
        include_str!("../../models/quint_second/named_runs/named_runs.qnt"),
    )
    .expect("the spec can be written");
    fs::write(
        folder.path().join("earnest.quint.json"),
        r#"{ "checks": [ { "file": "named_runs.qnt", "main": "Named", "init": "C::init", "step": "C::step", "runs": true } ] }"#,
    )
    .expect("the checks file can be written");
    let output = Command::new(binary())
        .args(["verify", "--json", "--no-cache", "."])
        .env("EARNEST_QUINT", fake.join("quint"))
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let report: Value = serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON");
    let named = report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == "Named"))
        .unwrap_or(&Value::Null);
    assert_eq!(
        rule(named, "upTest")["verdict"],
        "holds",
        "`quint test --main Named` plays `Named::C::upTest` and prints `ok Named::C::upTest \
         passed 1 test(s)`; the docs say `\"runs\": true` plays the spec's runs whose name ends \
         in `Test`, `C::name` under a name, and the same check beside a second check of the \
         file plays it\n{report:#}"
    );
}

fn confirmed_with_tlc_as_it_prints_today(
    name: &str,
    json: &str,
    outputs: &[(&str, &str)],
) -> String {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    program(&fake, "java", FAKE_JAVA);
    for (config, text) in outputs {
        fs::write(fake.join(format!("{config}.tlc.txt")), text)
            .expect("a TLC output can be written");
    }
    let jar = fake.join("tla2tools.jar");
    fs::write(&jar, "").expect("the jar's place can be written");
    let file = format!("{name}.json");
    fs::write(folder.path().join(&file), json).expect("the module can be written");
    let output = Command::new(binary())
        .arg("confirm")
        .arg("--tlc")
        .arg(&jar)
        .arg(&file)
        .env("PATH", format!("{}:/usr/bin:/bin", fake.display()))
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn confirm_reads_a_temporal_property_that_tlc_names_as_violated_as_broken() {
    let said = confirmed_with_tlc_as_it_prints_today(
        "broken_eventually",
        include_str!("../../models/quint_second/confirm/broken_eventually.json"),
        &[
            (
                "count",
                include_str!("../../models/quint_second/confirm/broken_eventually.count.tlc.txt"),
            ),
            (
                "eventually0",
                include_str!(
                    "../../models/quint_second/confirm/broken_eventually.eventually0.tlc.txt"
                ),
            ),
        ],
    );
    assert!(
        said.starts_with("same "),
        "TLC 2026.11.26 (the `tla2tools.jar` of the v1.8.0 release and the TLC in Apalache \
         0.62.4's jar) prints `Error: Temporal property Eventually_finish was violated.` for \
         the export of `eventually(done)` with no fairness, which earnest judges broken: TLC \
         agrees, so confirm says `same`\n{said}"
    );
}

#[test]
fn confirm_reads_an_invariant_that_tlc_finds_false_without_a_state_as_broken() {
    let said = confirmed_with_tlc_as_it_prints_today(
        "constant_false",
        include_str!("../../models/quint_second/confirm/constant_false.json"),
        &[
            (
                "count",
                include_str!("../../models/quint_second/confirm/constant_false.count.tlc.txt"),
            ),
            (
                "rule0",
                include_str!("../../models/quint_second/confirm/constant_false.rule0.tlc.txt"),
            ),
        ],
    );
    assert!(
        said.starts_with("same "),
        "`val never = Set(1, 2).size() == 3` reads no variable, so TLC judges it before the \
         search and prints `Error: The invariant of Rule_never is equal to FALSE`: the rule is \
         false, as earnest says, so confirm says `same`\n{said}"
    );
}
