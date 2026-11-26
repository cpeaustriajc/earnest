use std::fs;

use serde_json::{Value, json};

use crate::support::{Run, Sandbox, actions, models, repository, rule, without};

const DIFFERS_BY_LANGUAGE: [&str; 4] = ["file", "line", "milliseconds", "cached"];
const PAIRS: [&str; 3] = ["gate", "kettle", "clock"];

fn runs_fixture(file: &str) -> String {
    let folder = if file.starts_with("clock") {
        "setups"
    } else {
        "runs"
    };
    let path = repository()
        .join("tasks/conformance")
        .join(folder)
        .join(file);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

fn verified(file: &str, text: &str, options: &[&str]) -> Run {
    let sandbox = Sandbox::empty();
    sandbox.write(&format!("models/{file}"), text);
    let path = format!("models/{file}");
    let mut arguments = vec!["verify", path.as_str(), "--no-cache"];
    arguments.extend(options);
    sandbox.earnest(&arguments)
}

fn without_timings(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                [head @ .., _, "ms"] => head.join(" "),
                _ => words.join(" "),
            }
        })
        .collect()
}

fn agree(name: &str, rust: &str, typescript: &str) -> Value {
    let rust_file = format!("{name}.rs");
    let typescript_file = format!("{name}.model.ts");
    let twin = verified(&rust_file, rust, &["--json"]);
    let original = verified(&typescript_file, typescript, &["--json"]);
    assert_eq!(twin.code, original.code, "{name}: both exit alike\n{twin}");
    let report = twin.json();
    assert_eq!(
        without(&report, &DIFFERS_BY_LANGUAGE),
        without(&original.json(), &DIFFERS_BY_LANGUAGE),
        "{rust_file} and {typescript_file} give the same JSON"
    );
    let twin_text = verified(&rust_file, rust, &[]);
    let original_text = verified(&typescript_file, typescript, &[]);
    assert_eq!(
        without_timings(&twin_text.stdout),
        without_timings(&original_text.stdout),
        "{rust_file} and {typescript_file} print the same text"
    );
    report
}

fn only(report: &Value) -> &Value {
    let found = models(report);
    assert_eq!(found.len(), 1, "one model: {report}");
    &found[0]
}

#[test]
fn a_rust_model_with_rules_about_runs_and_witnesses_reports_as_its_typescript_twin_does() {
    for name in PAIRS {
        agree(
            name,
            &runs_fixture(&format!("{name}.rs")),
            &runs_fixture(&format!("{name}.model.ts")),
        );
    }
}

#[test]
fn a_rust_witness_holds_or_breaks_with_the_path_played_and_the_reason() {
    let report = agree(
        "gate",
        &runs_fixture("gate.rs"),
        &runs_fixture("gate.model.ts"),
    );
    let gate = only(&report);
    assert_eq!(
        gate["states"], 6,
        "three fields, the light blinks only unpaid"
    );
    let held = rule(gate, "pay_then_push");
    assert_eq!(
        held["kind"], "witness",
        "a witness is listed with the rules"
    );
    assert_eq!(held["verdict"], "holds", "pay, then push opens the gate");
    let broken = rule(gate, "push_first");
    assert_eq!(broken["verdict"], "broken", "push needs a payment first");
    assert_eq!(
        broken["reason"], "step 1, `push`, can't happen here",
        "the step that can't happen"
    );
    assert_eq!(
        actions(&broken["path"]),
        Vec::<String>::new(),
        "no step is played"
    );
    let report = agree(
        "kettle",
        &runs_fixture("kettle.rs"),
        &runs_fixture("kettle.model.ts"),
    );
    let kettle = only(&report);
    assert_eq!(
        rule(kettle, "starts_cold")["verdict"],
        "holds",
        "a witness without steps"
    );
    let false_at_the_end = rule(kettle, "hot_at_once");
    assert_eq!(false_at_the_end["verdict"], "broken", "one heat is warm");
    assert_eq!(
        false_at_the_end["reason"], "what the witness expects is false here",
        "the condition is false after the steps"
    );
    assert_eq!(
        actions(&false_at_the_end["path"]),
        ["heat"],
        "the path played"
    );
}

