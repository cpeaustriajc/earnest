use std::fs;

use crate::support::{Run, Sandbox, model, repository};

const FIXTURES: &str = "crates/earnest_quint/fixtures";

fn verified(file: &str) -> (Run, serde_json::Value) {
    let path = repository().join(FIXTURES).join(format!("{file}.json"));
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
    let sandbox = Sandbox::empty();
    sandbox.write(&format!("models/{file}.json"), &source);
    let run = sandbox.earnest(&["verify", &format!("models/{file}.json"), "--json"]);
    let report = run.json();
    (run, report)
}

fn verdicts(report: &serde_json::Value, name: &str) -> Vec<(String, String)> {
    model(report, name)["rules"]
        .as_array()
        .expect("a model lists its rules")
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn pairs(all: &[(&str, &str)]) -> Vec<(String, String)> {
    all.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

#[test]
fn the_owners_all_of_conditions_gives_tlcs_four_states_and_type_ok_holds() {
    let (run, report) = verified("allasbool");
    run.exits(0);
    assert_eq!(
        model(&report, "AllAsBool")["states"],
        4,
        "quint verify --backend tlc finds 4 distinct states\n{run}"
    );
    assert_eq!(
        verdicts(&report, "AllAsBool"),
        pairs(&[("TypeOK", "holds")])
    );
}

#[test]
fn the_owners_action_given_as_an_argument_gives_tlcs_four_states_and_bounded_holds() {
    let (run, report) = verified("actionargument");
    run.exits(0);
    assert_eq!(
        model(&report, "ActionArgument")["states"],
        4,
        "quint verify --backend tlc finds 4 distinct states\n{run}"
    );
    assert_eq!(
        verdicts(&report, "ActionArgument"),
        pairs(&[("Bounded", "holds")])
    );
}

#[test]
fn the_wider_sample_gives_tlcs_nine_states_its_verdicts_and_the_runs_quint_test_plays() {
    let (run, report) = verified("wider");
    run.exits(1);
    assert_eq!(
        model(&report, "wider")["states"],
        9,
        "quint verify --backend tlc finds 9 distinct states\n{run}"
    );
    assert_eq!(
        verdicts(&report, "wider"),
        pairs(&[
            ("Sane", "holds"),
            ("Few", "broken"),
            ("Low", "broken"),
            ("moveTest", "holds"),
            ("wrongTest", "broken"),
        ]),
        "TLC: `Sane` holds, `Few` and `Low` are violated; quint test: `moveTest` passes and \
         `wrongTest` fails\n{run}"
    );
}
