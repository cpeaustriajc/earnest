#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::binary;

const FAILING_COMPILE: &str = r##"#!/bin/sh
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
    cp "$here/parse.json" "$out"
    sed "s#\"0\":\"#\"0\":\"$folder/#" "$here/sourcemap.json" > "$map"
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
"##;

#[test]
fn a_compile_quint_fails_on_is_reported_in_the_qnt_file_and_not_in_quint_s_own_code() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    let program = fake.join("quint");
    fs::write(&program, FAILING_COMPILE).expect("the fake program can be written");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
    for (name, fixture) in [
        (
            "parse.json",
            include_str!("../../models/quint_fourth/named_helpers/parse.json"),
        ),
        (
            "sourcemap.json",
            include_str!("../../models/quint_fourth/named_helpers/named_helpers.sourcemap.json"),
        ),
        ("typecheck.json", ""),
    ] {
        fs::write(fake.join(name), fixture).expect("a fixture can be written");
    }
    fs::write(
        folder.path().join("named_helpers.qnt"),
        include_str!("../../models/quint_fourth/named_helpers/named_helpers.qnt"),
    )
    .expect("the spec can be written");
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

fn run_on(file: &str, json: &str, arguments: &[&str]) -> (Value, String) {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(folder.path().join(file), json).expect("the fixture can be written");
    let verified = Command::new(binary())
        .args(["verify", "--json", "--no-cache", file])
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let charted = Command::new(binary())
        .args(arguments)
        .arg(file)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    (
        serde_json::from_slice(&verified.stdout).expect("earnest verify prints JSON"),
        String::from_utf8_lossy(&charted.stdout).into_owned(),
    )
}

fn messages(report: &Value) -> Vec<String> {
    report["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|problem| problem["message"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn fairness_of_an_action_behind_a_named_guarded_action_is_read_as_the_same_guard_in_place() {
    let (report, _) = run_on(
        "named_step.json",
        include_str!("../../models/quint_fourth/fixes/named_step.json"),
        &["verify"],
    );
    let verdicts: Vec<(String, String)> = report["models"][0]["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|found| {
            (
                found["id"].as_str().unwrap_or_default().to_owned(),
                found["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        (verdicts, &report["models"][0]["states"]),
        (
            vec![
                ("onePays".to_owned(), "holds".to_owned()),
                ("twoPays".to_owned(), "broken".to_owned())
            ],
            &Value::from(8)
        ),
        "the step calls `pay(b)` only inside `payG = all {{ open, nondet b = … pay(b) }}`; \
         `weakFair(pay(1), …)` is about `pay` alone, as when the guard is written in place, and \
         `quint verify --backend tlc` finds `onePays` holds and `twoPays` violated in 8 states\n\
         {report:#}"
    );
}

#[test]
fn an_action_of_two_matches_is_named_after_both_and_a_run_names_a_step_as_the_search_does() {
    let (report, chart) = run_on(
        "names.json",
        include_str!("../../models/quint_fourth/fixes/names.json"),
        &["chart"],
    );
    let run = report["models"][0]["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|found| found["id"] == "handleTest")
        .cloned()
        .unwrap_or(Value::Null);
    assert!(
        ["both A C", "both A D", "both B C"]
            .iter()
            .all(|name| chart.contains(name))
            && run["path"]["steps"][0]["action"] == "handle Req  w = 1  m = Req",
        "`both` is an `all` of two `match`es, so each step is named after both branches; a run's \
         step `handle(1, Req)` reads like the search's `handle Req  …`, branch first, then the \
         written arguments\n{chart}\n{run:#}"
    );
}

#[test]
fn a_refusal_says_what_a_bare_fairness_an_implies_of_a_temporal_formula_and_an_empty_pick_are() {
    let (bare, _) = run_on(
        "bare.json",
        include_str!("../../models/quint_fourth/fixes/bare.json"),
        &["verify"],
    );
    let (empty, _) = run_on(
        "no_items.json",
        include_str!("../../models/quint_fourth/fixes/no_items.json"),
        &["verify"],
    );
    let said = [messages(&bare), messages(&empty)].concat();
    for part in [
        "`weakFair` alone, as a property or a part of an `and`, is not read: fairness is read only \
         as what a property assumes, `weakFair(A, vars) implies eventually(P)`",
        "For \"once P, then Q\" write `always(P implies eventually(Q))` or `P.leadsTo(Q)`",
        "fairness of `buy` given arguments the step never gives it is not read: TLC holds such a \
         step possible where the step never takes it. Here the step picks `p` from a set that is \
         empty",
    ] {
        assert!(
            said.iter().any(|message| message.contains(part)),
            "no message has {part:?}: {said:#?}"
        );
    }
}
