use crate::build::{
    HYPERCUBE_STATES, Moves, Sketch, apply, at_point, below, counter, hypercube_sketch, int, is,
    not, plus, steps, texts, yes,
};
use earnest_check::{
    FindingKind, Limits, Outcome, RuleKind, RuleOutcome, Status, Verdict, explore,
};
use earnest_ir::{Code, Eventually, Expr, Math, Model, Op};
use earnest_value::Value;
use std::time::Duration;

const STAYS: &str = "nothing changes the state from here, so the run can stay in it forever";

fn no() -> Expr {
    Expr::Const(Value::Bool(false))
}

fn rule(id: &str, after: Option<Expr>, reaches: Expr) -> Eventually {
    Eventually {
        id: id.to_owned(),
        says: format!("{id} comes"),
        line: 1,
        after: after.map(Code::new),
        reaches: Code::new(reaches),
        slots: 0,
    }
}

fn with_rules(sketch: Sketch, fair: &[usize], rules: Vec<Eventually>) -> Model {
    let mut model = sketch.model();
    model.runs.fair = fair.to_vec();
    model.runs.eventually = rules;
    model
}

fn searched(sketch: Sketch, fair: &[usize], rules: Vec<Eventually>) -> Outcome {
    explore(&with_rules(sketch, fair, rules), &Limits::default())
}

fn the_rule<'a>(outcome: &'a Outcome, id: &str) -> &'a RuleOutcome {
    outcome
        .rules
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("no outcome for {id} in {:?}", outcome.rules))
}

fn verdict(outcome: &Outcome, id: &str) -> Verdict {
    let found = the_rule(outcome, id);
    assert_eq!(found.kind, RuleKind::Eventually, "{id} is about runs");
    if found.verdict != Verdict::Broken {
        assert_eq!(found.path, None, "{id}: only a broken rule has a path");
        assert_eq!(found.reason, None, "{id}: only a broken rule has a reason");
        assert_eq!(found.repeats_from, None, "{id}: only a loop repeats");
    }
    found.verdict
}

#[derive(Debug, PartialEq, Eq)]
struct Broken {
    start: Vec<String>,
    steps: Vec<String>,
    repeats_from: Option<usize>,
    reason: String,
}

fn broken(outcome: &Outcome, id: &str) -> Broken {
    let found = the_rule(outcome, id);
    assert_eq!(found.verdict, Verdict::Broken, "{id} is broken: {found:?}");
    assert_eq!(found.kind, RuleKind::Eventually, "{id} is about runs");
    let path = found
        .path
        .as_ref()
        .unwrap_or_else(|| panic!("{id} has a path: {found:?}"));
    Broken {
        start: texts(&path.start),
        steps: steps(path),
        repeats_from: found.repeats_from,
        reason: found
            .reason
            .clone()
            .unwrap_or_else(|| panic!("{id} has a reason: {found:?}")),
    }
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).to_owned()).collect()
}

fn ring() -> (Sketch, Expr) {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 3, &[0]);
    sketch.action("step", below(&n, 2), &[(&n, plus(&n, 1))]);
    sketch.action("wrap", is(&n, 2), &[(&n, int(0))]);
    sketch.action("leave", is(&n, 1), &[(&n, int(3))]);
    sketch.end("out", is(&n, 3));
    (sketch, n)
}

struct Worker {
    sketch: Sketch,
    done: Expr,
}

const FINISH: usize = 1;

fn worker(finish_needs_busy: bool) -> Worker {
    let mut sketch = Sketch::new();
    let busy = sketch.boolean("busy", &[false]);
    let done = sketch.boolean("done", &[false]);
    let working = not(&done);
    sketch.action("fidget", working.clone(), &[(&busy, not(&busy))]);
    let can_finish = if finish_needs_busy {
        Expr::And(Box::new(busy.clone()), Box::new(working))
    } else {
        working
    };
    sketch.action("finish", can_finish, &[(&done, yes())]);
    sketch.end("over", done.clone());
    Worker { sketch, done }
}

