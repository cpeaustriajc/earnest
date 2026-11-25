use crate::build::{Sketch, apply, below, int, is, not, plus, steps, texts, yes};
use earnest_check::{Limits, Outcome, RuleKind, RuleOutcome, Status, Verdict, explore};
use earnest_ir::{Code, Expr, Math, Model, Op, Order, Played, Witness};
use earnest_value::{Domain, Value};

const UP: usize = 0;
const DOWN: usize = 1;
const OPEN: usize = 2;
const SHUT: usize = 3;

struct Lift {
    sketch: Sketch,
    floor: Expr,
    doors: Expr,
}

fn lift() -> Lift {
    let mut sketch = Sketch::new();
    let floor = sketch.number("floor", 0, 2, &[0, 1, 2]);
    let doors = sketch.boolean("doors", &[false]);
    let closed = not(&doors);
    sketch.action(
        "up",
        Expr::And(Box::new(closed.clone()), Box::new(below(&floor, 2))),
        &[(&floor, plus(&floor, 1))],
    );
    sketch.action(
        "down",
        Expr::And(
            Box::new(closed.clone()),
            Box::new(apply(Op::Order(Order::Greater), &floor, int(0))),
        ),
        &[(&floor, plus(&floor, -1))],
    );
    sketch.action("open", closed, &[(&doors, yes())]);
    sketch.action("shut", doors.clone(), &[(&doors, no())]);
    Lift {
        sketch,
        floor,
        doors,
    }
}

fn no() -> Expr {
    Expr::Const(Value::Bool(false))
}

fn played(action: usize) -> Played {
    Played {
        action,
        input: Vec::new(),
    }
}

fn witness(id: &str, steps: Vec<Played>, expect: Expr) -> Witness {
    Witness {
        id: id.to_owned(),
        says: format!("{id} can be done"),
        line: 1,
        setup: None,
        steps,
        expect: Code::new(expect),
        slots: 0,
    }
}

fn with_witnesses(sketch: Sketch, witnesses: Vec<Witness>) -> Model {
    let mut model = sketch.model();
    model.runs.witnesses = witnesses;
    model
}

fn searched(sketch: Sketch, witnesses: Vec<Witness>) -> Outcome {
    explore(&with_witnesses(sketch, witnesses), &Limits::default())
}

fn the_witness<'a>(outcome: &'a Outcome, id: &str) -> &'a RuleOutcome {
    outcome
        .rules
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("no outcome for {id} in {:?}", outcome.rules))
}

fn broken<'a>(outcome: &'a Outcome, id: &str) -> (Vec<String>, Vec<String>, &'a str) {
    let found = the_witness(outcome, id);
    assert_eq!(found.verdict, Verdict::Broken, "{id} is broken: {found:?}");
    assert_eq!(found.kind, RuleKind::Witness, "{id} is a witness");
    let path = found
        .path
        .as_ref()
        .unwrap_or_else(|| panic!("{id} has the path played: {found:?}"));
    let reason = found
        .reason
        .as_deref()
        .unwrap_or_else(|| panic!("{id} has a reason: {found:?}"));
    (texts(&path.start), steps(path), reason)
}

#[test]
fn a_witness_that_can_be_played_from_every_start_holds() {
    let Lift { sketch, doors, .. } = lift();
    let rides = witness("rides", vec![played(OPEN), played(SHUT)], not(&doors));
    let outcome = searched(sketch, vec![rides]);
    let found = the_witness(&outcome, "rides");
    assert_eq!(
        found.verdict,
        Verdict::Holds,
        "open then shut from every floor"
    );
    assert_eq!(found.kind, RuleKind::Witness, "the kind");
    assert_eq!(found.path, None, "a witness that holds has no path");
    assert_eq!(found.reason, None, "a witness that holds has no reason");
    assert_eq!(found.says, "rides can be done", "the sentence");
    assert_eq!(outcome.status, Status::Pass, "nothing is broken");
    assert_eq!(outcome.states, 6, "the witness does not change the count");
}

#[test]
fn a_step_that_can_not_happen_breaks_the_witness_after_the_steps_before_it() {
    let Lift { sketch, floor, .. } = lift();
    let climbs = witness("climbs", vec![played(UP), played(UP)], is(&floor, 2));
    let outcome = searched(sketch, vec![climbs]);
    assert_eq!(
        broken(&outcome, "climbs"),
        (
            vec!["floor = 1".to_owned(), "doors = false".to_owned()],
            vec!["up  floor: 1 -> 2".to_owned()],
            "step 2, `up`, can't happen here"
        ),
        "from the first start where the second `up` can't happen"
    );
}

