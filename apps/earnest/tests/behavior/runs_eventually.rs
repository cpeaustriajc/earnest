use serde_json::{Value, json};

use crate::support::{Sandbox, finding, model, models, named, rule, without_timings};

const EVENTUALLY: [&str; 7] = [
    "cart.model.ts",
    "cart_broken.model.ts",
    "kettle.model.ts",
    "ticket.model.ts",
    "worker.model.ts",
    "lazy_worker.model.ts",
    "dispatch.model.ts",
];

const STAYS: &str = "nothing changes the state from here, so the run can stay in it forever";

const CART_BROKEN_DETAIL: [&str; 7] = [
    "broken  offered  \"Once approved, the cart offers checkout\"",
    "start  approved = false, offers = false, paid = false",
    "1. approve  approved: false -> true",
    "2. cancel  approved: true -> false",
    "3. approve  approved: false -> true",
    "end    approved = true, offers = false, paid = false",
    "why    steps 2 to 3 can repeat forever",
];

const KETTLE_DETAIL: [&str; 7] = [
    "broken  switchesOff  \"Once the water boils, the kettle switches off\"",
    "start  heat = 0, on = false",
    "1. press  on: false -> true",
    "2. warm  heat: 0 -> 1",
    "3. warm  heat: 1 -> 2",
    "end    heat = 2, on = true",
    "why    nothing changes the state from here, so the run can stay in it forever",
];

const TICKET_DETAIL: [&str; 5] = [
    "broken  fixed  \"Every ticket is fixed\"",
    "start  stage = \"open\"",
    "1. decline  stage: \"open\" -> \"closed\"",
    "end    stage = \"closed\"",
    "why    nothing changes the state from here, so the run can stay in it forever",
];

const LAZY_DETAIL: [&str; 6] = [
    "broken  finishes  \"The job gets done\"",
    "start  busy = false, done = false",
    "1. fidget  busy: false -> true",
    "2. fidget  busy: true -> false",
    "end    busy = false, done = false",
    "why    steps 1 to 2 can repeat forever",
];

const DISPATCH_DETAIL: [&str; 5] = [
    "broken  toTheBack  \"Every call is sent to the back desk\"",
    "start  desk = null, tick = false",
    "1. route  to = \"front\"  desk: null -> \"front\"",
    "end    desk = \"front\", tick = false",
    "why    nothing changes the state from here, so the run can stay in it forever",
];

