use serde_json::{Value, json};

use crate::support::{Sandbox, model, models, named, rule, without_timings};

const RUNS: [&str; 2] = ["lift.model.ts", "panel.model.ts"];

const CLIMBS_DETAIL: [&str; 5] = [
    "broken  climbs  \"Two floors up reaches the top floor\"",
    "start  floor = 1, doors = false",
    "1. up  floor: 1 -> 2",
    "end    floor = 2, doors = false",
    "why    step 2, `up`, can't happen here",
];

fn lift_detail() -> Vec<&'static str> {
    let mut lines = CLIMBS_DETAIL.to_vec();
    lines.extend([
        "broken  arrives  \"One floor up reaches the first floor\"",
        "start  floor = 1, doors = false",
        "1. up  floor: 1 -> 2",
        "end    floor = 2, doors = false",
        "why    what the witness expects is false here",
        "broken  grounded  \"The lift starts on the ground floor\"",
        "start  floor = 1, doors = false",
        "why    what the witness expects is false here",
    ]);
    lines
}

fn floor_one_up() -> Value {
    json!({
        "start": named(&[("floor", json!(1)), ("doors", json!(false))]),
        "steps": [{
            "action": "up",
            "label": "the lift goes up a floor",
            "input": [],
            "changes": [{ "field": "floor", "before": 1, "after": 2 }],
            "state": named(&[("floor", json!(2)), ("doors", json!(false))]),
        }],
    })
}

fn verdicts(found: &Value) -> Vec<(String, String, Option<String>)> {
    found["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("the model has a `rules` array: {found}"))
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
                each["kind"].as_str().map(str::to_owned),
            )
        })
        .collect()
}

fn listed(rows: &[(&str, &str, Option<&str>)]) -> Vec<(String, String, Option<String>)> {
    rows.iter()
        .map(|(id, verdict, kind)| {
            (
                (*id).to_owned(),
                (*verdict).to_owned(),
                kind.map(str::to_owned),
            )
        })
        .collect()
}

#[test]
fn a_model_whose_witnesses_hold_passes_and_one_with_a_broken_witness_fails() {
    let sandbox = Sandbox::with_runs(&RUNS);
    let run = sandbox.earnest(&["verify"]);
    run.exits(1);
    assert_eq!(
        run.model_line("Lift").get(..3),
        Some(["fail", "Lift", "6 states"].map(str::to_owned).as_slice()),
        "the lift fails\n{run}"
    );
    assert_eq!(
        run.model_line("Panel").get(..3),
        Some(["pass", "Panel", "14 states"].map(str::to_owned).as_slice()),
        "the panel passes\n{run}"
    );
    assert_eq!(run.summary_count("pass"), 1, "models that pass\n{run}");
    assert_eq!(run.summary_count("fail"), 1, "models that fail\n{run}");
    assert_eq!(
        run.detail("Lift"),
        lift_detail(),
        "each broken witness, the path played and why it broke\n{run}"
    );
    assert_eq!(
        run.detail("Panel"),
        Vec::<String>::new(),
        "a passing model prints no detail\n{run}"
    );
}

#[test]
fn a_model_with_only_witnesses_that_hold_exits_zero() {
    let sandbox = Sandbox::with_runs(&["panel.model.ts"]);
    let run = sandbox.earnest(&["verify"]);
    run.exits(0);
    assert_eq!(run.summary_count("pass"), 1, "the panel passes\n{run}");
}

#[test]
fn the_json_lists_each_witness_after_the_rules_with_its_kind_and_reason() {
    let sandbox = Sandbox::with_runs(&RUNS);
    let run = sandbox.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    let panel = model(&report, "Panel");
    assert_eq!(panel["status"], "pass", "the panel passes\n{run}");
    assert_eq!(panel["states"], 14, "the panel's states\n{run}");
    assert_eq!(
        verdicts(panel),
        listed(&[
            ("paged", "holds", None),
            ("reopens", "holds", Some("witness")),
            ("startsClosed", "holds", Some("witness")),
        ]),
        "the rule, then the witnesses, which hold\n{run}"
    );
    let paged = rule(panel, "paged");
    assert!(
        paged.get("kind").is_none() && paged.get("reason").is_none(),
        "a plain rule has no `kind` and no `reason`: {paged}"
    );
    let reopens = rule(panel, "reopens");
    assert_eq!(
        reopens["path"],
        Value::Null,
        "a witness that holds has no path"
    );
    assert!(
        reopens.get("reason").is_none(),
        "a witness that holds has no `reason`: {reopens}"
    );
    assert_eq!(
        reopens["says"], "Open, show more, close, open: the panel opens on the first page",
        "the sentence"
    );

    let lift = model(&report, "Lift");
    assert_eq!(lift["status"], "fail", "the lift fails\n{run}");
    assert_eq!(lift["states"], 6, "the lift's states\n{run}");
    assert_eq!(lift["findings"], json!([]), "no finding\n{run}");
    assert_eq!(
        verdicts(lift),
        listed(&[
            ("rides", "holds", Some("witness")),
            ("climbs", "broken", Some("witness")),
            ("arrives", "broken", Some("witness")),
            ("grounded", "broken", Some("witness")),
        ]),
        "the witnesses in file order\n{run}"
    );
    for (id, reason, path) in [
        ("climbs", "step 2, `up`, can't happen here", floor_one_up()),
        (
            "arrives",
            "what the witness expects is false here",
            floor_one_up(),
        ),
        (
            "grounded",
            "what the witness expects is false here",
            json!({
                "start": named(&[("floor", json!(1)), ("doors", json!(false))]),
                "steps": [],
            }),
        ),
    ] {
        let found = rule(lift, id);
        assert_eq!(found["reason"], reason, "why {id} is broken\n{run}");
        assert_eq!(
            found["path"], path,
            "{id}: the path played from the start with floor 1\n{run}"
        );
    }
}

