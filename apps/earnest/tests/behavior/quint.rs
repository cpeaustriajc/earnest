use std::fs;

use crate::support::{Run, Sandbox, model, models, path_under, rule};

struct Sample {
    module: &'static str,
    states: u64,
    status: &'static str,
}

const AS_TLC_FOUND: [Sample; 4] = [
    Sample {
        module: "counter",
        states: 12,
        status: "fail",
    },
    Sample {
        module: "mixed",
        states: 10,
        status: "fail",
    },
    Sample {
        module: "starts",
        states: 14,
        status: "pass",
    },
    Sample {
        module: "dead",
        states: 4,
        status: "pass",
    },
];

fn with_compiled(modules: &[&str]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for module in modules {
        let source = crate::support::repository()
            .join("tasks/quint/models")
            .join(format!("{module}.json"));
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", source.display()));
        sandbox.write(&format!("models/{module}.json"), &text);
    }
    sandbox
}

fn verify(sandbox: &Sandbox, module: &str, more: &[&str]) -> Run {
    let file = format!("models/{module}.json");
    let mut arguments = vec!["verify", file.as_str()];
    arguments.extend_from_slice(more);
    sandbox.earnest(&arguments)
}

#[test]
fn the_compiled_counter_has_twelve_states_with_in_range_holding_and_too_big_broken() {
    let sandbox = with_compiled(&["counter"]);
    let run = verify(&sandbox, "counter", &["--json"]);
    run.exits(1);
    let report = run.json();
    let counter = model(&report, "counter");
    assert_eq!(counter["states"], 12, "TLC finds 12 distinct states\n{run}");
    assert_eq!(
        rule(counter, "inRange")["verdict"],
        "holds",
        "TLC finds no violation of inRange\n{run}"
    );
    assert_eq!(
        rule(counter, "tooBig")["verdict"],
        "broken",
        "TLC finds a violation of tooBig\n{run}"
    );
}

#[test]
fn the_shortest_path_to_a_broken_invariant_names_each_quint_action_and_the_value_it_chose() {
    let sandbox = with_compiled(&["counter"]);
    let run = verify(&sandbox, "counter", &[]);
    run.exits(1);
    let path = path_under(&run.detail("counter"), &["broken", "tooBig"]);
    assert_eq!(
        path.actions(),
        ["toggle", "bump", "bump"],
        "the bump needs the counter on, and two bumps reach 4\n{run}"
    );
    assert!(
        path.steps.iter().skip(1).all(|step| step.contains("by = ")),
        "each bump shows the value its nondet chose\n{run}"
    );
}

#[test]
fn each_sample_gives_the_state_count_and_the_verdict_tlc_gives() {
    let sandbox = with_compiled(&AS_TLC_FOUND.map(|sample| sample.module));
    for sample in &AS_TLC_FOUND {
        let run = verify(&sandbox, sample.module, &["--json"]);
        let report = run.json();
        let found = model(&report, sample.module);
        assert_eq!(
            found["states"], sample.states,
            "{} has the state count TLC finds\n{run}",
            sample.module
        );
        assert_eq!(
            found["status"], sample.status,
            "{} has the verdict TLC gives\n{run}",
            sample.module
        );
    }
}

#[test]
fn a_state_with_no_next_action_is_not_reported_stuck() {
    let sandbox = with_compiled(&["dead"]);
    let run = verify(&sandbox, "dead", &[]);
    run.exits(0);
    assert!(
        !run.stdout.contains("stuck"),
        "Quint's TLC runs do not check deadlock, so a = 3 is no finding\n{run}"
    );
}

#[test]
fn a_record_variable_is_read_and_its_unbounded_count_runs_into_the_state_limit() {
    let sandbox = with_compiled(&["record"]);
    let run = verify(&sandbox, "record", &["--max-states", "100"]);
    run.exits(1);
    assert!(
        run.stdout.starts_with("unfinished"),
        "r.n grows without end, so the search stops at the limit\n{run}"
    );
    assert!(
        run.stdout.contains("state limit of 100"),
        "the line says which limit stopped it\n{run}"
    );
    assert!(
        !run.stdout.contains("`var r`"),
        "slice 2 reads a record variable instead of refusing it\n{run}"
    );
    let report = verify(&sandbox, "record", &["--max-states", "100", "--json"]).json();
    let record = model(&report, "record");
    assert_eq!(
        record["states"], 100,
        "r.n takes 0, 1, 2, ..., one state each, so the search meets the limit of 100\n{run}"
    );
    assert!(
        record["findings"].as_array().is_some_and(Vec::is_empty),
        "r.n >= 0 holds in every state reached, so there is no finding\n{run}"
    );
}

#[test]
fn an_operator_still_refused_makes_the_file_unreadable_and_names_the_construct() {
    let sandbox = Sandbox::empty();
    let source = crate::support::repository().join("crates/earnest_quint/fixtures/refused.json");
    let text = fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", source.display()))
        .replace("\"powerset\"", "\"flatten\"");
    sandbox.write("models/refused.json", &text);
    let run = verify(&sandbox, "refused", &[]);
    run.exits(1);
    assert!(
        run.stdout.starts_with("unreadable"),
        "a construct the reader does not take is never skipped\n{run}"
    );
    assert!(
        run.stdout.contains("`chooseSome`"),
        "the problem names the construct\n{run}"
    );
    assert!(
        run.stdout.contains("in the definition `step`"),
        "the problem names the definition the construct is in\n{run}"
    );
}

#[test]
fn a_named_json_file_that_is_not_compiled_quint_is_not_read() {
    let sandbox = Sandbox::empty();
    sandbox.write("settings.json", "{\"a\": 1}\n");
    let run = sandbox.earnest(&["verify", "settings.json"]);
    run.exits(1);
    assert!(
        run.stdout.contains("does not know how to read it"),
        "only JSON that `quint compile` wrote is read\n{run}"
    );
}

#[test]
fn a_folder_search_does_not_pick_up_compiled_quint() {
    let sandbox = with_compiled(&["counter"]);
    let run = sandbox.earnest(&["verify", "models", "--json"]);
    run.exits(0);
    assert!(
        models(&run.json()).is_empty(),
        "a compiled module is checked only when it is named\n{run}"
    );
}

#[test]
fn the_rule_option_checks_one_invariant_of_a_compiled_module() {
    let sandbox = with_compiled(&["counter"]);
    let run = verify(&sandbox, "counter", &["--rule", "inRange", "--json"]);
    run.exits(0);
    let report = run.json();
    let rules = model(&report, "counter")["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        rules.len(),
        1,
        "only the asked-for invariant is checked\n{run}"
    );
    assert_eq!(rules[0]["id"], "inRange", "the one rule is inRange\n{run}");
}

#[test]
fn a_second_run_of_an_unchanged_compiled_module_comes_from_the_cache() {
    let sandbox = with_compiled(&["counter"]);
    verify(&sandbox, "counter", &["--json"]).exits(1);
    let run = verify(&sandbox, "counter", &["--json"]);
    run.exits(1);
    assert_eq!(
        model(&run.json(), "counter")["cached"],
        true,
        "the cache keys on the file's text as it does for every model\n{run}"
    );
}
