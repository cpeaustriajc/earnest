use crate::build::{Sketch, below, int, is, steps, texts};
use earnest_check::{Limits, Outcome, RuleKind, RuleOutcome, Verdict, explore};
use earnest_ir::{Code, Eventually, Expr, Fairness, Model};

const STAYS: &str = "nothing changes the state from here, so the run can stay in it forever";
const STOPS: &str =
    "the run may stop here, because no step it must take can change the state from here";
const A: usize = 0;
const B: usize = 1;
const OUT: usize = 2;

fn only(actions: &[&[usize]]) -> Fairness {
    Fairness {
        step: false,
        actions: actions.iter().map(|part| part.to_vec()).collect(),
    }
}

fn with_the_step(actions: &[&[usize]]) -> Fairness {
    Fairness {
        step: true,
        ..only(actions)
    }
}

fn nothing_fair() -> Fairness {
    only(&[])
}

fn reaches(id: &str, goal: Expr, fairness: Fairness) -> Eventually {
    Eventually {
        id: id.to_owned(),
        says: format!("{id} comes"),
        line: 1,
        after: None,
        reaches: Code::new(goal),
        slots: 0,
        fair: Some(fairness),
    }
}

fn searched(model: &Model) -> Outcome {
    let one = explore(
        model,
        &Limits {
            jobs: 1,
            ..Limits::default()
        },
    );
    let four = explore(
        model,
        &Limits {
            jobs: 4,
            ..Limits::default()
        },
    );
    assert_eq!(one, four, "one worker and four find the same");
    one
}

fn the_rule<'a>(outcome: &'a Outcome, id: &str) -> &'a RuleOutcome {
    let found = outcome
        .rules
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("no outcome for {id} in {:?}", outcome.rules));
    assert_eq!(found.kind, RuleKind::Eventually, "{id} is about runs");
    found
}

fn holds(outcome: &Outcome, id: &str) {
    let found = the_rule(outcome, id);
    assert_eq!(found.verdict, Verdict::Holds, "{id} holds: {found:?}");
    assert_eq!(found.path, None, "{id}: a rule that holds has no path");
}

fn broken(outcome: &Outcome, id: &str) -> (Vec<String>, Vec<String>, Option<usize>, String) {
    let found = the_rule(outcome, id);
    assert_eq!(found.verdict, Verdict::Broken, "{id} is broken: {found:?}");
    let path = found
        .path
        .as_ref()
        .unwrap_or_else(|| panic!("{id} has a path: {found:?}"));
    (
        texts(&path.start),
        steps(path),
        found.repeats_from,
        found.reason.clone().unwrap_or_default(),
    )
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).to_owned()).collect()
}

fn with(sketch: Sketch, rules: Vec<Eventually>) -> Model {
    let mut model = sketch.model();
    model.runs.eventually = rules;
    model
}

fn still() -> (Sketch, Expr) {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 1, &[0]);
    (sketch, n)
}

fn one_way_out() -> (Sketch, Expr) {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 1, &[0]);
    sketch.action("up", below(&n, 1), &[(&n, int(1))]);
    sketch.action("never", is(&n, 5), &[(&n, int(0))]);
    (sketch, n)
}

fn swing(out_from: fn(&Expr) -> Expr) -> (Sketch, Expr) {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 2, &[0]);
    let out_from = out_from(&n);
    sketch.action("a", is(&n, 0), &[(&n, int(1))]);
    sketch.action("b", is(&n, 1), &[(&n, int(0))]);
    sketch.action("out", out_from, &[(&n, int(2))]);
    (sketch, n)
}

#[test]
fn a_state_without_ways_out_is_kept_forever_whatever_is_fair() {
    let (sketch, n) = still();
    let model = with(
        sketch,
        vec![
            reaches("unfair", is(&n, 1), nothing_fair()),
            reaches("stepFair", is(&n, 1), with_the_step(&[])),
        ],
    );
    let outcome = searched(&model);
    for id in ["unfair", "stepFair"] {
        assert_eq!(
            broken(&outcome, id),
            (owned(&["n = 0"]), Vec::new(), None, STAYS.to_owned())
        );
    }
}

#[test]
fn a_run_may_stop_where_it_could_go_on_when_nothing_there_is_fair() {
    let (sketch, n) = one_way_out();
    let model = with(
        sketch,
        vec![
            reaches("unfair", is(&n, 1), nothing_fair()),
            reaches("neverFair", is(&n, 1), only(&[&[1]])),
            reaches("upFair", is(&n, 1), only(&[&[0]])),
            reaches("stepFair", is(&n, 1), with_the_step(&[])),
        ],
    );
    let outcome = searched(&model);
    for id in ["unfair", "neverFair"] {
        assert_eq!(
            broken(&outcome, id),
            (owned(&["n = 0"]), Vec::new(), None, STOPS.to_owned())
        );
    }
    holds(&outcome, "upFair");
    holds(&outcome, "stepFair");
}

#[test]
fn the_model_fairness_is_kept_for_a_rule_without_its_own() {
    let (sketch, n) = one_way_out();
    let mut model = with(sketch, vec![reaches("unfair", is(&n, 1), nothing_fair())]);
    model.runs.eventually.push(Eventually {
        fair: None,
        ..reaches("modelFair", is(&n, 1), nothing_fair())
    });
    let outcome = searched(&model);
    holds(&outcome, "modelFair");
    assert_eq!(broken(&outcome, "unfair").3, STOPS);
}

#[test]
fn a_loop_of_fair_actions_needs_each_one_s_turn() {
    let (sketch, n) = swing(|n| below(n, 2));
    let model = with(
        sketch,
        vec![
            reaches("swingFair", is(&n, 2), only(&[&[A], &[B]])),
            reaches("outFair", is(&n, 2), only(&[&[OUT]])),
            reaches("allFair", is(&n, 2), only(&[&[A], &[B], &[OUT]])),
            reaches("eitherFair", is(&n, 2), only(&[&[OUT, A]])),
            reaches("stepFair", is(&n, 2), with_the_step(&[])),
        ],
    );
    let outcome = searched(&model);
    let looped = (
        owned(&["n = 0"]),
        owned(&["a  n: 0 -> 1", "b  n: 1 -> 0"]),
        Some(0),
        "steps 1 to 2 can repeat forever".to_owned(),
    );
    for id in ["swingFair", "eitherFair", "stepFair"] {
        assert_eq!(broken(&outcome, id), looped, "{id}");
    }
    holds(&outcome, "outFair");
    holds(&outcome, "allFair");
}

#[test]
fn a_fair_action_that_is_not_always_possible_can_be_put_off() {
    let (sketch, n) = swing(|n| is(n, 1));
    let model = with(
        sketch,
        vec![
            reaches("outFair", is(&n, 2), only(&[&[OUT]])),
            reaches("outAndAFair", is(&n, 2), only(&[&[OUT], &[A]])),
            reaches("outAndStepFair", is(&n, 2), with_the_step(&[&[OUT]])),
        ],
    );
    let outcome = searched(&model);
    assert_eq!(
        broken(&outcome, "outFair"),
        (owned(&["n = 0"]), Vec::new(), None, STOPS.to_owned())
    );
    let looped = (
        owned(&["n = 0"]),
        owned(&["a  n: 0 -> 1", "b  n: 1 -> 0"]),
        Some(0),
        "steps 1 to 2 can repeat forever".to_owned(),
    );
    assert_eq!(broken(&outcome, "outAndAFair"), looped);
    assert_eq!(broken(&outcome, "outAndStepFair"), looped);
}
