use crate::{Runs, read, read_located};
use earnest_ir::Model;
use earnest_value::{Notation, Quint};

const MATCHES: &str = include_str!("../fixtures/matches.json");
const ASSERTS: &str = include_str!("../fixtures/asserts.json");
const FAIR_TUPLE: &str = include_str!("../fixtures/fairtuple.json");
const VALUES: &str = include_str!("../fixtures/printed.json");
const LOOKALIKE: &str = include_str!("../fixtures/lookalike.json");
const RUN_PLACE: &str = include_str!("../fixtures/runplace.json");

fn model(source: &str) -> Model {
    read("model.json", source).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

fn ids(model: &Model) -> Vec<&str> {
    model
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect()
}

#[test]
fn each_branch_of_a_match_of_actions_is_an_action_and_each_branch_in_init_a_start() {
    let matches = model(MATCHES);
    assert_eq!(
        ids(&matches),
        [
            "follow Move",
            "follow Jump",
            "follow _",
            "turn",
            "turn#2",
            "turn Idle",
            "turn _"
        ],
        "follow's three branches, named by their variants, then turn's: the `if`'s two, and the \
         `match`'s `Idle` and `_`, whose `false` keeps it from happening"
    );
    assert_eq!(
        matches.runs.starts.as_ref().map(Vec::len),
        Some(3),
        "init's `match` gives each of the three orders its start"
    );
}

#[test]
fn an_assert_inside_an_action_is_read() {
    let asserts = model(ASSERTS);
    assert_eq!(
        ids(&asserts),
        ["up", "down"],
        "both asserts are part of a guard"
    );
}

#[test]
fn a_tuple_of_variables_through_vals_is_fairness_over_every_variable() {
    let fair = model(FAIR_TUPLE);
    let fair_to = |id: &str| {
        fair.runs
            .eventually
            .iter()
            .find(|rule| rule.id == id)
            .and_then(|rule| rule.fair.clone())
            .unwrap_or_else(|| panic!("no rule about runs {id}"))
    };
    for id in ["finishes", "finishesThroughVals", "finishesBackwards"] {
        assert_eq!(
            fair_to(id).actions,
            [vec![0]],
            "{id} is fair to `tick`, the step's first action"
        );
    }
    assert!(fair_to("unfairFinishes").actions.is_empty(), "no fairness");
    assert!(
        fair.rules.iter().any(|rule| rule.id == "neverFour"),
        "`not(eventually(P))` is a rule that always holds"
    );
    assert!(
        fair.runs
            .eventually
            .iter()
            .any(|rule| rule.id == "leavesZero"),
        "`not(always(P))` is a rule about runs"
    );
}

#[test]
fn a_module_that_writes_a_record_like_a_variant_keeps_variants_as_records() {
    assert_eq!(
        model(VALUES).symbols.notation(),
        Notation::Quint(Quint {
            tuples: true,
            variants: true,
        }),
        "values writes tuples and variants only"
    );
    assert_eq!(
        model(LOOKALIKE).symbols.notation(),
        Notation::Quint(Quint {
            tuples: true,
            variants: false,
        }),
        "lookalike writes the record `{{ tag: \"a\", value: 0 }}`"
    );
}

#[test]
fn a_refusal_inside_a_run_knows_the_run_and_the_refused_expression() {
    let Err(refused) = read_located("runplace.json", RUN_PLACE, &Runs::Tests) else {
        panic!("runplace's runs are refused");
    };
    let found: Vec<(Option<&str>, bool)> = refused
        .iter()
        .map(|each| (each.definition.as_deref(), each.id.is_some()))
        .collect();
    assert_eq!(
        found,
        [
            (Some("fromNowhereTest"), true),
            (Some("lateFailTest"), true)
        ],
        "{refused:?}"
    );
}