fn verdicts(found: &Value) -> Vec<(String, String)> {
    found["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("the model has a `rules` array: {found}"))
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

fn cart(approved: bool, offers: bool) -> Value {
    named(&[
        ("approved", json!(approved)),
        ("offers", json!(offers)),
        ("paid", json!(false)),
    ])
}

fn step(action: &str, label: &str, field: &str, before: bool, state: Value) -> Value {
    json!({
        "action": action,
        "label": label,
        "input": [],
        "changes": [{ "field": field, "before": before, "after": !before }],
        "state": state,
    })
}

#[test]
fn each_model_whose_rules_about_runs_hold_passes_and_each_with_one_broken_fails() {
    let sandbox = Sandbox::with_runs(&EVENTUALLY);
    let run = sandbox.earnest(&["verify"]);
    run.exits(1);
    for (name, status, states) in [
        ("Cart", "pass", "4 states"),
        ("CartBroken", "fail", "4 states"),
        ("Kettle", "fail", "4 states"),
        ("Ticket", "fail", "3 states"),
        ("Worker", "pass", "4 states"),
        ("LazyWorker", "fail", "4 states"),
        ("Dispatch", "fail", "6 states"),
    ] {
        assert_eq!(
            run.model_line(name).get(..3),
            Some([status, name, states].map(str::to_owned).as_slice()),
            "the result line of {name}\n{run}"
        );
    }
    assert_eq!(run.summary_total(), 7, "every model is a run\n{run}");
    assert_eq!(run.summary_count("pass"), 2, "Cart and Worker pass\n{run}");
    assert_eq!(run.summary_count("fail"), 5, "the other five fail\n{run}");
    for (name, detail) in [
        ("CartBroken", CART_BROKEN_DETAIL.as_slice()),
        ("Kettle", KETTLE_DETAIL.as_slice()),
        ("Ticket", TICKET_DETAIL.as_slice()),
        ("LazyWorker", LAZY_DETAIL.as_slice()),
        ("Dispatch", DISPATCH_DETAIL.as_slice()),
    ] {
        assert_eq!(
            run.detail(name),
            detail,
            "{name}: the path into the run that never reaches, and why it goes on forever\n{run}"
        );
    }
    for name in ["Cart", "Worker"] {
        assert_eq!(
            run.detail(name),
            Vec::<String>::new(),
            "{name} passes and prints no detail\n{run}"
        );
    }
}

#[test]
fn models_whose_rules_about_runs_hold_exit_zero() {
    let sandbox = Sandbox::with_runs(&["cart.model.ts", "worker.model.ts"]);
    let run = sandbox.earnest(&["verify"]);
    run.exits(0);
    assert_eq!(run.summary_count("pass"), 2, "both pass\n{run}");
}

#[test]
fn a_loop_that_only_puts_off_a_fair_action_breaks_no_rule() {
    let sandbox = Sandbox::with_runs(&["worker.model.ts", "lazy_worker.model.ts"]);
    let report = sandbox.earnest(&["verify", "--json"]).json();
    assert_eq!(
        verdicts(model(&report, "Worker")),
        listed(&[("finishes", "holds")]),
        "`finish` is fair and possible in both states of the `fidget` loop"
    );
    assert_eq!(
        verdicts(model(&report, "LazyWorker")),
        listed(&[("finishes", "broken")]),
        "without `fair`, the same loop is a run that never finishes"
    );
}

#[test]
fn the_json_gives_the_kind_the_reason_and_where_the_loop_starts() {
    let sandbox = Sandbox::with_runs(&EVENTUALLY);
    let run = sandbox.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    let broken = rule(model(&report, "CartBroken"), "offered");
    assert_eq!(
        broken,
        &json!({
            "id": "offered",
            "says": "Once approved, the cart offers checkout",
            "verdict": "broken",
            "path": {
                "start": cart(false, false),
                "steps": [
                    step("approve", "the shop approves the cart", "approved", false, cart(true, false)),
                    step("cancel", "the shop takes the approval back", "approved", true, cart(false, false)),
                    step("approve", "the shop approves the cart", "approved", false, cart(true, false)),
                ],
            },
            "kind": "eventually",
            "reason": "steps 2 to 3 can repeat forever",
            "repeatsFrom": 1,
        }),
        "the path into the loop and once round it, with the 0-based start of the loop\n{run}"
    );
    let holds = rule(model(&report, "CartBroken"), "approvedSoon");
    assert_eq!(
        holds,
        &json!({
            "id": "approvedSoon",
            "says": "The cart is approved",
            "verdict": "holds",
            "path": null,
            "kind": "eventually",
        }),
        "a rule that holds has its kind and no reason\n{run}"
    );
    let lazy = rule(model(&report, "LazyWorker"), "finishes");
    assert_eq!(
        lazy["repeatsFrom"], 0,
        "the loop starts at the start state\n{run}"
    );
    for (name, id) in [
        ("Kettle", "switchesOff"),
        ("Ticket", "fixed"),
        ("Dispatch", "toTheBack"),
    ] {
        let stays = rule(model(&report, name), id);
        assert_eq!(stays["reason"], STAYS, "{name}: the run stays\n{run}");
        assert!(
            stays.get("repeatsFrom").is_none(),
            "{name}: a run that stays has no loop: {stays}"
        );
    }
    let route = &rule(model(&report, "Dispatch"), "toTheBack")["path"]["steps"][0];
    assert_eq!(
        route["input"],
        named(&[("to", json!("front"))]),
        "the step of the fair action takes its input\n{run}"
    );
}

#[test]
fn rules_about_runs_come_after_the_rules_and_before_the_witnesses() {
    let sandbox = Sandbox::with_runs(&["clock.model.ts"]);
    let report = sandbox.earnest(&["verify", "--json"]).json();
    let kinds: Vec<(&str, Option<&str>)> = model(&report, "Clock")["rules"]
        .as_array()
        .expect("the model has its rules")
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default(),
                each["kind"].as_str(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("close", None),
            ("syncs", Some("eventually")),
            ("goesRound", Some("witness")),
        ],
        "the plain rule, the rule about runs, the witness"
    );
}

#[test]
fn checking_only_one_rule_about_runs_by_its_id() {
    let sandbox = Sandbox::with_runs(&EVENTUALLY);
    let run = sandbox.earnest(&["verify", "--rule", "switchesOff"]);
    run.exits(1);
    assert_eq!(run.model_lines().len(), 1, "only the kettle\n{run}");
    assert_eq!(run.detail("Kettle"), KETTLE_DETAIL, "only that rule\n{run}");
    let holds = sandbox.earnest(&["verify", "--rule", "boils", "--json"]);
    holds.exits(0);
    let report = holds.json();
    assert_eq!(models(&report).len(), 1, "only the kettle\n{holds}");
    assert_eq!(
        verdicts(model(&report, "Kettle")),
        listed(&[("boils", "holds")]),
        "only the rule asked for\n{holds}"
    );
    let shared = sandbox.earnest(&["verify", "--rule", "offered"]);
    shared.exits(2);
    assert!(
        shared.stderr.contains("`Cart`, `CartBroken`") && shared.stderr.contains("--model"),
        "two models have `offered`, so `--model` must name one\n{shared}"
    );
    let named_model = sandbox.earnest(&["verify", "--rule", "offered", "--model", "CartBroken"]);
    named_model.exits(1);
    assert_eq!(
        named_model.detail("CartBroken"),
        CART_BROKEN_DETAIL,
        "the rule of the model named\n{named_model}"
    );
}

#[test]
fn a_cached_run_prints_the_same_loop_and_why_line() {
    let sandbox = Sandbox::with_runs(&EVENTUALLY);
    sandbox.earnest(&["verify"]).exits(1);
    let second = sandbox.earnest(&["verify"]);
    second.exits(1);
    assert_eq!(
        second.model_line("CartBroken").last().map(String::as_str),
        Some("(cached)"),
        "the second run comes from the cache\n{second}"
    );
    assert_eq!(
        second.detail("CartBroken"),
        CART_BROKEN_DETAIL,
        "the cached run prints the same detail\n{second}"
    );
    let fresh = sandbox.earnest(&["verify", "--json", "--no-cache"]).json();
    let cached = sandbox.earnest(&["verify", "--json"]).json();
    assert_eq!(
        without_timings(&cached),
        without_timings(&fresh),
        "the cached JSON keeps `kind`, `reason` and `repeatsFrom`"
    );
}

#[test]
fn the_report_shows_a_broken_rule_about_runs_with_its_why_line() {
    let sandbox = Sandbox::with_runs(&EVENTUALLY);
    let run = sandbox.earnest(&["report"]);
    for line in CART_BROKEN_DETAIL.iter().chain(&KETTLE_DETAIL) {
        assert!(
            run.stdout.lines().any(|printed| printed.trim() == *line),
            "the report has the line `{line}`\n{run}"
        );
    }
}

#[test]
fn a_search_stopped_by_the_state_limit_judges_no_rule_about_runs() {
    let sandbox = Sandbox::with_runs(&["cart_broken.model.ts"]);
    let run = sandbox.earnest(&["verify", "--json", "--max-states", "2"]);
    run.exits(1);
    let found = model(&run.json(), "CartBroken").clone();
    assert_eq!(found["status"], "unfinished", "the search stopped\n{run}");
    assert_eq!(
        verdicts(&found),
        listed(&[("offered", "unknown"), ("approvedSoon", "unknown")]),
        "neither rule is judged on part of the states, not even the one that holds\n{run}"
    );
}

#[test]
fn a_rule_about_runs_that_faults_is_unknown_with_a_fault_naming_it() {
    let sandbox = Sandbox::empty();
    sandbox.write(
        "count.model.ts",
        "import { m } from \"@earnestjs/model\";\n\
         export const Count = m.model(\"Count\", { n: m.int(0, 2).initial(2) });\n\
         export const down = m.action(Count, \"the count goes down\", {\n\
         \twhen: (s) => s.n > 0,\n\
         \tchanges: (s) => ({ n: s.n - 1 }),\n\
         });\n\
         export const zero = m.end(Count, \"The count is zero\", (s) => s.n === 0);\n\
         export const halves = m.eventually(Count, \"The count halves cleanly\", {\n\
         \treaches: (s) => m.div(2, s.n) === 2,\n\
         });\n\
         export const low = m.eventually(Count, \"The count gets low\", {\n\
         \treaches: (s) => s.n <= 1,\n\
         });\n",
    );
    let run = sandbox.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, "Count");
    assert_eq!(
        verdicts(found),
        listed(&[("halves", "unknown"), ("low", "holds")]),
        "`m.div(2, 0)` has no value at `n = 0`, and the other rule is judged as usual\n{run}"
    );
    let fault = finding(found, "fault");
    assert_eq!(
        fault["message"],
        "the rule `halves` takes a division with a negative number or a divisor below 1 in this state",
        "the fault names the rule\n{run}"
    );
    assert_eq!(
        fault["path"]["steps"].as_array().map(Vec::len),
        Some(2),
        "the path to `n = 0`\n{run}"
    );
}

