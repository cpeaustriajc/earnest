#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::binary;

const NOTES_A_CALL: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
touch "$(dirname "$(readlink -f "$0")")/called"
exit 1
"#;

#[test]
fn a_spec_quint_s_flattener_stops_on_is_read_and_no_quint_is_started() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    let program = fake.join("quint");
    fs::write(&program, NOTES_A_CALL).expect("the fake program can be written");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
    fs::write(
        folder.path().join("named_helpers.qnt"),
        include_str!("../../models/quint_fourth/named_helpers/named_helpers.qnt"),
    )
    .expect("the spec can be written");
    let output = Command::new(binary())
        .args(["verify", "--json", "--no-cache", "named_helpers.qnt"])
        .env("EARNEST_QUINT", &program)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let report: Value = serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON");
    let small = &report["models"][0];
    let verdicts: Vec<(&str, &str)> = small["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|rule| {
            (
                rule["id"].as_str().unwrap_or(""),
                rule["verdict"].as_str().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(
        (
            small["name"].as_str(),
            small["status"].as_str(),
            small["states"].as_u64(),
            verdicts
        ),
        (
            Some("small"),
            Some("pass"),
            Some(3),
            vec![("inRange", "holds"), ("upTest", "holds")]
        ),
        "the spec imports its helpers under a name, on which `quint compile --main small` of \
         Quint 0.33.1 stops with `Internal error while flattening`; the check is built from \
         the typecheck, so it is read, and `upTest` is played as `quint test` plays it\n{report:#}"
    );
    assert!(
        !fake.join("called").exists(),
        "the checker reads the `.qnt` file itself: the program `EARNEST_QUINT` names is never \
         started, where a failing `quint compile` used to be reported at the check's file"
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