#[test]
fn a_step_with_an_input_is_played_with_the_value_written() {
    let sandbox = Sandbox::with_runs(&["panel.model.ts"]);
    let broken = sandbox
        .read("runs/panel.model.ts")
        .replace("s.open && s.page === 1,", "s.open && s.page === 2,");
    sandbox.write("runs/panel.model.ts", &broken);
    let run = sandbox.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    let reopens = rule(model(&report, "Panel"), "reopens");
    assert_eq!(
        reopens["reason"], "what the witness expects is false here",
        "the panel opens on page 1\n{run}"
    );
    let steps = reopens["path"]["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("the path has steps: {reopens}"));
    let played: Vec<(&str, &Value)> = steps
        .iter()
        .map(|step| (step["action"].as_str().unwrap_or_default(), &step["input"]))
        .collect();
    assert_eq!(
        played,
        [
            ("open", &json!([])),
            ("more", &named(&[("by", json!("mouse"))])),
            ("close", &json!([])),
            ("open", &json!([])),
        ],
        "every step in order, `more` with the mouse\n{run}"
    );
    assert_eq!(
        steps.last().map(|step| &step["state"]),
        Some(&named(&[
            ("open", json!(true)),
            ("page", json!(1)),
            ("last", json!("mouse")),
        ])),
        "the state the witness ends in\n{run}"
    );
}

#[test]
fn checking_only_one_witness_by_its_id() {
    let sandbox = Sandbox::with_runs(&RUNS);
    let run = sandbox.earnest(&["verify", "--rule", "climbs"]);
    run.exits(1);
    assert_eq!(
        run.model_lines().len(),
        1,
        "only the model with the witness\n{run}"
    );
    assert_eq!(
        run.detail("Lift"),
        CLIMBS_DETAIL,
        "only the witness asked for\n{run}"
    );
    let holds = sandbox.earnest(&["verify", "--rule", "reopens", "--json"]);
    holds.exits(0);
    let report = holds.json();
    assert_eq!(models(&report).len(), 1, "only the panel\n{holds}");
    assert_eq!(
        verdicts(model(&report, "Panel")),
        listed(&[("reopens", "holds", Some("witness"))]),
        "only the witness asked for\n{holds}"
    );
}

#[test]
fn a_cached_run_prints_the_same_result_with_the_why_line() {
    let sandbox = Sandbox::with_runs(&RUNS);
    let first = sandbox.earnest(&["verify"]);
    first.exits(1);
    let second = sandbox.earnest(&["verify"]);
    second.exits(1);
    assert_eq!(
        second.model_line("Lift").last().map(String::as_str),
        Some("(cached)"),
        "the second run comes from the cache\n{second}"
    );
    assert_eq!(
        second.detail("Lift"),
        lift_detail(),
        "the cached run prints the same detail\n{second}"
    );
    let as_json = sandbox.earnest(&["verify", "--json", "--no-cache"]).json();
    let cached = sandbox.earnest(&["verify", "--json"]).json();
    assert_eq!(
        without_timings(&cached),
        without_timings(&as_json),
        "the cached JSON is the JSON of a fresh run"
    );
}

#[test]
fn the_report_shows_a_broken_witness_with_why_it_broke() {
    let sandbox = Sandbox::with_runs(&RUNS);
    let run = sandbox.earnest(&["report"]);
    for line in lift_detail() {
        assert!(
            run.stdout.lines().any(|printed| printed.trim() == line),
            "the report has the line `{line}`\n{run}"
        );
    }
    assert!(
        run.stdout.lines().any(|line| ["fail", "Lift", "6 states"]
            .iter()
            .all(|token| line.contains(token))),
        "the report has the lift's result line\n{run}"
    );
}

#[test]
fn a_witness_is_not_written_to_tla() {
    let sandbox = Sandbox::with_runs(&["panel.model.ts"]);
    sandbox
        .earnest(&["tla", "--out", "tla", "runs/panel.model.ts"])
        .exits(0);
    let config = sandbox.read("tla/Panel.cfg");
    assert!(
        config.contains("Rule_paged"),
        "the rule is an invariant\n{config}"
    );
    let module = sandbox.read("tla/Panel.tla");
    for id in ["reopens", "startsClosed"] {
        assert!(
            !config.contains(id) && !module.contains(id),
            "the witness `{id}` is left out\n{config}\n{module}"
        );
    }
}

#[test]
fn a_start_state_past_the_state_limit_is_not_played() {
    let sandbox = Sandbox::with_runs(&["lift.model.ts"]);
    let verdicts_at = |limit: &str| {
        let run = sandbox.earnest(&["verify", "--json", "--max-states", limit]);
        run.exits(1);
        verdicts(model(&run.json(), "Lift"))
    };
    assert_eq!(
        verdicts_at("2"),
        listed(&[
            ("rides", "unknown", Some("witness")),
            ("climbs", "broken", Some("witness")),
            ("arrives", "broken", Some("witness")),
            ("grounded", "broken", Some("witness")),
        ]),
        "floor 0 and floor 1 are played, and floor 1 breaks three of them"
    );
    assert_eq!(
        verdicts_at("1"),
        listed(&[
            ("rides", "unknown", Some("witness")),
            ("climbs", "unknown", Some("witness")),
            ("arrives", "unknown", Some("witness")),
            ("grounded", "unknown", Some("witness")),
        ]),
        "only floor 0 is played, and it breaks none"
    );
}
