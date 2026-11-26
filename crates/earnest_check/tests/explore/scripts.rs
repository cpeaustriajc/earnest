use crate::build::{Sketch, apply, below, int, is, plus, steps, texts, yes};
use earnest_check::{Limits, Outcome, RuleKind, RuleOutcome, Status, Verdict, explore};
use earnest_ir::{Code, Expr, Math, Model, Op, Order, Plan, Script, Stage, Witness};
use earnest_value::{Domain, Value};

const UP: usize = 0;
const JUMP: usize = 1;
const RESET: usize = 2;
const MOST_BRANCHES: &str = "more than 100000 branches";

fn numbers(all: &[i64]) -> Vec<Value> {
    all.iter().copied().map(Value::Int).collect()
}

fn ladder(starts: &[i64]) -> (Sketch, Expr) {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 3, starts);
    sketch.action("up", below(&n, 3), &[(&n, plus(&n, 1))]);
    let landed = apply(Op::Math(Math::Add), &n, Expr::Local(0));
    sketch.action_with(
        "jump",
        &[("by", numbers(&[1, 2]))],
        apply(Op::Order(Order::LessOrEqual), &landed, int(3)),
        &[(&n, landed.clone())],
    );
    sketch.action("reset", yes(), &[(&n, int(0))]);
    (sketch, n)
}

fn stage(choices: &[usize]) -> Stage {
    Stage {
        choices: choices.to_vec(),
        asserts: None,
        expect: None,
        refused: false,
    }
}

fn expecting(choices: &[usize], expect: Expr) -> Stage {
    Stage {
        expect: Some(Code::new(expect)),
        ..stage(choices)
    }
}

fn refused(choices: &[usize]) -> Stage {
    Stage {
        refused: true,
        ..stage(choices)
    }
}

fn plan(steps: Vec<Stage>) -> Plan {
    Plan {
        chosen: Vec::new(),
        start: None,
        steps,
        must_not_finish: false,
    }
}

fn played(sketch: Sketch, scripts: Vec<(&str, Vec<Plan>)>) -> Model {
    let mut model = sketch.model();
    let actions = model.actions.clone();
    model.runs.scripts = scripts
        .into_iter()
        .map(|(id, plans)| Script {
            id: id.to_owned(),
            says: format!("The Quint run `{id}` passes"),
            line: 1,
            actions: actions.clone(),
            plans,
            slots: 0,
        })
        .collect();
    model
}

fn searched(sketch: Sketch, id: &str, plans: Vec<Plan>) -> Outcome {
    explore(&played(sketch, vec![(id, plans)]), &Limits::default())
}

fn the_run<'a>(outcome: &'a Outcome, id: &str) -> &'a RuleOutcome {
    outcome
        .rules
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("no outcome for {id} in {:?}", outcome.rules))
}

fn holds(outcome: &Outcome, id: &str) {
    let found = the_run(outcome, id);
    assert_eq!(found.verdict, Verdict::Holds, "{id} holds: {found:?}");
    assert_eq!(found.kind, RuleKind::Witness, "{id} is a witness");
    assert_eq!(found.path, None, "{id} has no path");
    assert_eq!(found.reason, None, "{id} has no reason");
}

fn broken(outcome: &Outcome, id: &str) -> (Vec<String>, Vec<String>, String) {
    let found = the_run(outcome, id);
    assert_eq!(found.verdict, Verdict::Broken, "{id} is broken: {found:?}");
    assert_eq!(found.kind, RuleKind::Witness, "{id} is a witness");
    let path = found
        .path
        .as_ref()
        .unwrap_or_else(|| panic!("{id} has the path played: {found:?}"));
    let reason = found
        .reason
        .clone()
        .unwrap_or_else(|| panic!("{id} has a reason: {found:?}"));
    (texts(&path.start), steps(path), reason)
}

#[test]
fn a_step_of_alternatives_plays_each_action_with_each_input_that_can_happen() {
    let (sketch, n) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "climbs",
        vec![plan(vec![expecting(
            &[UP, JUMP],
            apply(Op::Order(Order::GreaterOrEqual), &n, int(1)),
        )])],
    );
    holds(&outcome, "climbs");
    assert_eq!(
        the_run(&outcome, "climbs").says,
        "The Quint run `climbs` passes"
    );
    let (sketch, n) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "one",
        vec![plan(vec![expecting(&[UP, JUMP], is(&n, 1))])],
    );
    assert_eq!(
        broken(&outcome, "one"),
        (
            vec!["n = 0".to_owned()],
            vec!["jump  by = 2  n: 0 -> 2".to_owned()],
            "what the run expects after step 1 is false here".to_owned()
        ),
        "up and a jump by 1 give 1, so the jump by 2 is the first branch to break, with its input"
    );
}