#[test]
fn a_rust_rule_about_runs_holds_breaks_by_a_loop_breaks_by_a_stop_and_holds_by_fairness() {
    let report = agree(
        "gate",
        &runs_fixture("gate.rs"),
        &runs_fixture("gate.model.ts"),
    );
    let gate = only(&report);
    for id in ["opens", "opens_once_paid"] {
        let held = rule(gate, id);
        assert_eq!(held["kind"], "eventually", "{id} is a rule about runs");
        assert_eq!(held["verdict"], "holds", "{id} holds");
    }
    let looped = rule(gate, "lit_and_paid");
    assert_eq!(looped["verdict"], "broken", "pay, push, enter repeats");
    assert_eq!(
        actions(&looped["path"]),
        ["pay", "push", "enter"],
        "the loop"
    );
    assert_eq!(looped["repeatsFrom"], 0, "the loop starts at the start");
    assert_eq!(
        looped["reason"], "steps 1 to 3 can repeat forever",
        "the steps that repeat"
    );

    let report = agree(
        "kettle",
        &runs_fixture("kettle.rs"),
        &runs_fixture("kettle.model.ts"),
    );
    let kettle = only(&report);
    let stopped = rule(kettle, "boils");
    assert_eq!(
        stopped["verdict"], "broken",
        "an unplugged kettle stops cold"
    );
    assert_eq!(actions(&stopped["path"]), ["unplug"], "the run that stops");
    assert_eq!(
        stopped["reason"], "nothing changes the state from here, so the run can stay in it forever",
        "the run stops"
    );
    assert_eq!(
        rule(kettle, "settles")["verdict"],
        "holds",
        "every run stops hot or off"
    );

    let unfair_rust = runs_fixture("gate.rs").replacen(", fair)", ")", 1);
    let unfair_typescript = runs_fixture("gate.model.ts").replacen("\tfair: true,\n", "", 1);
    let report = agree("gate", &unfair_rust, &unfair_typescript);
    let gate = only(&report);
    let put_off = rule(gate, "opens");
    assert_eq!(
        put_off["verdict"], "broken",
        "without `fair`, the light can blink forever"
    );
    assert_eq!(
        actions(&put_off["path"]),
        ["blink", "blink"],
        "the loop that puts paying off"
    );
    assert_eq!(
        rule(gate, "opens_once_paid")["verdict"],
        "holds",
        "a paid gate opens with or without `fair`"
    );
}

#[test]
fn a_rust_model_with_two_setups_is_checked_in_each_and_plays_a_witness_in_its_setup() {
    let report = agree(
        "clock",
        &runs_fixture("clock.rs"),
        &runs_fixture("clock.model.ts"),
    );
    let found = models(&report);
    let setups: Vec<&Value> = found.iter().map(|model| &model["setup"]).collect();
    assert_eq!(
        setups,
        [
            &Value::Null,
            &json!({ "id": "behind", "says": "The clock runs two seconds behind" }),
            &json!({ "id": "synced_already", "says": "The clock starts synced" }),
        ],
        "the model as declared, then each setup in file order"
    );
    let states: Vec<&Value> = found.iter().map(|model| &model["states"]).collect();
    assert_eq!(states, [6, 6, 1], "five skews and the synced clock, or one");
    let witnesses = |model: &Value| -> Vec<(String, String)> {
        model["rules"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|rule| rule["kind"] == "witness")
            .map(|rule| {
                (
                    rule["id"].as_str().unwrap_or_default().to_owned(),
                    rule["verdict"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    };
    assert_eq!(
        witnesses(&found[0]),
        [("drifts_ahead".to_owned(), "holds".to_owned())],
        "the witness without a setup"
    );
    assert_eq!(
        witnesses(&found[1]),
        [("catches_up".to_owned(), "holds".to_owned())],
        "the witness of the setup"
    );
    assert_eq!(witnesses(&found[2]), [], "no witness names this setup");
    let syncs: Vec<&Value> = found
        .iter()
        .map(|model| &rule(model, "syncs")["verdict"])
        .collect();
    assert_eq!(
        syncs,
        ["broken", "broken", "holds"],
        "drifting forever never syncs, and a synced start is synced"
    );
    let text = verified("clock.rs", &runs_fixture("clock.rs"), &[]);
    let heads: Vec<String> = without_timings(&text.stdout)
        .into_iter()
        .filter(|line| !line.starts_with(' ') && line.contains("states"))
        .collect();
    assert_eq!(
        heads,
        [
            "fail Clock 6 states",
            "fail Clock (behind) 6 states",
            "pass Clock (synced_already) 1 states",
        ],
        "a result line for each setup"
    );
}
