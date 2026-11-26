use std::fs;

use crate::support::{Run, Sandbox, finding, model, path_under, rule};

const SAMPLES: &str = "tasks/quint/models";
const FIXTURES: &str = "crates/earnest_quint/fixtures";

fn with_compiled(folder: &str, files: &[&str]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for file in files {
        let source = crate::support::repository()
            .join(folder)
            .join(format!("{file}.json"));
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", source.display()));
        sandbox.write(&format!("models/{file}.json"), &text);
    }
    sandbox
}

fn verify(sandbox: &Sandbox, file: &str, more: &[&str]) -> Run {
    let path = format!("models/{file}.json");
    let mut arguments = vec!["verify", path.as_str()];
    arguments.extend_from_slice(more);
    sandbox.earnest(&arguments)
}

#[test]
fn the_bank_has_the_fifty_five_states_tlc_finds_and_every_invariant_holds() {
    let sandbox = with_compiled(SAMPLES, &["bank"]);
    let run = verify(&sandbox, "bank", &["--json"]);
    run.exits(0);
    let report = run.json();
    let bank = model(&report, "bank");
    assert_eq!(bank["states"], 55, "TLC finds 55 distinct states\n{run}");
    for invariant in ["sumMatches", "ownersMatch", "someRich", "noOverdraft"] {
        assert_eq!(
            rule(bank, invariant)["verdict"],
            "holds",
            "TLC finds no violation of {invariant}\n{run}"
        );
    }
}

#[test]
fn the_shapes_have_twenty_one_states_and_not_both_is_broken_after_fourteen_changes() {
    let sandbox = with_compiled(SAMPLES, &["shapes"]);
    let run = verify(&sandbox, "shapes", &["--json"]);
    run.exits(1);
    let report = run.json();
    let shapes = model(&report, "shapes");
    assert_eq!(
        shapes["states"], 21,
        "a light of 3 colours and an inbox of 7 messages cycle together: 21 states\n{run}"
    );
    assert_eq!(
        rule(shapes, "notBoth")["verdict"],
        "broken",
        "TLC finds a violation of notBoth\n{run}"
    );
    let text = verify(&sandbox, "shapes", &[]);
    let path = path_under(&text.detail("shapes"), &["broken", "notBoth"]);
    assert_eq!(
        path.steps.len(),
        14,
        "TLC's trace has 15 states: the start and 14 changes\n{text}"
    );
}

#[test]
fn a_path_prints_maps_records_sets_and_lists_with_their_values() {
    let sandbox = with_compiled(FIXTURES, &["missingkey"]);
    let run = verify(&sandbox, "missingkey", &[]);
    run.exits(1);
    let path = path_under(&run.detail("missingKey"), &["fault"]);
    assert!(
        path.start.contains("balances = {\"alice\" -> 1}"),
        "a map prints as {{key -> value}}\n{run}"
    );
    let shapes = with_compiled(SAMPLES, &["shapes"]);
    let text = verify(&shapes, "shapes", &[]);
    let broken = path_under(&text.detail("shapes"), &["broken", "notBoth"]);
    assert!(
        broken.start.contains("pair = {_1: 0, _2: \"x\"}"),
        "a tuple prints as its record\n{text}"
    );
    assert!(
        broken.start.contains("light = {tag: \"Red\", value: {}}"),
        "a variant prints as the record of its tag and its value\n{text}"
    );
}

fn faulted(file: &str, module: &str, invariant: &str) {
    let sandbox = with_compiled(FIXTURES, &[file]);
    let run = verify(&sandbox, file, &["--json"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, module);
    let fault = finding(found, "fault");
    assert!(
        fault["message"]
            .as_str()
            .is_some_and(|message| message.contains("that is not there")),
        "{module} reads something that is not there\n{run}"
    );
    assert!(
        fault["path"]["steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty()),
        "the fault comes with the path to the state\n{run}"
    );
    assert_eq!(
        rule(found, invariant)["verdict"],
        "unknown",
        "after a fault the rule is unknown\n{run}"
    );
}

#[test]
fn getting_a_key_the_map_lacks_is_a_fault_with_a_path() {
    faulted("missingkey", "missingKey", "keyRead");
}

#[test]
fn reading_a_position_past_the_end_of_a_list_is_a_fault_with_a_path() {
    faulted("pastend", "pastTheEnd", "lastRead");
}

#[test]
fn the_head_of_an_empty_list_is_a_fault_with_a_path() {
    faulted("emptyhead", "emptyHead", "headRead");
}

#[test]
fn an_operator_the_reader_does_not_take_makes_the_file_unreadable_and_names_it() {
    let sandbox = with_compiled(FIXTURES, &["refused"]);
    let run = verify(&sandbox, "refused", &[]);
    run.exits(1);
    assert!(
        run.stdout.starts_with("unreadable"),
        "a construct the reader does not take is never skipped\n{run}"
    );
    assert!(
        run.stdout.contains("`chooseSome`") && run.stdout.contains("in the definition `step`"),
        "the problem names the operator and the definition it is in\n{run}"
    );
    assert!(
        run.stdout.contains("`foldr`"),
        "every refused operator is named\n{run}"
    );
}

#[test]
fn a_model_that_uses_every_data_operator_has_the_states_tlc_finds() {
    let sandbox = with_compiled(FIXTURES, &["everything"]);
    let run = verify(&sandbox, "everything", &["--json"]);
    run.exits(0);
    assert_eq!(
        model(&run.json(), "everything")["states"],
        82,
        "TLC finds 82 distinct states and no violation\n{run}"
    );
}