#[test]
fn a_step_that_no_alternative_can_take_breaks_the_run_and_names_them() {
    let (sketch, _) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "over",
        vec![plan(vec![
            stage(&[UP]),
            stage(&[UP]),
            stage(&[UP]),
            stage(&[UP, JUMP]),
        ])],
    );
    let (start, path, reason) = broken(&outcome, "over");
    assert_eq!(start, ["n = 0"], "the start");
    assert_eq!(path.len(), 3, "the three steps up: {path:?}");
    assert_eq!(
        reason, "step 4: none of `up`, `jump` can happen here",
        "neither up nor a jump by any value goes past 3"
    );
}

#[test]
fn a_condition_after_a_middle_step_is_checked_where_it_is_written() {
    let (sketch, n) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "middle",
        vec![plan(vec![
            stage(&[UP]),
            expecting(&[UP], is(&n, 2)),
            stage(&[UP]),
        ])],
    );
    holds(&outcome, "middle");
    let (sketch, n) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "early",
        vec![plan(vec![
            stage(&[UP]),
            expecting(&[UP], is(&n, 3)),
            stage(&[UP]),
        ])],
    );
    let (_, path, reason) = broken(&outcome, "early");
    assert_eq!(
        path,
        ["up  n: 0 -> 1", "up  n: 1 -> 2"],
        "the path stops at the step the condition follows"
    );
    assert_eq!(reason, "what the run expects after step 2 is false here");
}

#[test]
fn a_last_step_that_must_be_refused_holds_where_it_can_not_happen_and_breaks_where_it_can() {
    let (sketch, _) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "stops",
        vec![plan(vec![
            stage(&[UP]),
            stage(&[UP]),
            stage(&[UP]),
            refused(&[UP]),
        ])],
    );
    holds(&outcome, "stops");
    let (sketch, _) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "goes",
        vec![plan(vec![stage(&[UP]), refused(&[UP])])],
    );
    let (_, path, reason) = broken(&outcome, "goes");
    assert_eq!(
        path,
        ["up  n: 0 -> 1", "up  n: 1 -> 2"],
        "the path shows the step that could happen"
    );
    assert_eq!(
        reason,
        "step 2, `up`, can happen here, and the run's `fail()` expects it can't"
    );
}

#[test]
fn a_run_is_played_from_every_start_state_and_a_broken_one_names_the_values_it_chose() {
    let (sketch, _) = ladder(&[0, 2]);
    let mut twice = plan(vec![stage(&[UP]), stage(&[UP])]);
    twice.chosen = vec![("k".to_owned(), Value::Int(2))];
    let outcome = searched(sketch, "twice", vec![twice]);
    let (start, path, reason) = broken(&outcome, "twice");
    assert_eq!(start, ["n = 2"], "the second start state breaks it");
    assert_eq!(path, ["up  n: 2 -> 3"], "one step up from 2");
    assert_eq!(reason, "with `k` = 2: step 2, `up`, can't happen here");
}

#[test]
fn every_plan_must_hold() {
    let (sketch, n) = ladder(&[0]);
    let outcome = searched(
        sketch,
        "plans",
        vec![
            plan(vec![expecting(&[UP], is(&n, 1))]),
            plan(vec![expecting(&[RESET], is(&n, 1))]),
        ],
    );
    let (_, path, reason) = broken(&outcome, "plans");
    assert_eq!(
        path,
        ["reset"],
        "the second plan resets, which changes nothing"
    );
    assert_eq!(reason, "what the run expects after step 1 is false here");
}

#[test]
fn an_assertion_a_start_condition_and_a_run_that_must_not_finish_each_say_so() {
    let (sketch, n) = ladder(&[0]);
    let mut asserted = stage(&[UP]);
    asserted.asserts = Some(Code::new(is(&n, 1)));
    let mut started = plan(vec![stage(&[UP])]);
    started.start = Some(Code::new(is(&n, 1)));
    let mut through = plan(vec![stage(&[UP])]);
    through.must_not_finish = true;
    let mut false_run = plan(Vec::new());
    false_run.must_not_finish = true;
    let model = played(
        sketch,
        vec![
            ("asserted", vec![plan(vec![asserted])]),
            ("started", vec![started]),
            ("through", vec![through]),
            ("false", vec![false_run]),
        ],
    );
    let outcome = explore(&model, &Limits::default());
    let reasons: Vec<(Vec<String>, String)> = ["asserted", "started", "through", "false"]
        .into_iter()
        .map(|id| {
            let (_, path, reason) = broken(&outcome, id);
            (path, reason)
        })
        .collect();
    assert_eq!(
        reasons,
        [
            (
                Vec::new(),
                "the assertion of step 1 is false here".to_owned()
            ),
            (
                Vec::new(),
                "what the run expects at the start is false here".to_owned()
            ),
            (
                vec!["up  n: 0 -> 1".to_owned()],
                "the run gets through every step, and its `fail()` expects it not to".to_owned()
            ),
            (Vec::new(), "the run is false".to_owned()),
        ],
        "an assertion is checked before the step, a start condition before the first step"
    );
}

