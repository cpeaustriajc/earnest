use std::fs;

use crate::support::{Run, Sandbox, model, repository, rule};

const FIXTURES: &str = "crates/earnest_quint/fixtures";

fn with_compiled(files: &[&str]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for file in files {
        let path = repository().join(FIXTURES).join(format!("{file}.json"));
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
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
fn the_owners_fairness_over_a_set_of_the_state_is_read_and_holds_as_tlc_says() {
    let sandbox = with_compiled(&["fairnessval"]);
    let run = verify(&sandbox, "fairnessval", &["--json"]);
    run.exits(0);
    let report = run.json();
    assert_eq!(model(&report, "FairnessVal")["states"], 3, "{run}");
    assert_eq!(
        verdicts(&report, "FairnessVal"),
        pairs(&[("ReachesTwo", "holds")]),
        "quint verify --backend tlc --temporal ReachesTwo: no violation, 3 states\n{run}"
    );
}

#[test]
fn the_owners_property_that_is_an_and_is_checked_as_each_of_its_parts() {
    let sandbox = with_compiled(&["fairnessinand"]);
    let run = verify(&sandbox, "fairnessinand", &["--json"]);
    run.exits(0);
    let report = run.json();
    let found = model(&report, "FairnessInAnd");
    assert_eq!(found["states"], 3, "{run}");
    assert_eq!(
        verdicts(&report, "FairnessInAnd"),
        pairs(&[("ReachesTwo", "holds"), ("ReachesTwo#2", "holds")]),
        "TLC: `always(st.n <= 2)` and `fair.implies(eventually(st.n == 2))` both hold\n{run}"
    );
    assert_eq!(rule(found, "ReachesTwo#2")["kind"], "eventually", "{run}");
}

#[test]
fn fairness_over_any_one_to_one_shape_of_the_variables_and_each_part_of_an_and_are_judged_as_tlc_judges_them()
 {
    let sandbox = with_compiled(&["fairshapes"]);
    let run = verify(&sandbox, "fairshapes", &["--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(model(&report, "fairshapes")["states"], 6, "{run}");
    let mut found = verdicts(&report, "fairshapes");
    found.sort();
    let mut expected = pairs(&[
        ("overPair", "holds"),
        ("overRecord", "holds"),
        ("overNested", "holds"),
        ("overList", "holds"),
        ("overSetOfOne", "holds"),
        ("parts", "holds"),
        ("parts#2", "holds"),
        ("parts#3", "holds"),
        ("parts#4", "broken"),
        ("infix", "holds"),
        ("infix#2", "holds"),
        ("infix#3", "holds"),
        ("nestedParts", "holds"),
        ("nestedParts#2", "holds"),
        ("nestedParts#3", "broken"),
        ("startsAtOne", "broken"),
        ("eachTarget", "holds"),
        ("eachTarget#2", "holds"),
        ("fairEach", "holds"),
        ("unfairEach", "broken"),
        ("unfairEach#2", "broken"),
    ]);
    expected.sort();
    assert_eq!(
        found, expected,
        "each verdict is the one quint verify --backend tlc gives the property, or the part \
         written as a property of its own\n{run}"
    );
}

#[test]
fn a_rule_named_on_the_command_line_selects_every_part_of_that_property() {
    let sandbox = with_compiled(&["fairshapes"]);
    let run = verify(&sandbox, "fairshapes", &["--json", "--rule", "parts"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        verdicts(&report, "fairshapes"),
        pairs(&[
            ("parts", "holds"),
            ("parts#2", "holds"),
            ("parts#3", "holds"),
            ("parts#4", "broken"),
        ]),
        "{run}"
    );
}

#[test]
fn fairness_of_an_action_given_each_buyer_over_some_variables_is_judged_as_tlc_judges_it() {
    let sandbox = with_compiled(&["buyers"]);
    let run = verify(&sandbox, "buyers", &["--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(model(&report, "buyers")["states"], 18, "{run}");
    let mut found = verdicts(&report, "buyers");
    found.sort();
    let mut expected = pairs(&[
        ("allReach", "holds"),
        ("unfairAllReach", "broken"),
        ("noisy", "broken"),
        ("annReaches", "holds"),
        ("bobReaches", "broken"),
        ("overPeer", "broken"),
        ("parts", "holds"),
        ("parts#2", "holds"),
        ("stepOverSome", "holds"),
        ("allForms", "holds"),
        ("allForms#2", "holds"),
    ]);
    expected.sort();
    assert_eq!(
        found, expected,
        "each verdict is the one quint verify --backend tlc gives the property\n{run}"
    );
}

#[test]
fn the_tla_export_writes_the_fairness_of_an_action_given_an_argument_over_the_tuple_written() {
    let sandbox = with_compiled(&["buyers"]);
    let run = sandbox.earnest(&["tla", "models/buyers.json", "--out", "exported"]);
    run.exits(0);
    let module = fs::read_to_string(sandbox.path().join("exported/buyers.tla"))
        .unwrap_or_else(|error| panic!("the module is written: {error}\n{run}"));
    assert!(
        module.contains(
            "Fair_reachDeadline_1_way_1 ==\n    IF InRange\n    THEN /\\ lastBuyer' = \"ann\""
        ) && module.contains("Fair_reachDeadline_1 ==\n    Fair_reachDeadline_1_way_1\n"),
        "the fairness is of `reachDeadline(\"ann\")` alone, its definition given `ann`, with the \
         variables it does not assign left to the rest of the step\n{module}"
    );
    assert!(
        module.contains(
            "Eventually_annReaches ==\n    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1)) =>"
        ),
        "the fairness is over the tuple the file wrote\n{module}"
    );
}

#[test]
fn the_owners_fairness_per_buyer_of_an_action_given_the_buyer_holds_and_one_buyer_alone_does_not() {
    let sandbox = with_compiled(&["fairargs"]);
    let run = verify(&sandbox, "fairargs", &["--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(model(&report, "FairArgs")["states"], 4, "{run}");
    assert_eq!(
        verdicts(&report, "FairArgs"),
        pairs(&[
            ("EveryBuyerFinishes", "holds"),
            ("EveryBuyerFinishes#2", "holds"),
            ("OnlyBuyerOne", "holds"),
            ("OnlyBuyerOne#2", "broken"),
        ]),
        "TLC: EveryBuyerFinishes holds; fair to `finish(1)` alone, buyer 2 may never finish\n{run}"
    );
}