fn divided(left: Expr, right: &Expr) -> Expr {
    apply(Op::Math(Math::Divide), &left, right.clone())
}

#[test]
fn a_rule_every_run_reaches_holds() {
    let (mut sketch, n) = counter(3);
    sketch.end("top", is(&n, 3));
    let outcome = searched(sketch, &[], vec![rule("tops", None, is(&n, 3))]);
    assert_eq!(verdict(&outcome, "tops"), Verdict::Holds, "`inc` runs to 3");
    assert_eq!(
        the_rule(&outcome, "tops").says,
        "tops comes",
        "the sentence"
    );
    assert_eq!(outcome.status, Status::Pass, "nothing is broken");
    assert_eq!(
        outcome.states, 4,
        "0 to 3: the rule does not change the count"
    );
}

#[test]
fn the_state_a_rule_starts_in_may_be_the_one_it_reaches() {
    let (mut sketch, n) = counter(3);
    sketch.end("top", is(&n, 3));
    let outcome = searched(
        sketch,
        &[],
        vec![
            rule("now", None, is(&n, 0)),
            rule("there", Some(is(&n, 2)), below(&n, 3)),
        ],
    );
    assert_eq!(
        verdict(&outcome, "now"),
        Verdict::Holds,
        "the start state has `n = 0`"
    );
    assert_eq!(
        verdict(&outcome, "there"),
        Verdict::Holds,
        "`n = 2` is below 3 itself"
    );
}

#[test]
fn a_loop_that_never_reaches_breaks_the_rule_with_the_path_once_round_it() {
    let (sketch, n) = ring();
    let outcome = searched(sketch, &[], vec![rule("leaves", None, is(&n, 3))]);
    assert_eq!(
        broken(&outcome, "leaves"),
        Broken {
            start: owned(&["n = 0"]),
            steps: owned(&["step  n: 0 -> 1", "step  n: 1 -> 2", "wrap  n: 2 -> 0"]),
            repeats_from: Some(0),
            reason: "steps 1 to 3 can repeat forever".to_owned(),
        },
        "the start is in the loop `step`, `step`, `wrap`, which never leaves"
    );
    assert_eq!(
        outcome.status,
        Status::Fail,
        "a broken rule fails the model"
    );
    assert_eq!(outcome.findings, [], "a broken rule is no finding");
}

#[test]
fn a_rule_with_after_starts_in_each_reachable_state_where_after_is_true() {
    let (sketch, n) = ring();
    let outcome = searched(
        sketch,
        &[],
        vec![
            rule("fromTwo", Some(is(&n, 2)), is(&n, 1)),
            rule("fromOne", Some(is(&n, 1)), is(&n, 0)),
            rule("never", Some(is(&n, 9)), no()),
        ],
    );
    assert_eq!(
        verdict(&outcome, "fromTwo"),
        Verdict::Holds,
        "from `n = 2` only `wrap` to 0, and from 0 only `step` to 1"
    );
    assert_eq!(
        broken(&outcome, "fromOne"),
        Broken {
            start: owned(&["n = 0"]),
            steps: owned(&["step  n: 0 -> 1", "leave  n: 1 -> 3"]),
            repeats_from: None,
            reason: STAYS.to_owned(),
        },
        "the shortest way to `n = 1`, then `leave` to the end, which never comes back to 0"
    );
    assert_eq!(
        verdict(&outcome, "never"),
        Verdict::Holds,
        "a rule no state starts is never broken"
    );
}

#[test]
fn a_run_that_stops_in_a_stuck_state_breaks_the_rule_and_the_state_is_stuck() {
    let (sketch, n) = counter(2);
    let outcome = searched(sketch, &[], vec![rule("past", None, is(&n, 3))]);
    assert_eq!(
        broken(&outcome, "past"),
        Broken {
            start: owned(&["n = 0"]),
            steps: owned(&["inc  n: 0 -> 1", "inc  n: 1 -> 2"]),
            repeats_from: None,
            reason: STAYS.to_owned(),
        },
        "no action can happen at `n = 2`"
    );
    let kinds: Vec<FindingKind> = outcome.findings.iter().map(|found| found.kind).collect();
    assert_eq!(
        kinds,
        [FindingKind::Stuck],
        "the stuck state is still found"
    );
}

