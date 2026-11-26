use crate::read;
use earnest_ir::{Fairness, Fault, Frame, Model};
use earnest_value::Value;

const FIXES: &str = include_str!("../fixtures/fixes.json");
const FAIR_PART: &str = include_str!("../fixtures/fairpart.json");

fn fixes() -> Model {
    read("fixes.json", FIXES).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

struct Answer(Result<Value, Fault>);

fn answer(model: &Model, id: &str) -> Answer {
    let rule = model
        .rules
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("no rule {id}"));
    let state = [Value::Int(0), Value::Int(0)];
    Answer(rule.holds.eval(&mut Frame::new(&state, rule.slots)))
}

fn at_start(model: &Model, id: &str) -> Result<Value, Fault> {
    answer(model, id).0
}

fn holds_at_start(ids: &[&str]) {
    let model = fixes();
    for id in ids {
        assert_eq!(
            at_start(&model, id),
            Ok(Value::Bool(true)),
            "{id} holds where n = 0"
        );
    }
}

#[test]
fn a_range_that_counts_down_or_stays_is_empty() {
    holds_at_start(&["rangeDown", "rangeSame", "rangeUp"]);
}

#[test]
fn set_to_map_with_every_key_once_is_the_map_and_a_key_twice_is_a_fault() {
    holds_at_start(&["keysOnce"]);
    assert_eq!(at_start(&fixes(), "keyTwice"), Err(Fault::KeyTwice));
}

#[test]
fn a_fold_over_a_set_takes_tlc_order() {
    holds_at_start(&["foldSets", "foldMaps", "foldNumbers"]);
}

#[test]
fn membership_in_a_range_compares_and_any_other_wide_range_is_a_fault() {
    holds_at_start(&[
        "inWide",
        "containsWide",
        "outsideWide",
        "subsetWide",
        "notSubsetWide",
    ]);
    assert_eq!(at_start(&fixes(), "sizeWide"), Err(Fault::TooWide));
}

#[test]
fn fairness_of_an_action_that_assigns_every_variable_or_of_the_step_is_read() {
    let model = fixes();
    let fair = |id: &str| {
        model
            .runs
            .eventually
            .iter()
            .find(|rule| rule.id == id)
            .and_then(|rule| rule.fair.clone())
    };
    assert_eq!(
        fair("moveFair"),
        Some(Fairness {
            step: false,
            actions: vec![vec![
                model.actions.len()
                    + model
                        .runs
                        .fair_steps
                        .iter()
                        .position(|step| step
                            .alone
                            .as_ref()
                            .is_some_and(|alone| alone.named == "move()"))
                        .expect("`move` alone is a fair step")
            ]],
        })
    );
    assert_eq!(
        fair("stepFair"),
        Some(Fairness {
            step: true,
            actions: Vec::new(),
        })
    );
}

#[test]
fn fairness_of_an_action_that_leaves_a_variable_to_another_is_refused() {
    let Err(problems) = read("fairpart.json", FAIR_PART) else {
        panic!("weakFair(bump, (n, t)) must be refused")
    };
    assert!(
        problems
            .iter()
            .any(|problem| problem.feature == "`weakFair`"
                && problem.message.contains("`bump` alone does not give `t'`")
                && problem.message.contains("TLC runs stop")),
        "the refusal names `weakFair`, the action and the variable: {problems:?}"
    );
}