#[test]
fn a_state_out_of_range_leaves_a_rule_about_runs_unknown() {
    let sandbox = Sandbox::empty();
    sandbox.write(
        "climb.model.ts",
        "import { m } from \"@earnestjs/model\";\n\
         export const Climb = m.model(\"Climb\", { n: m.int(0, 2).initial(0) });\n\
         export const up = m.action(Climb, \"the count goes up\", {\n\
         \tchanges: (s) => ({ n: s.n + 1 }),\n\
         });\n\
         export const top = m.eventually(Climb, \"The count reaches the top\", {\n\
         \treaches: (s) => s.n === 2,\n\
         });\n",
    );
    let run = sandbox.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, "Climb");
    assert_eq!(
        verdicts(found),
        listed(&[("top", "unknown")]),
        "`n` reaches 2, but the search stopped at `n = 3`\n{run}"
    );
    assert_eq!(
        finding(found, "out-of-range")["message"],
        "the field `n` leaves the values it declares",
        "the state out of range is the finding\n{run}"
    );
}

#[test]
fn the_tla_export_of_a_model_with_rules_about_runs_has_its_fairness_and_properties() {
    let sandbox = Sandbox::with_runs(&["dispatch.model.ts", "panel.model.ts"]);
    sandbox.earnest(&["tla", "--out", "tla", "runs"]).exits(0);
    let module = sandbox.read("tla/Dispatch.tla");
    for line in [
        "    Init /\\ [][Next]_vars /\\ WF_vars(Next) /\\ WF_vars(Action_route)",
        "    <>((desk # <<>>))",
        "    <>((desk = <<\"back\">>))",
    ] {
        assert!(
            module.lines().any(|written| written == line),
            "the module has the line `{line}`\n{module}"
        );
    }
    assert_eq!(
        sandbox.read("tla/Dispatch.cfg"),
        "SPECIFICATION Spec\nCHECK_DEADLOCK FALSE\nPROPERTY Eventually_routed\nPROPERTY Eventually_toTheBack\n",
        "the specification with its fairness, then one property per rule"
    );
    let panel = sandbox.read("tla/Panel.cfg");
    assert!(
        panel.starts_with("INIT Init\nNEXT Next\n") && !panel.contains("PROPERTY"),
        "a model without rules about runs is exported as before\n{panel}"
    );
    assert!(
        !sandbox.read("tla/Panel.tla").contains("Spec =="),
        "a model without rules about runs has no `Spec`"
    );
}