#[test]
fn an_action_that_keeps_the_state_is_no_step_so_the_run_can_stay() {
    let (mut sketch, n) = counter(3);
    sketch.action("hold", is(&n, 3), &[(&n, n.clone())]);
    let outcome = searched(
        sketch,
        &[2],
        vec![rule("resets", Some(is(&n, 3)), is(&n, 0))],
    );
    assert_eq!(
        broken(&outcome, "resets"),
        Broken {
            start: owned(&["n = 0"]),
            steps: owned(&["inc  n: 0 -> 1", "inc  n: 1 -> 2", "inc  n: 2 -> 3"]),
            repeats_from: None,
            reason: STAYS.to_owned(),
        },
        "`hold` can happen at `n = 3`, even fairly, but changes nothing"
    );
    assert_eq!(
        outcome.findings,
        [],
        "`hold` can happen, so `n = 3` is not stuck"
    );
}

#[test]
fn a_loop_that_only_puts_off_a_fair_action_breaks_no_rule() {
    let Worker { sketch, done } = worker(false);
    let fair = searched(sketch, &[FINISH], vec![rule("finishes", None, done)]);
    assert_eq!(
        verdict(&fair, "finishes"),
        Verdict::Holds,
        "`finish` is possible in both states of the `fidget` loop and fair"
    );
    let Worker { sketch, done } = worker(false);
    let lazy = searched(sketch, &[], vec![rule("finishes", None, done)]);
    assert_eq!(
        broken(&lazy, "finishes"),
        Broken {
            start: owned(&["busy = false", "done = false"]),
            steps: owned(&["fidget  busy: false -> true", "fidget  busy: true -> false"]),
            repeats_from: Some(0),
            reason: "steps 1 to 2 can repeat forever".to_owned(),
        },
        "without fairness the loop is a run"
    );
}

#[test]
fn a_fair_action_that_is_not_possible_in_every_state_of_a_loop_may_be_put_off() {
    let Worker { sketch, done } = worker(true);
    let outcome = searched(sketch, &[FINISH], vec![rule("finishes", None, done)]);
    let found = broken(&outcome, "finishes");
    assert_eq!(
        found.steps,
        owned(&["fidget  busy: false -> true", "fidget  busy: true -> false"]),
        "`finish` can't happen at `busy = false`, so weak fairness asks nothing of the loop"
    );
    assert_eq!(found.repeats_from, Some(0), "the loop is the whole path");
}

#[test]
fn a_loop_with_two_fair_actions_shows_each_one_taken() {
    let mut sketch = Sketch::new();
    let a = sketch.boolean("a", &[false]);
    let b = sketch.boolean("b", &[false]);
    let never = sketch.boolean("never", &[false]);
    sketch.action("flipA", yes(), &[(&a, not(&a))]);
    sketch.action("flipB", yes(), &[(&b, not(&b))]);
    let outcome = searched(sketch, &[0, 1], vec![rule("lands", None, never)]);
    let found = broken(&outcome, "lands");
    assert_eq!(
        found,
        Broken {
            start: owned(&["a = false", "b = false", "never = false"]),
            steps: owned(&[
                "flipA  a: false -> true",
                "flipB  b: false -> true",
                "flipB  b: true -> false",
                "flipA  a: true -> false",
            ]),
            repeats_from: Some(0),
            reason: "steps 1 to 4 can repeat forever".to_owned(),
        },
        "both fair actions stay possible, so the loop takes each, then comes home"
    );
}

