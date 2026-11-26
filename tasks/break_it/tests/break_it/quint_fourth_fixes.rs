#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::binary;

const FAILING_COMPILE: &str = r#"#!/bin/sh
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
    echo "Error: Internal error while flattening [QNT404] Name 'spec::H::up' not found" >&2
    echo "    at resolveNamesOrThrow (/opt/quint/node_modules/@informalsystems/quint/dist/src/flattening/fullFlattener.js:90:15)" >&2
    echo "    at /opt/quint/node_modules/@informalsystems/quint/dist/src/cli.js:58:39" >&2
    exit 1
    ;;
esac
"#;

#[test]
fn a_compile_quint_fails_on_is_reported_in_the_qnt_file_and_not_in_quint_s_own_code() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    let program = fake.join("quint");
    fs::write(&program, FAILING_COMPILE).expect("the fake program can be written");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
    fs::write(
        fake.join("typecheck.json"),
        include_str!("../../models/quint_fourth/named_helpers/typecheck.json"),
    )
    .expect("a fixture can be written");
    fs::write(
        folder.path().join("named_helpers.qnt"),
        include_str!("../../models/quint_fourth/named_helpers/named_helpers.qnt"),
    )
    .expect("the spec can be written");
    fs::write(
        folder.path().join("earnest.quint.json"),
        r#"{ "checks": [ { "file": "named_helpers.qnt", "main": "spec", "invariants": ["inRange"] } ] }"#,
    )
    .expect("the checks file can be written");
    let output = Command::new(binary())
        .args(["verify", "--json", "--no-cache", "."])
        .env("EARNEST_QUINT", &program)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let report: Value = serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON");
    let problem = &report["unreadable"][0];
    assert!(
        problem["file"] == "named_helpers.qnt"
            && problem["line"] == 1
            && problem["message"]
                .as_str()
                .is_some_and(|message| message.contains("Internal error while flattening")),
        "`quint compile` stops with its own error and a stack of its own files; the problem is \
         at the check's file with Quint's message, not at `…/quint/dist/src/cli.js:58:39`\n\
         {report:#}"
    );
}

#[test]
fn the_tla_export_states_the_fairness_of_the_action_alone_without_the_step_s_guard() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(
        folder.path().join("guarded_step.json"),
        include_str!("../../models/quint_fourth/guarded_fair/guarded_step.json"),
    )
    .expect("the fixture can be written");
    let output = Command::new(binary())
        .args(["tla", "guarded_step.json", "--out", "exported"])
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let module = fs::read_to_string(folder.path().join("exported/guarded_step.tla"))
        .unwrap_or_else(|error| {
            panic!(
                "the module is written: {error}\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    assert!(
        module
            .contains("Fair_pay_1_way_1 ==\n    IF InRange /\\ (~paid)\n    THEN /\\ paid' = TRUE")
            && module.contains("(WF_vars(Fair_pay_1)) =>")
            && module.contains("Action_pay ==\n    IF InRange /\\ ((open \\/ paid) /\\ (~paid))"),
        "the step's `pay` keeps the guard `open or paid` the step puts around it, and the \
         fairness is `WF_vars` of `pay` alone, guarded by `not(paid)` only, as TLC reads \
         `weakFair(pay, (paid, open))`\n{module}"
    );
}