#[test]
fn an_expectation_that_is_false_at_the_end_breaks_the_witness() {
    let Lift { sketch, floor, .. } = lift();
    let arrives = witness("arrives", vec![played(UP)], is(&floor, 1));
    let outcome = searched(sketch, vec![arrives]);
    assert_eq!(
        broken(&outcome, "arrives"),
        (
            vec!["floor = 1".to_owned(), "doors = false".to_owned()],
            vec!["up  floor: 1 -> 2".to_owned()],
            "what the witness expects is false here"
        ),
        "every step was taken and the floor is 2"
    );
}

#[test]
fn the_path_comes_from_the_first_start_state_that_breaks_the_witness() {
    let Lift { sketch, floor, .. } = lift();
    let grounded = witness("grounded", Vec::new(), is(&floor, 0));
    let outcome = searched(sketch, vec![grounded]);
    assert_eq!(
        broken(&outcome, "grounded"),
        (
            vec!["floor = 1".to_owned(), "doors = false".to_owned()],
            Vec::new(),
            "what the witness expects is false here"
        ),
        "floor 1 comes before floor 2, and a witness without steps has an empty path"
    );
}

#[test]
fn a_broken_witness_makes_the_model_fail() {
    let Lift {
        sketch,
        floor,
        doors,
    } = lift();
    let rides = witness("rides", vec![played(OPEN), played(SHUT)], not(&doors));
    let grounded = witness("grounded", vec![played(DOWN)], is(&floor, 0));
    let outcome = searched(sketch, vec![rides, grounded]);
    assert_eq!(outcome.status, Status::Fail, "a witness is broken");
    assert!(
        outcome.findings.is_empty(),
        "a broken witness is no finding: {:?}",
        outcome.findings
    );
    let ids: Vec<&str> = outcome.rules.iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(ids, ["rides", "grounded"], "the witnesses in order");
    let (start, _, reason) = broken(&outcome, "grounded");
    assert_eq!(
        (start, reason),
        (
            vec!["floor = 0".to_owned(), "doors = false".to_owned()],
            "step 1, `down`, can't happen here"
        ),
        "the ground floor is the first start"
    );
}

#[test]
fn the_witnesses_are_listed_after_the_rules() {
    let Lift {
        mut sketch, floor, ..
    } = lift();
    sketch.rule("low", below(&floor, 3));
    let grounded = witness("grounded", Vec::new(), yes());
    let outcome = searched(sketch, vec![grounded]);
    let kinds: Vec<(&str, RuleKind)> = outcome
        .rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule.kind))
        .collect();
    assert_eq!(
        kinds,
        [("low", RuleKind::Always), ("grounded", RuleKind::Witness)],
        "the rule first, then the witness"
    );
}

#[test]
fn a_step_with_inputs_shows_them_in_the_path() {
    let mut sketch = Sketch::new();
    let key = sketch.text("key");
    let mouse = sketch.text("mouse");
    let domain = Domain::Optional(Box::new(Domain::OneOf(vec![key.clone(), mouse.clone()])));
    let last = sketch.field("last", domain, vec![Value::Null]);
    sketch.action_with(
        "press",
        &[("by", vec![key, mouse.clone()])],
        yes(),
        &[(&last, Expr::Local(0))],
    );
    let pressed = witness(
        "pressed",
        vec![Played {
            action: 0,
            input: vec![mouse],
        }],
        no(),
    );
    let outcome = searched(sketch, vec![pressed]);
    let (_, path, _) = broken(&outcome, "pressed");
    assert_eq!(
        path,
        ["press  by = \"mouse\"  last: null -> \"mouse\""],
        "the input given and the change it made"
    );
}

#[test]
fn a_step_that_takes_a_field_out_of_range_is_in_the_path() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 1, &[0]);
    sketch.action("grow", yes(), &[(&n, plus(&n, 5))]);
    let grows = witness("grows", vec![played(0), played(0)], yes());
    let outcome = searched(sketch, vec![grows]);
    assert_eq!(
        broken(&outcome, "grows"),
        (
            vec!["n = 0".to_owned()],
            vec!["grow  n: 0 -> 5".to_owned()],
            "step 1, `grow`, takes the field `n` out of the values it declares"
        ),
        "the step that left the range is the last of the path"
    );
}