#[test]
fn a_loop_with_two_fair_actions_shows_a_state_where_the_other_can_not_happen() {
    let mut sketch = Sketch::new();
    let a = sketch.boolean("a", &[false]);
    let b = sketch.boolean("b", &[false]);
    let never = sketch.boolean("never", &[false]);
    sketch.action("flip", yes(), &[(&a, not(&a))]);
    sketch.action("lock", not(&a), &[(&b, not(&b))]);
    let outcome = searched(sketch, &[0, 1], vec![rule("lands", None, never)]);
    let found = broken(&outcome, "lands");
    assert_eq!(
        found.steps,
        owned(&["flip  a: false -> true", "flip  a: true -> false"]),
        "`flip` is taken, and `lock` can't happen at `a = true`"
    );
    assert_eq!(
        found.reason, "steps 1 to 2 can repeat forever",
        "the loop is both steps"
    );
}

#[test]
fn a_fair_action_with_inputs_is_fair_as_a_whole() {
    let mut sketch = Sketch::new();
    let front = sketch.text("front");
    let back = sketch.text("back");
    let nowhere = Expr::Const(Value::Null);
    let desk = sketch.field(
        "desk",
        earnest_value::Domain::OneOf(vec![Value::Null, front.clone(), back.clone()]),
        vec![Value::Null],
    );
    let tick = sketch.boolean("tick", &[false]);
    let waiting = Expr::Apply(Op::Equal, vec![desk.clone(), nowhere]);
    sketch.action("idle", waiting.clone(), &[(&tick, not(&tick))]);
    sketch.action_with(
        "route",
        &[("to", vec![front, back.clone()])],
        waiting.clone(),
        &[(&desk, Expr::Local(0))],
    );
    sketch.end("sent", not(&waiting));
    let at_back = Expr::Apply(Op::Equal, vec![desk, Expr::Const(back)]);
    let outcome = searched(
        sketch,
        &[1],
        vec![
            rule("routed", None, not(&waiting)),
            rule("toTheBack", None, at_back),
        ],
    );
    assert_eq!(
        verdict(&outcome, "routed"),
        Verdict::Holds,
        "the `idle` loop puts off `route`"
    );
    assert_eq!(
        broken(&outcome, "toTheBack"),
        Broken {
            start: owned(&["desk = null", "tick = false"]),
            steps: owned(&["route  to = \"front\"  desk: null -> \"front\""]),
            repeats_from: None,
            reason: STAYS.to_owned(),
        },
        "fairness takes some input, not each: `front` is enough"
    );
    assert_eq!(outcome.states, 6, "`desk` × `tick`");
}

#[test]
fn rules_about_runs_are_listed_after_the_rules_in_their_order() {
    let (mut sketch, n) = counter(3);
    sketch.end("top", is(&n, 3));
    sketch.rule("low", below(&n, 4));
    let outcome = searched(
        sketch,
        &[],
        vec![
            rule("second", None, is(&n, 3)),
            rule("first", None, is(&n, 2)),
        ],
    );
    let listed: Vec<(&str, RuleKind)> = outcome
        .rules
        .iter()
        .map(|found| (found.id.as_str(), found.kind))
        .collect();
    assert_eq!(
        listed,
        [
            ("low", RuleKind::Always),
            ("second", RuleKind::Eventually),
            ("first", RuleKind::Eventually),
        ],
        "the rule, then the rules about runs"
    );
}

#[test]
fn one_rule_about_runs_is_checked_alone_by_its_id() {
    let (sketch, n) = ring();
    let model = with_rules(
        sketch,
        &[],
        vec![
            rule("leaves", None, is(&n, 3)),
            rule("starts", None, is(&n, 0)),
        ],
    );
    let only = |id: &str| Limits {
        only_rule: Some(id.to_owned()),
        ..Limits::default()
    };
    let leaves = explore(&model, &only("leaves"));
    let ids: Vec<&str> = leaves.rules.iter().map(|found| found.id.as_str()).collect();
    assert_eq!(ids, ["leaves"], "only the rule asked for");
    assert_eq!(
        broken(&leaves, "leaves"),
        broken(&explore(&model, &Limits::default()), "leaves"),
        "the same result as when every rule is checked"
    );
    assert_eq!(leaves.graph, None, "no graph was asked for");
    let none = explore(&model, &only("missing"));
    assert_eq!(none.rules, [], "an id no rule has checks no rule");
}

