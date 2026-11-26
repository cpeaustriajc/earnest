use std::fs;
use std::process::Command;

use serde_json::{Value, json};

use crate::support::{Run, Sandbox, models, named, rule, without_timings};

const CLOCK: &str = "runs/clock.model.ts";

const BEHIND_DETAIL: [&str; 2] = [
    "broken  close  \"The clock is at most 5 behind\"",
    "start  skew = -10, hand = 0, synced = false",
];

const AHEAD_DETAIL: [&str; 10] = [
    "broken  syncs  \"The clock syncs\"",
    "start  skew = 10, hand = 0, synced = false",
    "1. tick  hand: 0 -> 1",
    "2. tick  hand: 1 -> 2",
    "3. tick  hand: 2 -> 0",
    "end    skew = 10, hand = 0, synced = false",
    "why    steps 1 to 3 can repeat forever",
    "broken  syncsAhead  \"A clock ahead can sync\"",
    "start  skew = 10, hand = 0, synced = false",
    "why    step 1, `sync`, can't happen here",
];

fn clock() -> Sandbox {
    Sandbox::with_runs(&["clock.model.ts"])
}

fn verdicts(found: &Value) -> Vec<(String, String)> {
    found["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("the run has a `rules` array: {found}"))
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn listed(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

fn run_in<'a>(report: &'a Value, setup: Option<&str>) -> &'a Value {
    models(report)
        .iter()
        .find(|found| found["setup"]["id"].as_str() == setup)
        .unwrap_or_else(|| panic!("no run in the setup {setup:?} in {report}"))
}

fn result_names(run: &Run) -> Vec<String> {
    run.model_lines()
        .iter()
        .filter_map(|line| line.get(1).cloned())
        .collect()
}

fn written(sandbox: &Sandbox, folder: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(sandbox.path().join(folder))
        .unwrap_or_else(|error| panic!("{folder} can't be listed: {error}"))
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn git(sandbox: &Sandbox, arguments: &[&str]) {
    let output = Command::new("git")
        .current_dir(sandbox.path())
        .args(["-c", "user.name=Conformance"])
        .args(["-c", "user.email=conformance@earnest.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(["-c", "init.defaultBranch=main"])
        .args(arguments)
        .output()
        .expect("`git` is on the `PATH`");
    assert!(
        output.status.success(),
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_model_with_two_setups_prints_three_result_lines_in_declaration_order() {
    let sandbox = clock();
    let run = sandbox.earnest(&["verify"]);
    run.exits(1);
    assert_eq!(
        result_names(&run),
        ["Clock", "Clock (behind)", "Clock (ahead)"],
        "the model as declared, then each setup in file order\n{run}"
    );
    for (name, status, states) in [
        ("Clock", "pass", "6 states"),
        ("Clock (behind)", "fail", "6 states"),
        ("Clock (ahead)", "fail", "3 states"),
    ] {
        assert_eq!(
            run.model_line(name).get(..3),
            Some([status, name, states].map(str::to_owned).as_slice()),
            "the result line of {name}, with its own state count\n{run}"
        );
    }
    assert_eq!(run.summary_total(), 3, "every run is counted\n{run}");
    assert_eq!(run.summary_count("pass"), 1, "the model's own run\n{run}");
    assert_eq!(run.summary_count("fail"), 2, "both setups\n{run}");
    assert_eq!(
        run.detail("Clock"),
        Vec::<String>::new(),
        "the model as declared passes\n{run}"
    );
    assert_eq!(
        run.detail("Clock (behind)"),
        BEHIND_DETAIL,
        "the rule only this setup breaks, from the setup's values\n{run}"
    );
    assert_eq!(
        run.detail("Clock (ahead)"),
        AHEAD_DETAIL,
        "the rule about runs and the witness this setup breaks\n{run}"
    );
}

#[test]
fn the_json_has_a_model_entry_per_setup_naming_the_setup() {
    let sandbox = clock();
    let run = sandbox.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(models(&report).len(), 3, "three runs\n{run}");
    let own = run_in(&report, None);
    assert!(
        own.get("setup").is_none(),
        "the model as declared names no setup: {own}"
    );
    assert_eq!(own["name"], "Clock", "the model's name");
    assert_eq!(own["states"], 6, "its states");
    assert_eq!(
        verdicts(own),
        listed(&[
            ("close", "holds"),
            ("syncs", "holds"),
            ("goesRound", "holds")
        ]),
        "the witness without a setup is listed here alone\n{run}"
    );
    let behind = run_in(&report, Some("behind"));
    assert_eq!(behind["name"], "Clock", "the same name\n{run}");
    assert_eq!(
        behind["setup"],
        json!({ "id": "behind", "says": "The clock runs 10 behind" }),
        "the setup's id and sentence"
    );
    assert_eq!(behind["status"], "fail", "the setup fails");
    assert_eq!(behind["states"], 6, "its own states");
    assert_eq!(
        verdicts(behind),
        listed(&[
            ("close", "broken"),
            ("syncs", "holds"),
            ("catchesUp", "holds")
        ]),
        "the witness that names this setup is listed here alone\n{run}"
    );
    assert_eq!(
        rule(behind, "close")["path"],
        json!({
            "start": named(&[("skew", json!(-10)), ("hand", json!(0)), ("synced", json!(false))]),
            "steps": [],
        }),
        "the path starts at the setup's values"
    );
    let ahead = run_in(&report, Some("ahead"));
    assert_eq!(
        ahead["setup"],
        json!({ "id": "ahead", "says": "The clock runs 10 ahead" }),
        "the other setup"
    );
    assert_eq!(
        ahead["states"], 3,
        "`sync` never happens, so `synced` stays false"
    );
    assert_eq!(
        verdicts(ahead),
        listed(&[
            ("close", "holds"),
            ("syncs", "broken"),
            ("syncsAhead", "broken")
        ]),
        "each setup has its own verdicts\n{run}"
    );
    assert_eq!(
        rule(ahead, "syncs")["repeatsFrom"],
        0,
        "the `tick` loop starts at the setup's start"
    );
    assert_eq!(
        rule(ahead, "syncsAhead")["reason"],
        "step 1, `sync`, can't happen here",
        "the witness is played from the setup's start"
    );
}

#[test]
fn a_model_name_selects_every_run_of_the_model() {
    let sandbox = Sandbox::with_runs(&["clock.model.ts", "panel.model.ts"]);
    let run = sandbox.earnest(&["verify", "--model", "Clock"]);
    run.exits(1);
    assert_eq!(
        result_names(&run),
        ["Clock", "Clock (behind)", "Clock (ahead)"],
        "the model and both its setups, and not the panel\n{run}"
    );
}

#[test]
fn a_rule_id_selects_the_runs_that_list_it() {
    let sandbox = clock();
    let witness = sandbox.earnest(&["verify", "--rule", "catchesUp"]);
    witness.exits(0);
    assert_eq!(
        result_names(&witness),
        ["Clock (behind)"],
        "only the setup the witness names lists it\n{witness}"
    );
    let own = sandbox.earnest(&["verify", "--rule", "goesRound", "--json"]);
    own.exits(0);
    let report = own.json();
    assert_eq!(models(&report).len(), 1, "only the model's own run\n{own}");
    assert_eq!(
        verdicts(run_in(&report, None)),
        listed(&[("goesRound", "holds")]),
        "only that witness\n{own}"
    );
    let every = sandbox.earnest(&["verify", "--rule", "syncs"]);
    every.exits(1);
    assert_eq!(
        result_names(&every),
        ["Clock", "Clock (behind)", "Clock (ahead)"],
        "a rule every run lists selects them all, with no need for `--model`\n{every}"
    );
    assert_eq!(
        every.detail("Clock (ahead)"),
        AHEAD_DETAIL.get(..7).unwrap_or_default(),
        "only the rule asked for\n{every}"
    );
}

#[test]
fn the_cache_keeps_each_run_apart() {
    let sandbox = clock();
    sandbox.earnest(&["verify"]).exits(1);
    let second = sandbox.earnest(&["verify"]);
    second.exits(1);
    for name in ["Clock", "Clock (behind)", "Clock (ahead)"] {
        assert_eq!(
            second.model_line(name).last().map(String::as_str),
            Some("(cached)"),
            "{name} comes from the cache\n{second}"
        );
    }
    assert_eq!(
        second.detail("Clock (behind)"),
        BEHIND_DETAIL,
        "each run keeps its own detail\n{second}"
    );
    assert_eq!(
        second.detail("Clock (ahead)"),
        AHEAD_DETAIL,
        "each run keeps its own detail\n{second}"
    );
    let fresh = sandbox.earnest(&["verify", "--json", "--no-cache"]).json();
    let cached = sandbox.earnest(&["verify", "--json"]).json();
    assert_eq!(
        without_timings(&cached),
        without_timings(&fresh),
        "the cached JSON is that of a fresh run, setup by setup"
    );
}

#[test]
fn a_changed_setup_is_checked_again() {
    let sandbox = clock();
    sandbox.earnest(&["verify"]).exits(1);
    let changed = sandbox.read(CLOCK).replace("{ skew: 10 }", "{ skew: 5 }");
    sandbox.write(CLOCK, &changed);
    let run = sandbox.earnest(&["verify"]);
    run.exits(1);
    assert_eq!(
        run.model_line("Clock (ahead)").get(..3),
        Some(
            ["pass", "Clock (ahead)", "6 states"]
                .map(str::to_owned)
                .as_slice()
        ),
        "at `skew` 5 the clock can sync: the new file is checked\n{run}"
    );
}

#[test]
fn the_report_has_a_section_per_run() {
    let sandbox = clock();
    let run = sandbox.earnest(&["report"]);
    for heading in ["## Clock", "## Clock \\(behind\\)", "## Clock \\(ahead\\)"] {
        assert!(
            run.stdout.lines().any(|line| line == heading),
            "the report has the heading `{heading}`\n{run}"
        );
    }
    for line in BEHIND_DETAIL.iter().chain(&AHEAD_DETAIL) {
        assert!(
            run.stdout.lines().any(|printed| printed.trim() == *line),
            "the report has the line `{line}`\n{run}"
        );
    }
}

#[test]
fn the_tla_export_writes_a_module_per_setup_with_its_start_values() {
    let sandbox = clock();
    let run = sandbox.earnest(&["tla", "--out", "tla", CLOCK]);
    run.exits(0);
    assert_eq!(
        written(&sandbox, "tla"),
        [
            "Clock.cfg",
            "Clock.tla",
            "Clock_ahead.cfg",
            "Clock_ahead.tla",
            "Clock_behind.cfg",
            "Clock_behind.tla",
        ],
        "a module and a configuration for the model and for each setup\n{run}"
    );
    for (module, skew) in [
        ("Clock", "0"),
        ("Clock_behind", "(-10)"),
        ("Clock_ahead", "10"),
    ] {
        let text = sandbox.read(&format!("tla/{module}.tla"));
        assert!(
            text.starts_with(&format!("---- MODULE {module} ----\n")),
            "the module is named after the model and the setup\n{text}"
        );
        assert!(
            text.contains(&format!(
                "Init ==\n    /\\ skew = {skew}\n    /\\ hand = 0\n"
            )),
            "{module} starts `skew` at {skew}\n{text}"
        );
        assert_eq!(
            sandbox.read(&format!("tla/{module}.cfg")),
            "SPECIFICATION Spec\nCHECK_DEADLOCK FALSE\nINVARIANT Rule_close\nPROPERTY Eventually_syncs\n",
            "{module}: the invariant, then the property"
        );
        for witness in ["goesRound", "catchesUp", "syncsAhead"] {
            assert!(
                !text.contains(witness),
                "{module}: the witness `{witness}` is not exported\n{text}"
            );
        }
    }
}

#[test]
fn rule_changes_against_the_base_are_listed_once_per_model() {
    let sandbox = clock();
    git(&sandbox, &["init", "--quiet"]);
    git(&sandbox, &["add", CLOCK]);
    git(
        &sandbox,
        &["commit", "--quiet", "--message", "Model the clock"],
    );
    let loosened = sandbox
        .read(CLOCK)
        .replace("(s) => s.skew >= -5", "(s) => s.skew >= -10");
    sandbox.write(CLOCK, &loosened);
    let run = sandbox.earnest(&["verify", "--base", "HEAD", "--json"]);
    let report = run.json();
    let changes: Vec<&Value> = report["ruleChanges"]
        .as_array()
        .unwrap_or_else(|| panic!("the report has `ruleChanges`\n{run}"))
        .iter()
        .collect();
    assert_eq!(
        changes.len(),
        1,
        "one change of `close`, not one per setup\n{run}"
    );
    assert_eq!(
        changes
            .first()
            .map(|change| (&change["id"], &change["model"])),
        Some((&json!("close"), &json!("Clock"))),
        "the change of the model's rule\n{run}"
    );
}