#[test]
fn a_fault_in_the_condition_of_a_step_breaks_the_witness() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", -1, 1, &[0]);
    sketch.action("drop", yes(), &[(&n, plus(&n, -1))]);
    let even = apply(
        Op::Equal,
        &apply(Op::Math(Math::Remainder), &n, int(2)),
        int(0),
    );
    sketch.action("fill", even, &[(&n, plus(&n, 1))]);
    let refills = witness("refills", vec![played(0), played(1)], yes());
    let outcome = searched(sketch, vec![refills]);
    assert_eq!(
        broken(&outcome, "refills"),
        (
            vec!["n = 0".to_owned()],
            vec!["drop  n: 0 -> -1".to_owned()],
            "step 2: the condition of the action `fill` takes a remainder with a negative \
             number or a divisor below 1 here"
        ),
        "the path stops before the step whose condition faults"
    );
}

#[test]
fn a_fault_in_the_change_of_a_step_breaks_the_witness() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, i64::MAX, &[i64::MAX]);
    sketch.action("inc", yes(), &[(&n, plus(&n, 1))]);
    let counts = witness("counts", vec![played(0)], yes());
    let outcome = searched(sketch, vec![counts]);
    let (_, path, reason) = broken(&outcome, "counts");
    assert_eq!(path, Vec::<String>::new(), "no step was taken");
    assert_eq!(
        reason, "step 1: the change of the action `inc` computes a number too large to hold here",
        "the change that faults"
    );
}

#[test]
fn a_fault_in_what_the_witness_expects_breaks_it() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", -1, 1, &[0]);
    sketch.action("drop", below(&n, 1), &[(&n, plus(&n, -1))]);
    let even = apply(
        Op::Equal,
        &apply(Op::Math(Math::Remainder), &n, int(2)),
        int(0),
    );
    let drops = witness("drops", vec![played(0)], even);
    let outcome = searched(sketch, vec![drops]);
    assert_eq!(
        broken(&outcome, "drops"),
        (
            vec!["n = 0".to_owned()],
            vec!["drop  n: 0 -> -1".to_owned()],
            "what the witness expects takes a remainder with a negative number or a divisor \
             below 1 here"
        ),
        "every step was taken, and the condition faults in the last state"
    );
}

#[test]
fn more_start_states_than_the_limit_leave_a_witness_unknown() {
    let limit = |max_states: usize| Limits {
        max_states,
        ..Limits::default()
    };
    let Lift { sketch, doors, .. } = lift();
    let rides = || witness("rides", vec![played(OPEN), played(SHUT)], not(&doors));
    let model = with_witnesses(sketch, vec![rides()]);
    let at = |max_states: usize| {
        let outcome = explore(&model, &limit(max_states));
        let found = the_witness(&outcome, "rides").clone();
        (found.verdict, found.path, found.reason)
    };
    assert_eq!(
        at(2),
        (Verdict::Unknown, None, None),
        "three start states and a limit of two"
    );
    assert_eq!(
        at(3),
        (Verdict::Holds, None, None),
        "as many start states as the limit"
    );
}

#[test]
fn a_start_state_within_the_limit_still_breaks_a_witness() {
    let Lift { sketch, floor, .. } = lift();
    let grounded = witness("grounded", Vec::new(), is(&floor, 0));
    let limits = Limits {
        max_states: 2,
        ..Limits::default()
    };
    let outcome = explore(&with_witnesses(sketch, vec![grounded]), &limits);
    let (start, _, _) = broken(&outcome, "grounded");
    assert_eq!(
        start,
        ["floor = 1", "doors = false"],
        "the second start state is within the limit and breaks it"
    );
}

#[test]
fn checking_only_one_witness_or_one_rule_leaves_the_others_out() {
    let Lift {
        mut sketch,
        floor,
        doors,
    } = lift();
    sketch.rule("low", below(&floor, 3));
    let model = with_witnesses(
        sketch,
        vec![
            witness("rides", vec![played(OPEN), played(SHUT)], not(&doors)),
            witness("grounded", Vec::new(), is(&floor, 0)),
        ],
    );
    let only = |id: &str| {
        let limits = Limits {
            only_rule: Some(id.to_owned()),
            ..Limits::default()
        };
        let outcome = explore(&model, &limits);
        let ids: Vec<String> = outcome.rules.iter().map(|rule| rule.id.clone()).collect();
        (ids, outcome.status)
    };
    assert_eq!(
        only("grounded"),
        (vec!["grounded".to_owned()], Status::Fail),
        "only the broken witness"
    );
    assert_eq!(
        only("rides"),
        (vec!["rides".to_owned()], Status::Pass),
        "only the witness that holds"
    );
    assert_eq!(
        only("low"),
        (vec!["low".to_owned()], Status::Pass),
        "only the rule, so the broken witness is not played"
    );
}