#[test]
fn a_caller_that_keeps_the_graph_still_gets_it_and_one_that_does_not_gets_none() {
    let (sketch, n) = ring();
    let model = with_rules(sketch, &[], vec![rule("leaves", None, is(&n, 3))]);
    let plain = explore(&model, &Limits::default());
    assert_eq!(
        plain.graph, None,
        "the graph kept for the rule is not handed out"
    );
    assert_eq!(plain.outline, None, "nor an outline");
    let kept = explore(
        &model,
        &Limits {
            keep_graph: true,
            ..Limits::default()
        },
    );
    let graph = kept.graph.as_ref().expect("the graph asked for");
    assert_eq!(graph.nodes.len(), 4, "every state is a node");
    assert_eq!(
        kept.rules, plain.rules,
        "keeping the graph does not change the verdicts"
    );
}

#[test]
fn a_search_stopped_by_a_limit_leaves_every_rule_about_runs_unknown() {
    let (sketch, n) = ring();
    let model = with_rules(
        sketch,
        &[],
        vec![
            rule("leaves", None, is(&n, 3)),
            rule("starts", None, is(&n, 0)),
        ],
    );
    let short = explore(
        &model,
        &Limits {
            max_states: 2,
            ..Limits::default()
        },
    );
    assert_eq!(short.status, Status::Unfinished, "the search stopped");
    for id in ["leaves", "starts"] {
        assert_eq!(
            verdict(&short, id),
            Verdict::Unknown,
            "{id}: a part of the graph proves nothing"
        );
    }
    let timed = explore(
        &model,
        &Limits {
            time_limit: Some(Duration::ZERO),
            ..Limits::default()
        },
    );
    assert_eq!(timed.status, Status::Unfinished, "out of time");
    assert_eq!(
        verdict(&timed, "starts"),
        Verdict::Unknown,
        "even the rule the start meets"
    );
}

#[test]
fn a_state_out_of_range_leaves_a_rule_about_runs_unknown() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 2, &[0]);
    sketch.action("up", yes(), &[(&n, plus(&n, 1))]);
    let outcome = searched(sketch, &[], vec![rule("top", None, is(&n, 2))]);
    assert_eq!(
        verdict(&outcome, "top"),
        Verdict::Unknown,
        "`n = 2` is reached, but the search stopped at `n = 3`"
    );
    let kinds: Vec<FindingKind> = outcome.findings.iter().map(|found| found.kind).collect();
    assert_eq!(kinds, [FindingKind::OutOfRange], "the state out of range");
}

#[test]
fn an_action_that_faults_leaves_a_rule_about_runs_unknown() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 2, &[2]);
    sketch.action("halve", yes(), &[(&n, divided(int(2), &n))]);
    sketch.action("down", not(&is(&n, 0)), &[(&n, plus(&n, -1))]);
    let outcome = searched(
        sketch,
        &[],
        vec![
            rule("zero", None, is(&n, 0)),
            rule("whole", None, is(&divided(int(2), &n), 1)),
        ],
    );
    assert_eq!(
        verdict(&outcome, "zero"),
        Verdict::Unknown,
        "`halve` faults at `n = 0`"
    );
    assert_eq!(
        verdict(&outcome, "whole"),
        Verdict::Unknown,
        "the rule faults at `n = 0` too"
    );
    let faults: Vec<&str> = outcome
        .findings
        .iter()
        .filter(|found| found.kind == FindingKind::Fault)
        .map(|found| found.message.as_str())
        .collect();
    assert_eq!(faults.len(), 1, "one fault: {faults:?}");
    assert!(
        faults.iter().all(|message| message.contains("`halve`")),
        "the action's fault was found first, so the rule's is not added: {faults:?}"
    );
}