fn wide_run() -> Model {
    let mut sketch = Sketch::new();
    let m = sketch.field(
        "m",
        Domain::Int {
            lo: 0,
            hi: 10_000_000,
        },
        numbers(&[0]),
    );
    let digits: Vec<i64> = (0..1000).collect();
    sketch.action_with(
        "add",
        &[("d", numbers(&digits))],
        yes(),
        &[(&m, apply(Op::Math(Math::Add), &m, Expr::Local(0)))],
    );
    sketch.end("anywhere", yes());
    let mut model = played(
        sketch,
        vec![("wide", vec![plan(vec![stage(&[0]), stage(&[0])])])],
    );
    model.actions.clear();
    model
}

#[test]
fn a_run_with_more_branches_than_the_cap_is_unknown_and_says_so() {
    let outcome = explore(&wide_run(), &Limits::default());
    let found = the_run(&outcome, "wide");
    assert_eq!(found.verdict, Verdict::Unknown, "never holds: {found:?}");
    assert_eq!(found.path, None, "no path");
    assert!(
        found
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains(MOST_BRANCHES)),
        "the reason names the cap: {found:?}"
    );
}

#[test]
fn only_rule_plays_only_the_run_it_names_and_runs_come_after_the_rules() {
    let (mut sketch, n) = ladder(&[0]);
    sketch.rule("low", below(&n, 4));
    let model = played(
        sketch,
        vec![
            ("climbs", vec![plan(vec![stage(&[UP])])]),
            ("over", vec![plan(vec![stage(&[UP]), refused(&[UP])])]),
        ],
    );
    let ids = |outcome: &Outcome| -> Vec<String> {
        outcome.rules.iter().map(|rule| rule.id.clone()).collect()
    };
    let all = explore(&model, &Limits::default());
    assert_eq!(
        ids(&all),
        ["low", "climbs", "over"],
        "the rule, then the runs"
    );
    assert_eq!(
        model.rule_ids().collect::<Vec<_>>(),
        ["low", "climbs", "over"],
        "the model lists the runs among its rule ids"
    );
    let only = |id: &str| {
        let limits = Limits {
            only_rule: Some(id.to_owned()),
            ..Limits::default()
        };
        let outcome = explore(&model, &limits);
        (ids(&outcome), outcome.status)
    };
    assert_eq!(
        only("over"),
        (vec!["over".to_owned()], Status::Fail),
        "only the broken run"
    );
    assert_eq!(
        only("climbs"),
        (vec!["climbs".to_owned()], Status::Pass),
        "only the run that holds"
    );
    assert_eq!(
        only("low"),
        (vec!["low".to_owned()], Status::Pass),
        "only the rule, so the broken run is not played"
    );
}

#[test]
fn a_run_stopped_at_the_branch_cap_leaves_the_model_unfinished_with_a_reason_that_names_it() {
    let outcome = explore(&wide_run(), &Limits::default());
    assert_eq!(
        outcome.status,
        Status::Unfinished,
        "never pass: {outcome:?}"
    );
    assert_eq!(
        outcome.reason.as_deref(),
        Some("the run `wide` has more than 100000 branches from one start state"),
        "the run and the limit"
    );
}

#[test]
fn a_search_stopped_at_its_state_limit_keeps_its_own_reason() {
    let mut model = wide_run();
    let more_starts: Vec<Vec<Value>> = (0..10).map(|start| numbers(&[start])).collect();
    model.runs.starts = Some(more_starts);
    let limits = Limits {
        max_states: 5,
        ..Limits::default()
    };
    let outcome = explore(&model, &limits);
    assert_eq!(outcome.status, Status::Unfinished, "{outcome:?}");
    let reason = outcome.reason.unwrap_or_default();
    assert!(
        !reason.is_empty() && !reason.contains("branches"),
        "the state limit is the reason, not the run: {reason}"
    );
}

#[test]
fn a_witness_with_more_start_states_than_the_limit_leaves_the_model_unfinished() {
    let (sketch, n) = ladder(&[0, 1, 2]);
    let mut model = sketch.model();
    model.runs.witnesses = vec![Witness {
        id: "low".to_owned(),
        says: "low can be done".to_owned(),
        line: 1,
        setup: None,
        steps: Vec::new(),
        expect: Code::new(below(&n, 3)),
        slots: 0,
    }];
    let limits = Limits {
        max_states: 2,
        ..Limits::default()
    };
    let outcome = explore(&model, &limits);
    assert_eq!(
        the_run(&outcome, "low").verdict,
        Verdict::Unknown,
        "three start states and a limit of two"
    );
    assert_eq!(
        outcome.status,
        Status::Unfinished,
        "never pass: {outcome:?}"
    );
    assert!(outcome.reason.is_some(), "a reason: {outcome:?}");
}
