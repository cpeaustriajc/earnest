use crate::read;
use earnest_ir::{Falls, Model, Plan, Script};
use earnest_value::Value;

const PLACED: &str =
    include_str!("../../../tasks/break_it/models/quint_101/compiled/called_place.json");
const JOINED: &str =
    include_str!("../../../tasks/break_it/models/quint_101/compiled/joined_top.json");
const ZERO: &str = include_str!("../../../tasks/break_it/models/quint_101/compiled/zero_reps.json");

fn model(source: &str) -> Model {
    read("model.json", source).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

fn script<'a>(model: &'a Model, id: &str) -> &'a Script {
    model
        .runs
        .scripts
        .iter()
        .find(|script| script.id == id)
        .unwrap_or_else(|| panic!("no script {id}"))
}

fn within(plan: &Plan) -> Vec<Option<usize>> {
    match &plan.falls {
        Falls::Retried { within, .. } => within.clone(),
        other => panic!("a plan that Quint retries, not {other:?}"),
    }
}

fn picked(plan: &Plan) -> Vec<(&str, &Value)> {
    plan.chosen
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect()
}

#[test]
fn a_run_called_in_the_middle_is_one_plan_per_value_retried_inside_the_call() {
    let placed = model(PLACED);
    let middle = script(&placed, "middleTest");
    assert_eq!(middle.plans.len(), 5, "one plan for each `n` of `fresh`");
    for plan in &middle.plans {
        assert_eq!(
            within(plan),
            [Some(0)],
            "`n` is retried while `fresh` is false"
        );
        let Falls::Retried { calls, .. } = &plan.falls else {
            panic!("a plan that Quint retries");
        };
        let [call] = calls.as_slice() else {
            panic!("one call: {calls:?}");
        };
        assert_eq!(call.says, "the run `fresh`");
        assert_eq!(
            call.last,
            Some(1),
            "the last step of `fresh` is the run's second step"
        );
        assert!(!call.ends_the_run, "something comes after `fresh`");
    }
}

#[test]
fn a_run_called_last_is_retried_while_the_whole_run_is_false() {
    let placed = model(PLACED);
    for plan in &script(&placed, "lastTest").plans {
        assert_eq!(within(plan), [None]);
    }
}

#[test]
fn a_run_called_under_a_top_nondet_names_both_values_outer_first() {
    let placed = model(PLACED);
    let under = script(&placed, "underTopTest");
    assert_eq!(under.plans.len(), 15, "three `m` by five `n`");
    let first = under.plans.first().map(picked);
    assert_eq!(
        first,
        Some(vec![("m", &Value::Int(0)), ("n", &Value::Int(0))])
    );
    for plan in &under.plans {
        assert_eq!(within(plan), [None, Some(0)]);
    }
}

#[test]
fn the_parts_of_an_or_are_numbered_and_played_in_order() {
    let joined = model(JOINED);
    let second = script(&joined, "orSecondTest");
    let parts: Vec<Vec<(&str, &Value)>> = second.plans.iter().map(picked).collect();
    assert_eq!(
        parts,
        [vec![("or", &Value::Int(1))], vec![("or", &Value::Int(2))]]
    );
    for plan in &second.plans {
        let Falls::Retried { ordered, .. } = &plan.falls else {
            panic!("a plan that Quint retries");
        };
        assert_eq!(ordered, &[true]);
    }
}

#[test]
fn the_first_part_of_an_and_makes_the_run_false_where_its_last_step_cannot_happen() {
    let joined = model(JOINED);
    let [plan] = script(&joined, "andFirstFalseTest").plans.as_slice() else {
        panic!("one plan");
    };
    let Falls::Retried { calls, .. } = &plan.falls else {
        panic!("a plan that Quint retries");
    };
    let [call] = calls.as_slice() else {
        panic!("one call: {calls:?}");
    };
    assert!(call.ends_the_run);
    assert_eq!(call.last, Some(0), "`high`, the first part's last step");
    assert_eq!(
        plan.steps.len(),
        3,
        "`high`, then the second part's `init` and `step`"
    );
}

#[test]
fn a_step_after_a_reps_of_0_rounds_that_reads_the_state_ends_the_run_in_an_error() {
    let zero = model(ZERO);
    let [plan] = script(&zero, "zeroReadsTest").plans.as_slice() else {
        panic!("one plan");
    };
    assert!(
        plan.steps.is_empty(),
        "`step` after `0.reps` is never played"
    );
    let Falls::Retried { calls, .. } = &plan.falls else {
        panic!("a plan that Quint retries");
    };
    let [call] = calls.as_slice() else {
        panic!("one call: {calls:?}");
    };
    assert_eq!(call.last, Some(0));
    assert!(!call.ends_the_run);
    assert!(call.reason.contains("QNT502"), "{}", call.reason);
}

#[test]
fn a_reps_of_0_rounds_as_the_last_part_is_no_step() {
    let zero = model(ZERO);
    let [plan] = script(&zero, "zeroTest").plans.as_slice() else {
        panic!("one plan");
    };
    assert!(plan.steps.is_empty());
    assert_eq!(plan.falls, Falls::Breaks);
}