#[test]
fn a_rule_about_runs_that_faults_is_unknown_and_named_in_a_fault() {
    let (mut sketch, n) = counter(2);
    sketch.end("top", is(&n, 2));
    let outcome = searched(
        sketch,
        &[],
        vec![
            rule("low", None, below(&n, 2)),
            rule("halves", None, is(&divided(int(2), &n), 2)),
            rule("thirds", Some(is(&divided(int(3), &n), 1)), yes()),
        ],
    );
    assert_eq!(verdict(&outcome, "low"), Verdict::Holds, "judged as usual");
    assert_eq!(
        verdict(&outcome, "halves"),
        Verdict::Unknown,
        "`2 / 0` has no value at the start"
    );
    assert_eq!(
        verdict(&outcome, "thirds"),
        Verdict::Unknown,
        "nor has `3 / 0` in `after`"
    );
    assert_eq!(outcome.status, Status::Fail, "a fault fails the model");
    let faults: Vec<(&str, usize)> = outcome
        .findings
        .iter()
        .map(|found| (found.message.as_str(), found.path.steps.len()))
        .collect();
    assert_eq!(
        faults,
        [(
            "the rule `halves` takes a division with a negative number or a divisor below 1 in \
             this state",
            0
        )],
        "one fault, for the first rule that faulted, with the path to its state"
    );
}

#[test]
fn a_model_without_rules_about_runs_keeps_no_graph_and_reports_as_before() {
    let (mut sketch, n) = counter(3);
    sketch.end("top", is(&n, 3));
    sketch.rule("low", below(&n, 4));
    let outcome = sketch.explored();
    assert_eq!(outcome.graph, None, "nothing kept");
    let kinds: Vec<RuleKind> = outcome.rules.iter().map(|found| found.kind).collect();
    assert_eq!(kinds, [RuleKind::Always], "the rule alone");
}

#[test]
fn the_outcome_is_the_same_on_one_thread_and_on_four() {
    for moves in [Moves::Up, Moves::UpAndDown] {
        let (sketch, axes) = hypercube_sketch(9, moves);
        let model = with_rules(
            sketch,
            &[],
            vec![
                rule("corner", None, at_point(&axes, [9, 9, 9, 9, 9])),
                rule(
                    "back",
                    Some(at_point(&axes, [9, 0, 0, 0, 0])),
                    at_point(&axes, [0, 0, 0, 0, 0]),
                ),
            ],
        );
        let alone = explore(&model, &Limits::default());
        let four = explore(
            &model,
            &Limits {
                jobs: 4,
                ..Limits::default()
            },
        );
        assert_eq!(alone.states, HYPERCUBE_STATES, "every point of the cube");
        assert_eq!(four.states, alone.states, "the same states");
        assert_eq!(
            four.rules, alone.rules,
            "the same verdicts, paths and loops"
        );
        assert_eq!(four.findings, alone.findings, "the same findings");
    }
}

#[test]
fn going_only_up_every_run_reaches_the_far_corner_and_going_both_ways_one_need_not() {
    let (up, axes) = hypercube_sketch(9, Moves::Up);
    let only_up = searched(
        up,
        &[],
        vec![rule("corner", None, at_point(&axes, [9, 9, 9, 9, 9]))],
    );
    assert_eq!(
        verdict(&only_up, "corner"),
        Verdict::Holds,
        "each run goes up until no action can, at the corner"
    );
    let (both, axes) = hypercube_sketch(9, Moves::UpAndDown);
    let both_ways = searched(
        both,
        &[],
        vec![rule("corner", None, at_point(&axes, [9, 9, 9, 9, 9]))],
    );
    let found = broken(&both_ways, "corner");
    assert_eq!(
        found.steps,
        owned(&["upA  a: 0 -> 1", "downA  a: 1 -> 0"]),
        "up and back down from the start"
    );
    assert_eq!(found.repeats_from, Some(0), "the start is in the loop");
}
