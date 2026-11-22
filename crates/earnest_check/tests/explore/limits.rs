use crate::build::{
    CUBE_TOP, Moves, Sketch, apply, at_corner, below, both, counter, cube_sketch, int, is, is_not,
    not, plus, steps, texts, yes,
};
use earnest_check::{FindingKind, Helpers, Limits, Outcome, Status, Verdict, explore};
use earnest_ir::{Expr, Math, Model, Op, Order, Shelf, Sweep};
use earnest_value::{Domain, Value};
use std::time::{Duration, Instant};

fn long_counter() -> Model {
    let (mut sketch, n) = counter(99);
    sketch.end("full", is(&n, 99));
    sketch.rule("neverFive", is_not(&n, 5));
    sketch.rule("neverFifty", is_not(&n, 50));
    sketch.model()
}

fn with_states(max_states: usize) -> Limits {
    Limits {
        max_states,
        ..Limits::default()
    }
}

fn with_jobs(jobs: usize) -> Limits {
    Limits {
        jobs,
        keep_graph: true,
        ..Limits::default()
    }
}

fn wide_level() -> Model {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..2000).collect();
    let n = sketch.number("n", 0, 1999, &every_start);
    let flipped = sketch.boolean("flipped", &[false]);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    sketch.end("flipped", flipped.clone());
    sketch.rule("not1234", not(&both(flipped, is(&n, 1234))));
    sketch.model()
}

fn cube() -> Model {
    let mut sketch = Sketch::new();
    let axes: Vec<Expr> = ["x", "y", "z"]
        .iter()
        .map(|name| sketch.number(name, 0, 29, &[0]))
        .collect();
    for (name, axis) in ["moveX", "moveY", "moveZ"].iter().zip(&axes) {
        sketch.action(name, below(axis, 29), &[(axis, plus(axis, 1))]);
    }
    let at = |x: i64, y: i64, z: i64| both(is(&axes[0], x), both(is(&axes[1], y), is(&axes[2], z)));
    sketch.end("corner", at(29, 29, 29));
    sketch.rule("notThere", not(&at(10, 20, 5)));
    sketch.model()
}

fn unfinished_at(outcome: &Outcome, states: usize, reason: &str) {
    assert_eq!(outcome.status, Status::Unfinished, "{:?}", outcome.reason);
    assert_eq!(outcome.states, states, "the count so far");
    assert_eq!(outcome.reason.as_deref(), Some(reason), "why it stopped");
    assert!(outcome.findings.is_empty(), "{:?}", outcome.findings);
}

#[test]
fn the_default_limits_are_five_million_states_no_time_limit_and_every_rule() {
    let limits = Limits::default();
    assert_eq!(limits.max_states, 5_000_000);
    assert_eq!(limits.time_limit, None);
    assert_eq!(limits.only_rule, None);
    assert!(!limits.keep_graph);
}

#[test]
fn the_state_limit_stops_the_search_with_the_count_so_far() {
    let outcome = explore(&long_counter(), &with_states(3));
    unfinished_at(&outcome, 3, "state limit of 3");
}

#[test]
fn a_rule_not_seen_broken_before_the_limit_is_unknown() {
    let outcome = explore(&long_counter(), &with_states(3));
    assert_eq!(outcome.rules[0].verdict, Verdict::Unknown);
    assert_eq!(outcome.rules[1].verdict, Verdict::Unknown);
    assert_eq!(outcome.rules[0].path, None);
}

#[test]
fn a_rule_seen_broken_before_the_limit_fails_the_model() {
    let outcome = explore(&long_counter(), &with_states(20));
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.states, 20);
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 20"));
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
    assert_eq!(outcome.rules[1].verdict, Verdict::Unknown);
    let path = outcome.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(path.steps.len(), 5);
}

#[test]
fn a_state_limit_equal_to_the_number_of_states_does_not_stop_the_search() {
    let (mut sketch, n) = counter(3);
    sketch.end("full", is(&n, 3));
    let model = sketch.model();
    assert_eq!(explore(&model, &with_states(4)).status, Status::Pass);
    unfinished_at(&explore(&model, &with_states(3)), 3, "state limit of 3");
}

#[test]
fn the_state_limit_also_bounds_the_start_states() {
    unfinished_at(
        &explore(&wide_level(), &with_states(100)),
        100,
        "state limit of 100",
    );
}

#[test]
fn a_time_limit_that_has_already_passed_stops_the_search_at_the_start_states() {
    let limits = Limits {
        time_limit: Some(Duration::ZERO),
        ..Limits::default()
    };
    let outcome = explore(&long_counter(), &limits);
    unfinished_at(&outcome, 1, "time limit of 0 s");
    assert_eq!(outcome.rules[0].verdict, Verdict::Unknown);
}

#[test]
fn a_time_limit_that_has_already_passed_stops_the_worker_threads_too() {
    let limits = Limits {
        time_limit: Some(Duration::ZERO),
        jobs: 4,
        ..Limits::default()
    };
    unfinished_at(&explore(&wide_level(), &limits), 2000, "time limit of 0 s");
}

#[test]
fn a_time_limit_that_is_not_reached_changes_nothing() {
    let limits = Limits {
        time_limit: Some(Duration::from_hours(1)),
        ..Limits::default()
    };
    let outcome = explore(&long_counter(), &limits);
    assert_eq!(outcome.states, 100);
    assert_eq!(outcome.reason, None);
    assert_eq!(outcome.status, Status::Fail);
}

#[test]
fn a_level_of_two_thousand_states_gives_the_same_outcome_on_one_thread_and_on_four() {
    let model = wide_level();
    let single = explore(&model, &with_jobs(1));
    assert_eq!(single.states, 4000);
    assert_eq!(single.status, Status::Fail);
    let path = single.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(texts(&path.start), ["n = 1234", "flipped = false"]);
    assert_eq!(steps(path), ["flip  flipped: false -> true"]);
    assert_eq!(explore(&model, &with_jobs(4)), single);
    assert_eq!(explore(&model, &with_jobs(3)), single);
    assert_eq!(explore(&model, &with_jobs(64)), single);
}

#[test]
fn a_search_with_many_wide_levels_gives_the_same_outcome_on_one_thread_and_on_four() {
    let model = cube();
    let single = explore(&model, &with_jobs(1));
    assert_eq!(single.states, 27_000);
    assert_eq!(single.status, Status::Fail);
    let path = single.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(path.steps.len(), 35);
    assert!(single.findings.is_empty(), "{:?}", single.findings);
    assert_eq!(explore(&model, &with_jobs(4)), single);
}

#[test]
fn no_worker_threads_counts_as_one() {
    let model = wide_level();
    assert_eq!(
        explore(&model, &with_jobs(0)),
        explore(&model, &with_jobs(1))
    );
}

#[test]
fn the_state_limit_gives_the_same_outcome_on_one_thread_and_on_four() {
    let model = wide_level();
    let limited = |jobs: usize| Limits {
        max_states: 3000,
        ..with_jobs(jobs)
    };
    let single = explore(&model, &limited(1));
    assert_eq!(single.states, 3000);
    assert_eq!(single.reason.as_deref(), Some("state limit of 3000"));
    assert_eq!(explore(&model, &limited(4)), single);
}

const CUBE_STATES: usize = 27_000;
const CUBE_STATES_THROUGH_SUM_43: usize = 13_500;
const CUBE_STATES_THROUGH_SUM_42: usize = 12_825;

fn cube_with_a_late_rule() -> Model {
    let (mut sketch, axes) = cube_sketch(CUBE_TOP, Moves::Up);
    sketch.end("corner", at_corner(&axes, [29, 29, 29]));
    sketch.rule("notEarly", not(&at_corner(&axes, [10, 20, 5])));
    sketch.rule("notAtSum43", not(&at_corner(&axes, [20, 20, 3])));
    sketch.rule("notAtSum44", not(&at_corner(&axes, [20, 20, 4])));
    sketch.model()
}

const IDLE_HELPERS: usize = 3;

fn same_outcome_with_every_crew(model: &Model, base: &Limits, worker_counts: &[usize]) -> Outcome {
    let with = |jobs: usize, helpers: Option<Helpers>, keep_graph: bool| Limits {
        jobs,
        helpers,
        keep_graph,
        ..base.clone()
    };
    let every_crew_gives = |expected: &Outcome, keep_graph: bool| {
        for jobs in worker_counts.iter().copied().chain([1]) {
            assert_eq!(
                &explore(model, &with(jobs, None, keep_graph)),
                expected,
                "{jobs} workers, graph kept: {keep_graph}"
            );
        }
        for idle in [IDLE_HELPERS, 0] {
            assert_eq!(
                &explore(model, &with(1, Some(Helpers::new(idle)), keep_graph)),
                expected,
                "one worker and {idle} idle helpers, graph kept: {keep_graph}"
            );
        }
    };
    let row_by_row = explore(model, &with(1, None, true));
    every_crew_gives(&row_by_row, true);
    every_crew_gives(
        &Outcome {
            graph: None,
            ..row_by_row.clone()
        },
        false,
    );
    row_by_row
}

fn same_outcome_on_every_worker_count(model: &Model, max_states: usize) -> Outcome {
    same_outcome_with_every_crew(model, &with_states(max_states), &[2, 4, 10])
}

const STARTS_OF_A_LEVEL_MERGED_ON_TEN_THREADS: i64 = 45_000;

fn a_level_merged_on_ten_threads() -> Model {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..STARTS_OF_A_LEVEL_MERGED_ON_TEN_THREADS).collect();
    let n = sketch.number(
        "n",
        0,
        STARTS_OF_A_LEVEL_MERGED_ON_TEN_THREADS - 1,
        &every_start,
    );
    let flipped = sketch.boolean("flipped", &[false]);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    sketch.end("flipped", flipped.clone());
    sketch.rule("not1234", not(&both(flipped, is(&n, 1234))));
    sketch.model()
}

#[test]
fn a_level_with_enough_new_states_to_merge_on_every_worker_gives_the_same_outcome_on_every_crew() {
    let outcome = same_outcome_on_every_worker_count(
        &a_level_merged_on_ten_threads(),
        Limits::default().max_states,
    );
    assert_eq!(outcome.states, 90_000);
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
}

#[test]
fn a_state_limit_at_the_end_of_a_level_gives_the_same_outcome_on_every_worker_count() {
    let outcome =
        same_outcome_on_every_worker_count(&cube_with_a_late_rule(), CUBE_STATES_THROUGH_SUM_43);
    assert_eq!(outcome.states, CUBE_STATES_THROUGH_SUM_43);
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 13500"));
    let verdicts: Vec<Verdict> = outcome.rules.iter().map(|rule| rule.verdict).collect();
    assert_eq!(
        verdicts,
        [Verdict::Broken, Verdict::Broken, Verdict::Unknown]
    );
}

#[test]
fn a_state_limit_inside_a_level_gives_the_same_outcome_on_every_worker_count() {
    let inside = CUBE_STATES_THROUGH_SUM_42 + 300;
    let outcome = same_outcome_on_every_worker_count(&cube_with_a_late_rule(), inside);
    assert_eq!(outcome.states, inside);
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
    assert_eq!(outcome.rules[2].verdict, Verdict::Unknown);
}

#[test]
fn a_state_limit_one_above_the_states_of_a_model_gives_the_same_outcome_on_every_worker_count() {
    let outcome = same_outcome_on_every_worker_count(&cube_with_a_late_rule(), CUBE_STATES + 1);
    assert_eq!(outcome.states, CUBE_STATES);
    assert_eq!(outcome.reason, None);
    let verdicts: Vec<Verdict> = outcome.rules.iter().map(|rule| rule.verdict).collect();
    assert_eq!(verdicts, [Verdict::Broken; 3]);
}

const STARTS_OF_A_LEVEL_OF_KNOWN_STEPS: usize = 2000;

fn a_level_whose_steps_mostly_reach_known_states() -> Model {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..2000).collect();
    let n = sketch.number("n", 0, 1999, &every_start);
    let flipped = sketch.boolean("flipped", &[false]);
    sketch.action("step", below(&n, 1999), &[(&n, plus(&n, 1))]);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    sketch.end("any", yes());
    sketch.rule("not1234", not(&both(flipped, is(&n, 1234))));
    sketch.model()
}

#[test]
fn a_state_limit_one_above_the_states_of_a_wide_level_of_known_steps_is_never_reached() {
    let states = 2 * STARTS_OF_A_LEVEL_OF_KNOWN_STEPS;
    let outcome = same_outcome_on_every_worker_count(
        &a_level_whose_steps_mostly_reach_known_states(),
        states + 1,
    );
    assert_eq!(outcome.states, states);
    assert_eq!(outcome.reason, None);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
}

fn start_states_zero_to_four() -> Model {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 4, &[0, 1, 2, 3, 4]);
    sketch.end("any", yes());
    sketch.rule("neverThree", is_not(&n, 3));
    sketch.model()
}

#[test]
fn a_start_state_counted_at_the_state_limit_has_its_rules_judged() {
    let outcome = explore(&start_states_zero_to_four(), &with_states(4));
    assert_eq!(outcome.states, 4, "the start states 0, 1, 2 and 3");
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 4"));
    assert_eq!(outcome.status, Status::Fail, "state 3 is counted");
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
    let path = outcome.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(texts(&path.start), ["n = 3"]);
    assert!(path.steps.is_empty(), "a start state is no step away");
}

#[test]
fn a_start_state_beyond_the_state_limit_is_neither_counted_nor_judged() {
    let outcome = explore(&start_states_zero_to_four(), &with_states(3));
    unfinished_at(&outcome, 3, "state limit of 3");
    assert_eq!(outcome.rules[0].verdict, Verdict::Unknown);
}

fn fork() -> Model {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 9, &[0]);
    sketch.action("step", is(&n, 0), &[(&n, plus(&n, 1))]);
    sketch.action("leap", is(&n, 0), &[(&n, plus(&n, 7))]);
    sketch.end("moved", not(&is(&n, 0)));
    sketch.rule("neverOne", is_not(&n, 1));
    sketch.rule("neverSeven", is_not(&n, 7));
    sketch.model()
}

#[test]
fn a_state_counted_just_before_the_state_limit_has_its_rules_judged() {
    let outcome = explore(&fork(), &with_states(2));
    assert_eq!(outcome.states, 2, "the start and the state after `step`");
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 2"));
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken, "1 is counted");
    assert_eq!(
        outcome.rules[1].verdict,
        Verdict::Unknown,
        "7 is the state the limit left out"
    );
    let path = outcome.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(steps(path), ["step  n: 0 -> 1"]);
    assert!(
        outcome.findings.is_empty(),
        "no action is tried from a state the search did not go on from: {:?}",
        outcome.findings
    );
}

#[test]
fn a_state_counted_before_the_time_limit_has_its_rules_judged() {
    let (mut sketch, n) = counter(3);
    sketch.rule("neverZero", is_not(&n, 0));
    let limits = Limits {
        time_limit: Some(Duration::ZERO),
        ..Limits::default()
    };
    let outcome = explore(&sketch.model(), &limits);
    assert_eq!(outcome.states, 1, "the start state");
    assert_eq!(outcome.reason.as_deref(), Some("time limit of 0 s"));
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
}

#[test]
fn the_states_judged_at_the_limit_are_the_same_on_one_thread_and_on_four() {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..2000).collect();
    let n = sketch.number("n", 0, 1999, &every_start);
    sketch.end("any", yes());
    sketch.rule("not1234", is_not(&n, 1234));
    let model = sketch.model();
    let limited = |jobs: usize| Limits {
        max_states: 1500,
        ..with_jobs(jobs)
    };
    let single = explore(&model, &limited(1));
    assert_eq!(single.states, 1500);
    assert_eq!(single.rules[0].verdict, Verdict::Broken, "1234 is counted");
    assert_eq!(explore(&model, &limited(4)), single);
}

fn every_of(count: i64, body: Expr) -> Expr {
    let items = Value::set((0..count).map(Value::Int).collect());
    Expr::Sweep {
        sweep: Sweep::Every,
        shelf: Shelf::Set,
        slot: 0,
        source: Box::new(Expr::Const(items)),
        body: Box::new(body),
    }
}

#[test]
fn a_time_limit_stops_a_rule_that_takes_billions_of_steps_in_one_state() {
    let (mut sketch, n) = counter(3);
    sketch.end("full", is(&n, 3));
    sketch.rule(
        "slow",
        every_of(2000, every_of(2000, every_of(2000, yes()))),
    );
    let limits = Limits {
        time_limit: Some(Duration::from_secs(1)),
        ..Limits::default()
    };
    let began = Instant::now();
    let outcome = explore(&sketch.model(), &limits);
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "8 * 10^9 steps would take minutes: {:?}",
        began.elapsed()
    );
    assert_eq!(outcome.status, Status::Unfinished);
    assert_eq!(outcome.reason.as_deref(), Some("time limit of 1 s"));
    assert_eq!(outcome.rules[0].verdict, Verdict::Unknown);
    assert_eq!(
        outcome.states, 0,
        "no state had its rules judged, so none is counted"
    );
}

fn huge_input_space() -> Model {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 999, &[0]);
    let thousand: Vec<Value> = (0..1000).map(Value::Int).collect();
    sketch.action_with(
        "pick",
        &[
            ("a", thousand.clone()),
            ("b", thousand.clone()),
            ("c", thousand),
        ],
        yes(),
        &[(&n, Expr::Local(0))],
    );
    sketch.end("any", yes());
    sketch.model()
}

#[test]
fn an_action_with_a_billion_input_combinations_stops_at_a_small_state_limit_at_once() {
    let began = Instant::now();
    let outcome = explore(&huge_input_space(), &with_states(10));
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "{:?}",
        began.elapsed()
    );
    assert_eq!(outcome.status, Status::Unfinished);
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 10"));
    assert_eq!(outcome.states, 10);
}

#[test]
fn an_action_with_a_billion_input_combinations_stops_at_the_time_limit() {
    let limits = Limits {
        time_limit: Some(Duration::from_secs(1)),
        ..Limits::default()
    };
    let began = Instant::now();
    let outcome = explore(&huge_input_space(), &limits);
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "{:?}",
        began.elapsed()
    );
    assert_eq!(outcome.status, Status::Unfinished);
    assert_eq!(outcome.reason.as_deref(), Some("time limit of 1 s"));
}

#[test]
fn a_number_computed_beyond_32_bits_is_noted_in_the_outcome() {
    let (mut sketch, n) = counter(3);
    sketch.end("full", is(&n, 3));
    let square = Expr::Apply(
        Op::Math(Math::Multiply),
        vec![plus(&n, 50_000), plus(&n, 50_000)],
    );
    sketch.rule(
        "big",
        Expr::Apply(Op::Order(Order::Greater), vec![square, int(0)]),
    );
    assert!(explore(&sketch.model(), &Limits::default()).computes_beyond_32_bits);
    let (mut small, top) = counter(3);
    small.end("full", is(&top, 3));
    assert!(!explore(&small.model(), &Limits::default()).computes_beyond_32_bits);
}

fn grows_after_a_stuck_state() -> Model {
    let mut sketch = Sketch::new();
    let j = sketch.number("j", 0, 1, &[0]);
    let q = sketch.number("q", 0, 99, &[0]);
    sketch.action("grow", both(is(&j, 0), below(&q, 99)), &[(&q, plus(&q, 1))]);
    sketch.action("step", is(&j, 0), &[(&j, int(1))]);
    sketch.model()
}

#[test]
fn a_model_with_a_stuck_state_found_before_the_state_limit_is_fail_and_says_it_stopped_early() {
    let outcome = explore(&grows_after_a_stuck_state(), &with_states(6));
    assert_eq!(outcome.status, Status::Fail, "the stuck state was found");
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 6"));
    assert_eq!(outcome.findings.len(), 1, "{:?}", outcome.findings);
}

#[test]
fn a_model_with_a_fault_found_before_the_state_limit_is_fail_and_says_it_stopped_early() {
    let mut sketch = Sketch::new();
    let o = sketch.field(
        "o",
        Domain::Optional(Box::new(Domain::Int { lo: 0, hi: 1 })),
        vec![Value::Null],
    );
    let j = sketch.number("j", 0, 99, &[0]);
    let read = Expr::Apply(Op::Unwrap, vec![o]);
    sketch.action("faulty", apply(Op::Order(Order::Less), &read, int(1)), &[]);
    sketch.action("step", below(&j, 99), &[(&j, plus(&j, 1))]);
    let outcome = explore(&sketch.model(), &with_states(5));
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 5"));
    assert_eq!(outcome.findings.len(), 1, "{:?}", outcome.findings);
}

#[test]
fn a_model_stopped_by_a_limit_with_nothing_broken_is_unfinished() {
    let (mut sketch, n) = counter(99);
    sketch.end("full", is(&n, 99));
    unfinished_at(
        &explore(&sketch.model(), &with_states(10)),
        10,
        "state limit of 10",
    );
}

#[test]
fn a_state_out_of_range_is_counted_judged_by_no_rule_and_not_searched_from() {
    let mut sketch = Sketch::new();
    let n = sketch.number("n", 0, 2, &[0]);
    sketch.action("jump", is(&n, 0), &[(&n, int(5))]);
    sketch.action("never", is(&n, 5), &[(&n, int(1))]);
    sketch.rule("low", below(&n, 3));
    let outcome = explore(&sketch.model(), &Limits::default());
    assert_eq!(
        outcome.states, 2,
        "the start state and the state out of range"
    );
    assert_eq!(
        outcome.rules[0].verdict,
        Verdict::Unknown,
        "no rule is judged in it, so the rule is not known to hold"
    );
    let kinds: Vec<_> = outcome.findings.iter().map(|found| found.kind).collect();
    assert_eq!(
        kinds,
        [FindingKind::OutOfRange],
        "and nothing is stuck or faulted after it"
    );
    assert_eq!(
        outcome.findings[0].path.steps.len(),
        1,
        "with the path to it"
    );
}

fn three_stays_then_a_move() -> Model {
    let mut sketch = Sketch::new();
    let a = sketch.number("a", 0, 1, &[0]);
    for name in ["stayOne", "stayTwo", "stayThree"] {
        sketch.action(name, is(&a, 0), &[(&a, a.clone())]);
    }
    sketch.action("go", is(&a, 0), &[(&a, int(1))]);
    sketch.rule("neverMoves", is(&a, 0));
    sketch.end("moved", is(&a, 1));
    sketch.model()
}

fn jumps_from(starts: &[i64]) -> Model {
    let mut sketch = Sketch::new();
    let x = sketch.number("x", 0, 1023, starts);
    let y = sketch.number("y", 0, 1, &[0]);
    let z = sketch.number("z", 0, 1, &[0]);
    let every_k: Vec<Value> = (0..1024).map(Value::Int).collect();
    let at_top = both(is(&x, 1023), is(&z, 0));
    sketch.action("bump", is(&y, 0), &[(&y, int(1))]);
    for (name, lands) in [("jumpOne", 0), ("jumpTwo", 0), ("lift", 1)] {
        sketch.action_with(
            name,
            &[("k", every_k.clone())],
            at_top.clone(),
            &[(&x, Expr::Local(0)), (&y, int(lands))],
        );
    }
    sketch.action("last", at_top, &[(&z, int(1))]);
    sketch.rule("flat", is(&z, 0));
    sketch.end("rest", yes());
    sketch.model()
}

#[test]
fn successors_already_known_never_use_up_the_state_limit() {
    let every_start: Vec<i64> = (0..1024).collect();
    let cases = [
        (three_stays_then_a_move(), 2),
        (jumps_from(&every_start), 2050),
    ];
    for (model, states) in &cases {
        for (case, outcome) in every_worker_count(model, *states) {
            assert_eq!(outcome.status, Status::Fail, "{case}");
            assert_eq!(outcome.states, *states, "{case}");
            assert_eq!(outcome.reason, None, "{case}");
            assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
        }
    }
}

#[test]
fn a_state_with_more_new_successors_than_the_limit_leaves_the_search_unfinished() {
    let model = jumps_from(&[1023]);
    for (case, outcome) in every_worker_count(&model, 100) {
        assert_eq!(outcome.status, Status::Unfinished, "{case}");
        assert_eq!(outcome.reason.as_deref(), Some("state limit of 100"));
        assert_eq!(outcome.rules[0].verdict, Verdict::Unknown);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Faulting {
    First,
    Last,
}

fn two_fans_and_a_fault(faulting: Faulting) -> Model {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..600).collect();
    let x = sketch.number("x", 0, 599, &every_start);
    let t = sketch.number("t", 0, 1, &[0]);
    let k = sketch.number("k", 0, 2001, &[0]);
    let shared_k: Vec<Value> = (0..1000).map(Value::Int).collect();
    let own_k: Vec<Value> = (1000..2001).map(Value::Int).collect();
    let fans_out = |at: i64| both(is(&x, at), is(&t, 0));
    let last = fans_out(599);
    let divides_by_zero = is(&apply(Op::Math(Math::Divide), &k, int(0)), 0);
    let boom = both(last.clone(), divides_by_zero);
    let lands = [(&x, int(0)), (&t, int(1)), (&k, Expr::Local(0))];
    if faulting == Faulting::First {
        sketch.action("boom", boom.clone(), &[(&t, int(1))]);
    }
    let first_or_last = Expr::Or(Box::new(fans_out(0)), Box::new(last.clone()));
    sketch.action_with("fan", &[("k", shared_k)], first_or_last, &lands);
    sketch.action_with("fanMore", &[("k", own_k)], last, &lands);
    if faulting == Faulting::Last {
        sketch.action("boom", boom, &[(&t, int(1))]);
    }
    sketch.end("rest", yes());
    sketch.model()
}

#[test]
fn successors_an_earlier_state_of_the_level_found_never_use_up_the_room_of_a_later_one() {
    for (faulting, max_states) in [
        (Faulting::Last, 2000),
        (Faulting::First, 2000),
        (Faulting::Last, 3000),
    ] {
        let model = two_fans_and_a_fault(faulting);
        let alone = explore(&model, &with_states(max_states));
        assert_eq!(alone.status, Status::Fail, "the last start state faults");
        assert_eq!(kinds_of(&alone), [FindingKind::Fault], "{faulting:?}");
        for (case, outcome) in every_worker_count(&model, max_states) {
            let case = format!("{faulting:?} faults, room {max_states}, {case}");
            assert_eq!(outcome.status, alone.status, "{case}");
            assert_eq!(outcome.states, alone.states, "{case}");
            assert_eq!(outcome.reason, alone.reason, "{case}");
            assert_eq!(outcome.findings, alone.findings, "{case}");
        }
    }
}

fn every_worker_count(model: &Model, max_states: usize) -> Vec<(String, Outcome)> {
    let mut outcomes = Vec::new();
    for keep_graph in [false, true] {
        for jobs in [1, 2, 4, 10] {
            let limits = Limits {
                jobs,
                keep_graph,
                ..with_states(max_states)
            };
            outcomes.push((
                format!("{jobs} jobs, graph {keep_graph}"),
                explore(model, &limits),
            ));
        }
    }
    outcomes
}

fn kinds_of(outcome: &Outcome) -> Vec<FindingKind> {
    outcome.findings.iter().map(|found| found.kind).collect()
}

fn numbers(range: std::ops::RangeInclusive<i64>) -> Vec<Value> {
    range.map(Value::Int).collect()
}

fn nine_divided_by_x_in_y(sketch: &mut Sketch, x: &Expr, y: &Expr) {
    let divided = apply(Op::Math(Math::Divide), &int(9), x.clone());
    sketch.action("fault", below(y, 2), &[(y, divided)]);
}

fn a_fault_behind_a_fan_of_successors(first: i64, last: i64) -> Model {
    let mut sketch = Sketch::new();
    let x = sketch.number("x", 0, 9, &[0]);
    let y = sketch.number("y", 0, 9, &[0]);
    let f = sketch.boolean("f", &[false]);
    let p_above_four = apply(Op::Order(Order::Greater), &Expr::Local(0), int(4));
    sketch.action_with(
        "set",
        &[
            ("p", numbers(first..=last)),
            ("q", vec![Value::Bool(false), Value::Bool(true)]),
        ],
        below(&y, 2),
        &[
            (&x, Expr::Local(0)),
            (&f, both(Expr::Local(1), p_above_four)),
        ],
    );
    nine_divided_by_x_in_y(&mut sketch, &x, &y);
    sketch.model()
}

fn a_fault_behind_two_actions_that_reach_one_state(first: i64, last: i64) -> Model {
    let mut sketch = Sketch::new();
    let x = sketch.number("x", 0, 9, &[0]);
    let y = sketch.number("y", 0, 9, &[0]);
    sketch.action_with(
        "one of several",
        &[("p", numbers(first..=last))],
        below(&y, 2),
        &[(&x, Expr::Local(0))],
    );
    sketch.action("the last of them", below(&y, 2), &[(&x, int(last))]);
    nine_divided_by_x_in_y(&mut sketch, &x, &y);
    sketch.model()
}

fn a_fault_behind_a_return_to_a_state_the_store_holds(first: i64, last: i64) -> Model {
    let mut sketch = Sketch::new();
    let x = sketch.number("x", 0, 9, &[0]);
    let y = sketch.number("y", 0, 9, &[0]);
    sketch.action_with(
        "one of several",
        &[("p", numbers(first..=last))],
        below(&y, 2),
        &[(&x, Expr::Local(0))],
    );
    sketch.action(
        "back to the first of them",
        below(&y, 2),
        &[(&x, int(first))],
    );
    nine_divided_by_x_in_y(&mut sketch, &x, &y);
    sketch.model()
}

#[test]
fn once_the_store_is_full_every_new_successor_uses_up_the_room_of_the_state_that_offers_it() {
    let model = a_fault_behind_a_fan_of_successors(0, 9);
    for (case, outcome) in every_worker_count(&model, 2) {
        assert_eq!(
            outcome.status,
            Status::Unfinished,
            "the start state offers more new successors than the room of 2 allows, so the \
             search stops trying its actions before the one that divides by zero and nothing \
             is found broken: {case}"
        );
        assert_eq!(
            outcome.states, 2,
            "the start state and its first new successor fill the store: {case}"
        );
        assert_eq!(
            outcome.reason.as_deref(),
            Some("state limit of 2"),
            "the store is full: {case}"
        );
        assert_eq!(
            kinds_of(&outcome),
            [],
            "the action that divides by zero is never tried, since the new successors of the \
             action before it used up more room than the limit allows: {case}"
        );
    }
}

#[test]
fn once_the_store_is_full_a_new_successor_offered_twice_uses_up_the_room_once() {
    let model = a_fault_behind_two_actions_that_reach_one_state(1, 3);
    for (case, outcome) in every_worker_count(&model, 3) {
        assert_eq!(
            outcome.status,
            Status::Fail,
            "x = 3 is offered by two actions but is one state: it uses up the room once, so \
             the action that divides by zero is tried and its fault is found: {case}"
        );
        assert_eq!(
            outcome.states, 3,
            "the start state and x = 1 and x = 2 fill the store: {case}"
        );
        assert_eq!(
            outcome.reason.as_deref(),
            Some("state limit of 3"),
            "x = 3 did not fit, so the store is full: {case}"
        );
        assert_eq!(
            kinds_of(&outcome),
            [FindingKind::Fault],
            "9 / x divides by zero in the start state: {case}"
        );
    }
}

#[test]
fn once_the_store_is_full_every_new_state_out_of_range_uses_up_the_room_of_the_state_offering_it() {
    let model = a_fault_behind_a_fan_of_successors(10, 19);
    for (case, outcome) in every_worker_count(&model, 2) {
        assert_eq!(
            outcome.status,
            Status::Fail,
            "x = 10 is a state out of range and a finding: {case}"
        );
        assert_eq!(
            outcome.states, 2,
            "the start state and x = 10 fill the store: {case}"
        );
        assert_eq!(
            outcome.reason.as_deref(),
            Some("state limit of 2"),
            "x = 11 did not fit, so the store is full: {case}"
        );
        assert_eq!(
            kinds_of(&outcome),
            [FindingKind::OutOfRange],
            "the start state offers more new states out of range than the room of 2 allows, so \
             the search stops trying its actions before the one that divides by zero and finds \
             no fault: {case}"
        );
    }
}

#[test]
fn once_the_store_is_full_a_new_state_out_of_range_offered_twice_uses_up_the_room_once() {
    let model = a_fault_behind_two_actions_that_reach_one_state(11, 13);
    for (case, outcome) in every_worker_count(&model, 3) {
        assert_eq!(
            outcome.status,
            Status::Fail,
            "x = 11 is a state out of range and a finding: {case}"
        );
        assert_eq!(
            outcome.states, 3,
            "the start state and x = 11 and x = 12 fill the store: {case}"
        );
        assert_eq!(
            outcome.reason.as_deref(),
            Some("state limit of 3"),
            "x = 13 did not fit, so the store is full: {case}"
        );
        assert_eq!(
            kinds_of(&outcome),
            [FindingKind::OutOfRange, FindingKind::Fault],
            "x = 13 is offered by two actions but is one state: it uses up the room once, so \
             the action that divides by zero is tried and 9 / x divides by zero at x = 0: {case}"
        );
    }
}

#[test]
fn once_the_store_is_full_a_successor_the_store_already_holds_uses_up_no_room() {
    let model = a_fault_behind_a_return_to_a_state_the_store_holds(1, 3);
    for (case, outcome) in every_worker_count(&model, 3) {
        assert_eq!(
            outcome.status,
            Status::Fail,
            "x = 1 is already in the store, so going back to it is no new state and uses up \
             no room: the action that divides by zero is tried and its fault is found: {case}"
        );
        assert_eq!(
            outcome.states, 3,
            "the start state and x = 1 and x = 2 fill the store: {case}"
        );
        assert_eq!(
            outcome.reason.as_deref(),
            Some("state limit of 3"),
            "x = 3 did not fit, so the store is full: {case}"
        );
        assert_eq!(
            kinds_of(&outcome),
            [FindingKind::Fault],
            "9 / x divides by zero in the start state: {case}"
        );
    }
}

#[test]
fn once_the_store_is_full_a_state_out_of_range_the_store_already_holds_uses_up_no_room() {
    let model = a_fault_behind_a_return_to_a_state_the_store_holds(11, 13);
    for (case, outcome) in every_worker_count(&model, 3) {
        assert_eq!(
            outcome.status,
            Status::Fail,
            "x = 11 is a state out of range and a finding: {case}"
        );
        assert_eq!(
            outcome.states, 3,
            "the start state and x = 11 and x = 12 fill the store: {case}"
        );
        assert_eq!(
            outcome.reason.as_deref(),
            Some("state limit of 3"),
            "x = 13 did not fit, so the store is full: {case}"
        );
        assert_eq!(
            kinds_of(&outcome),
            [FindingKind::OutOfRange, FindingKind::Fault],
            "x = 11 is already in the store, so going back to it is no new state and uses up \
             no room: the action that divides by zero is tried and 9 / x divides by zero at \
             x = 0: {case}"
        );
    }
}

const STATES_TO_A_SLOW_GUARD: i64 = 2000;
const STATES_TO_A_SLOW_GUARD_A_LONE_THREAD_TAKES_IN_PARTS: i64 = 100_000;
const STUCK_START: i64 = 5;

fn a_level_whose_last_state_takes_billions_of_steps(starts: i64) -> Model {
    let slow_state = starts - 1;
    let stuck_start_in_the_last_round = starts - 2;
    let half_of_the_starts = starts / 2;
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..starts).collect();
    let n = sketch.number("n", 0, slow_state, &every_start);
    let flipped = sketch.boolean("flipped", &[false]);
    let low = both(below(&n, half_of_the_starts), is_not(&n, STUCK_START));
    sketch.action("flipLow", both(not(&flipped), low), &[(&flipped, yes())]);
    sketch.action(
        "flipHigh",
        both(
            both(not(&flipped), not(&below(&n, half_of_the_starts))),
            is_not(&n, stuck_start_in_the_last_round),
        ),
        &[(&flipped, yes()), (&n, plus(&n, -half_of_the_starts))],
    );
    sketch.action(
        "back",
        both(
            not(&flipped),
            both(
                both(not(&is(&n, 0)), is_not(&n, STUCK_START)),
                is_not(&n, stuck_start_in_the_last_round),
            ),
        ),
        &[(&n, plus(&n, -1))],
    );
    sketch.action(
        "slow",
        both(
            is(&n, slow_state),
            every_of(2000, every_of(2000, every_of(2000, yes()))),
        ),
        &[(&flipped, yes())],
    );
    sketch.end("flipped", flipped.clone());
    sketch.rule("neverFlippedTen", not(&both(flipped, is(&n, 10))));
    sketch.model()
}

fn a_time_stop_in_a_later_round_of_a_level_of(starts: i64) {
    let model = a_level_whose_last_state_takes_billions_of_steps(starts);
    let flipped = usize::try_from(starts / 2).unwrap_or(0);
    let starts = usize::try_from(starts).unwrap_or(0);
    let limits = Limits {
        max_states: starts + flipped + 1,
        time_limit: Some(Duration::from_secs(1)),
        ..Limits::default()
    };
    let began = Instant::now();
    let outcome = same_outcome_with_every_crew(&model, &limits, &[4]);
    assert!(
        began.elapsed() < Duration::from_mins(1),
        "each search stops near its limit of 1 s: {:?}",
        began.elapsed()
    );
    assert_eq!(outcome.status, Status::Unfinished, "{:?}", outcome.findings);
    assert_eq!(
        outcome.reason.as_deref(),
        Some("time limit of 1 s"),
        "the slow last state of the level"
    );
    assert!(
        outcome.findings.is_empty(),
        "both stuck start states were expanded in the level the time stop undid, the one next \
         to the slow state in the round the stop ended: {:?}",
        outcome.findings
    );
    let graph = outcome.graph.as_ref().expect("the graph is kept");
    assert!(
        graph.edges.is_empty(),
        "no state of the undone level has arrows out: {:?}",
        graph.edges
    );
}

#[test]
fn a_time_stop_in_a_later_round_of_a_level_leaves_nothing_of_the_level_behind() {
    a_time_stop_in_a_later_round_of_a_level_of(STATES_TO_A_SLOW_GUARD);
}

#[test]
fn a_time_stop_in_a_later_round_of_a_level_a_lone_thread_takes_in_parts_leaves_nothing_behind() {
    a_time_stop_in_a_later_round_of_a_level_of(STATES_TO_A_SLOW_GUARD_A_LONE_THREAD_TAKES_IN_PARTS);
}

const STARTS_OF_UNIQUE_AND_SHARED_CHILDREN: i64 = 2000;
const SHARED_CHILDREN: i64 = 500;

fn starts_whose_first_half_share_children() -> Model {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..STARTS_OF_UNIQUE_AND_SHARED_CHILDREN).collect();
    let n = sketch.number(
        "n",
        0,
        STARTS_OF_UNIQUE_AND_SHARED_CHILDREN - 1,
        &every_start,
    );
    let flipped = sketch.boolean("flipped", &[false]);
    let shared_up_to = 2 * SHARED_CHILDREN;
    sketch.action(
        "flipOwn",
        both(not(&flipped), not(&below(&n, shared_up_to))),
        &[(&flipped, yes())],
    );
    sketch.action(
        "flipLow",
        both(not(&flipped), below(&n, SHARED_CHILDREN)),
        &[(&flipped, yes())],
    );
    sketch.action(
        "flipHigh",
        both(
            not(&flipped),
            both(not(&below(&n, SHARED_CHILDREN)), below(&n, shared_up_to)),
        ),
        &[(&flipped, yes()), (&n, plus(&n, -SHARED_CHILDREN))],
    );
    sketch.end("flipped", flipped.clone());
    sketch.rule("neverFlipped1990", not(&both(flipped, is(&n, 1990))));
    sketch.model()
}

#[test]
fn every_state_of_a_level_held_back_at_a_limit_never_reached_is_expanded() {
    let starts = usize::try_from(STARTS_OF_UNIQUE_AND_SHARED_CHILDREN).unwrap_or(0);
    let children =
        usize::try_from(STARTS_OF_UNIQUE_AND_SHARED_CHILDREN - SHARED_CHILDREN).unwrap_or(0);
    let outcome = same_outcome_on_every_worker_count(
        &starts_whose_first_half_share_children(),
        starts + children + 1,
    );
    assert_eq!(outcome.states, starts + children);
    assert_eq!(outcome.reason, None);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
}

const STARTS_OF_ONE_CHILD_EACH: i64 = 2000;
const CHILDREN_THAT_FIT: i64 = 600;
const SLOW_CHILD: i64 = CHILDREN_THAT_FIT - 1;

fn starts_whose_last_child_that_fits_takes_billions_of_steps_to_judge() -> Model {
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..STARTS_OF_ONE_CHILD_EACH).collect();
    let n = sketch.number("n", 0, STARTS_OF_ONE_CHILD_EACH - 1, &every_start);
    let flipped = sketch.boolean("flipped", &[false]);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    let slow = every_of(2000, every_of(2000, every_of(2000, yes())));
    sketch.rule(
        "slowAtTheLastChild",
        not(&both(both(flipped, is(&n, SLOW_CHILD)), slow)),
    );
    sketch.model()
}

#[test]
fn a_time_stop_in_the_last_pass_keeps_every_state_judged_before_it_on_every_worker_count() {
    let model = starts_whose_last_child_that_fits_takes_billions_of_steps_to_judge();
    let starts = usize::try_from(STARTS_OF_ONE_CHILD_EACH).unwrap_or(0);
    let children = usize::try_from(CHILDREN_THAT_FIT).unwrap_or(0);
    for jobs in [1, 4] {
        let limits = Limits {
            max_states: starts + children,
            time_limit: Some(Duration::from_secs(1)),
            jobs,
            ..Limits::default()
        };
        let outcome = explore(&model, &limits);
        assert_eq!(
            outcome.reason.as_deref(),
            Some("time limit of 1 s"),
            "{jobs} workers"
        );
        assert_eq!(
            outcome.states,
            starts + children - 1,
            "the starts and every child but the slow last one had their rules judged: {jobs} \
             workers"
        );
    }
}

const STARTS_OF_A_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS: i64 = 65_536;
const CHILDREN_OF_PAIRS_OF_STARTS: i64 = STARTS_OF_A_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS / 2;
const LATE_CHILD: i64 = CHILDREN_OF_PAIRS_OF_STARTS - 3;
const LIMITS_INSIDE_THE_CHILDREN: usize = 6;

fn starts_whose_pairs_share_a_child() -> Model {
    let (mut sketch, n, flipped) =
        every_start_up_to(STARTS_OF_A_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS);
    let halved = apply(Op::Math(Math::Divide), &n, int(2));
    sketch.action(
        "flipHalf",
        not(&flipped),
        &[(&flipped, yes()), (&n, halved)],
    );
    sketch.end("flipped", flipped.clone());
    sketch.rule("neverFlippedLate", not(&both(flipped, is(&n, LATE_CHILD))));
    sketch.model()
}

#[test]
fn a_lone_thread_taking_a_wide_level_in_parts_matches_the_graph_kept_search_at_every_state_limit() {
    let model = starts_whose_pairs_share_a_child();
    let starts = usize::try_from(STARTS_OF_A_LEVEL_A_LONE_THREAD_TAKES_IN_PARTS).unwrap_or(0);
    let children = usize::try_from(CHILDREN_OF_PAIRS_OF_STARTS).unwrap_or(0);
    let states = starts + children;
    let inside = (1..=LIMITS_INSIDE_THE_CHILDREN)
        .map(|share| starts + children * share / (LIMITS_INSIDE_THE_CHILDREN + 1));
    let near_the_ends = [
        starts - 1,
        starts,
        starts + 1,
        starts + 2,
        states - 1,
        states,
        states + 1,
    ];
    for max_states in near_the_ends.into_iter().chain(inside) {
        let row_by_row = explore(
            &model,
            &Limits {
                keep_graph: true,
                ..with_states(max_states)
            },
        );
        let in_parts = explore(&model, &with_states(max_states));
        assert_eq!(
            in_parts,
            Outcome {
                graph: None,
                ..row_by_row
            },
            "state limit {max_states}"
        );
        assert_eq!(
            in_parts.states,
            max_states.min(states),
            "state limit {max_states}"
        );
    }
    let whole = explore(&model, &with_states(states + 1));
    assert_eq!(whole.status, Status::Fail);
    assert_eq!(whole.reason, None);
    assert_eq!(whole.rules[0].verdict, Verdict::Broken);
}

use crate::build::{
    HYPERCUBE_STATES, HYPERCUBE_TOP, at_point, every_start_up_to, hypercube_sketch,
};

const HYPERCUBE_STATES_THROUGH_SUM_21: usize = 44_000;
const HYPERCUBE_STATES_THROUGH_SUM_22: usize = 50_000;
const STARTS_OF_A_WIDE_LEVEL: i64 = 5000;

fn wide_level_of_flips(starts: i64) -> Model {
    let (mut sketch, n, flipped) = every_start_up_to(starts);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    sketch.end("flipped", flipped.clone());
    sketch.rule("not1234", not(&both(flipped, is(&n, 1234))));
    sketch.model()
}

fn hypercube_with_a_late_rule() -> Model {
    let (mut sketch, axes) = hypercube_sketch(HYPERCUBE_TOP, Moves::Up);
    sketch.end("corner", at_point(&axes, [HYPERCUBE_TOP; 5]));
    sketch.rule("notEarly", not(&at_point(&axes, [1, 2, 3, 4, 5])));
    sketch.rule("notAtSum22", not(&at_point(&axes, [9, 9, 2, 1, 1])));
    sketch.rule("notAtSum23", not(&at_point(&axes, [9, 9, 2, 2, 1])));
    sketch.model()
}

#[test]
fn a_level_of_five_thousand_states_gives_the_same_outcome_on_one_thread_and_on_several() {
    let model = wide_level_of_flips(STARTS_OF_A_WIDE_LEVEL);
    let single = explore(&model, &with_jobs(1));
    assert_eq!(single.states, 10_000);
    assert_eq!(single.status, Status::Fail);
    let path = single.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(texts(&path.start), ["n = 1234", "flipped = false"]);
    assert_eq!(steps(path), ["flip  flipped: false -> true"]);
    for jobs in [3, 4, 64] {
        assert_eq!(explore(&model, &with_jobs(jobs)), single, "{jobs} workers");
    }
}

#[test]
fn a_search_with_many_levels_of_thousands_of_states_gives_the_same_outcome_on_one_thread_and_on_four()
 {
    let (mut sketch, axes) = hypercube_sketch(HYPERCUBE_TOP, Moves::Up);
    sketch.end("corner", at_point(&axes, [HYPERCUBE_TOP; 5]));
    sketch.rule("notThere", not(&at_point(&axes, [3, 5, 7, 2, 9])));
    let model = sketch.model();
    let single = explore(&model, &with_jobs(1));
    assert_eq!(single.states, HYPERCUBE_STATES);
    assert_eq!(single.status, Status::Fail);
    let path = single.rules[0]
        .path
        .as_ref()
        .expect("a broken rule has a path");
    assert_eq!(path.steps.len(), 26);
    assert!(single.findings.is_empty(), "{:?}", single.findings);
    assert_eq!(explore(&model, &with_jobs(4)), single);
}

#[test]
fn the_state_limit_inside_a_level_of_five_thousand_states_gives_the_same_outcome_on_one_thread_and_on_four()
 {
    let model = wide_level_of_flips(STARTS_OF_A_WIDE_LEVEL);
    let limited = |jobs: usize| Limits {
        max_states: 7000,
        ..with_jobs(jobs)
    };
    let single = explore(&model, &limited(1));
    assert_eq!(single.states, 7000);
    assert_eq!(single.reason.as_deref(), Some("state limit of 7000"));
    assert_eq!(explore(&model, &limited(4)), single);
}

#[test]
fn a_state_limit_at_the_end_of_a_wide_level_gives_the_same_outcome_on_every_worker_count() {
    let outcome = same_outcome_on_every_worker_count(
        &hypercube_with_a_late_rule(),
        HYPERCUBE_STATES_THROUGH_SUM_22,
    );
    assert_eq!(outcome.states, HYPERCUBE_STATES_THROUGH_SUM_22);
    assert_eq!(outcome.reason.as_deref(), Some("state limit of 50000"));
    let verdicts: Vec<Verdict> = outcome.rules.iter().map(|rule| rule.verdict).collect();
    assert_eq!(
        verdicts,
        [Verdict::Broken, Verdict::Broken, Verdict::Unknown]
    );
}

#[test]
fn a_state_limit_inside_a_wide_level_gives_the_same_outcome_on_every_worker_count() {
    let inside = HYPERCUBE_STATES_THROUGH_SUM_21 + 3000;
    let outcome = same_outcome_on_every_worker_count(&hypercube_with_a_late_rule(), inside);
    assert_eq!(outcome.states, inside);
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
    assert_eq!(outcome.rules[2].verdict, Verdict::Unknown);
}

#[test]
fn a_state_limit_one_above_the_states_of_a_model_of_wide_levels_gives_the_same_outcome_on_every_worker_count()
 {
    let outcome =
        same_outcome_on_every_worker_count(&hypercube_with_a_late_rule(), HYPERCUBE_STATES + 1);
    assert_eq!(outcome.states, HYPERCUBE_STATES);
    assert_eq!(outcome.reason, None);
    let verdicts: Vec<Verdict> = outcome.rules.iter().map(|rule| rule.verdict).collect();
    assert_eq!(verdicts, [Verdict::Broken; 3]);
}

#[test]
fn a_state_limit_one_above_the_states_of_a_level_of_five_thousand_known_steps_is_never_reached() {
    let top = STARTS_OF_A_WIDE_LEVEL - 1;
    let (mut sketch, n, flipped) = every_start_up_to(STARTS_OF_A_WIDE_LEVEL);
    sketch.action("step", below(&n, top), &[(&n, plus(&n, 1))]);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    sketch.end("any", yes());
    sketch.rule("not1234", not(&both(flipped, is(&n, 1234))));
    let states = 2 * usize::try_from(STARTS_OF_A_WIDE_LEVEL).unwrap_or(0);
    let outcome = same_outcome_on_every_worker_count(&sketch.model(), states + 1);
    assert_eq!(outcome.states, states);
    assert_eq!(outcome.reason, None);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
}

const TOP_OF_WIDE_JUMPS: i64 = 4095;

fn wide_jumps_from(starts: &[i64]) -> Model {
    let mut sketch = Sketch::new();
    let x = sketch.number("x", 0, TOP_OF_WIDE_JUMPS, starts);
    let y = sketch.number("y", 0, 1, &[0]);
    let z = sketch.number("z", 0, 1, &[0]);
    let every_k: Vec<Value> = (0..=TOP_OF_WIDE_JUMPS).map(Value::Int).collect();
    let at_top = both(is(&x, TOP_OF_WIDE_JUMPS), is(&z, 0));
    sketch.action("bump", is(&y, 0), &[(&y, int(1))]);
    for (name, lands) in [("jumpOne", 0), ("jumpTwo", 0), ("lift", 1)] {
        sketch.action_with(
            name,
            &[("k", every_k.clone())],
            at_top.clone(),
            &[(&x, Expr::Local(0)), (&y, int(lands))],
        );
    }
    sketch.action("last", at_top, &[(&z, int(1))]);
    sketch.rule("flat", is(&z, 0));
    sketch.end("rest", yes());
    sketch.model()
}

#[test]
fn successors_already_known_never_use_up_the_state_limit_of_a_wide_level() {
    let every_start: Vec<i64> = (0..=TOP_OF_WIDE_JUMPS).collect();
    let model = wide_jumps_from(&every_start);
    let states = 2 * every_start.len() + 2;
    for (case, outcome) in every_worker_count(&model, states) {
        assert_eq!(outcome.status, Status::Fail, "{case}");
        assert_eq!(outcome.states, states, "{case}");
        assert_eq!(outcome.reason, None, "{case}");
        assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
    }
}

const STARTS_OF_WIDE_FANS: i64 = 2400;
const SHARED_FAN: i64 = 3000;
const WHOLE_FAN: i64 = 6001;

fn two_wide_fans_and_a_fault(faulting: Faulting) -> Model {
    let mut sketch = Sketch::new();
    let last_start = STARTS_OF_WIDE_FANS - 1;
    let every_start: Vec<i64> = (0..STARTS_OF_WIDE_FANS).collect();
    let x = sketch.number("x", 0, last_start, &every_start);
    let t = sketch.number("t", 0, 1, &[0]);
    let k = sketch.number("k", 0, WHOLE_FAN, &[0]);
    let shared_k: Vec<Value> = (0..SHARED_FAN).map(Value::Int).collect();
    let own_k: Vec<Value> = (SHARED_FAN..WHOLE_FAN).map(Value::Int).collect();
    let fans_out = |at: i64| both(is(&x, at), is(&t, 0));
    let last = fans_out(last_start);
    let divides_by_zero = is(&apply(Op::Math(Math::Divide), &k, int(0)), 0);
    let boom = both(last.clone(), divides_by_zero);
    let lands = [(&x, int(0)), (&t, int(1)), (&k, Expr::Local(0))];
    if faulting == Faulting::First {
        sketch.action("boom", boom.clone(), &[(&t, int(1))]);
    }
    let first_or_last = Expr::Or(Box::new(fans_out(0)), Box::new(last.clone()));
    sketch.action_with("fan", &[("k", shared_k)], first_or_last, &lands);
    sketch.action_with("fanMore", &[("k", own_k)], last, &lands);
    if faulting == Faulting::Last {
        sketch.action("boom", boom, &[(&t, int(1))]);
    }
    sketch.end("rest", yes());
    sketch.model()
}

#[test]
fn successors_an_earlier_state_of_a_wide_level_found_never_use_up_the_room_of_a_later_one() {
    for (faulting, max_states) in [
        (Faulting::Last, 6600),
        (Faulting::First, 6600),
        (Faulting::Last, 9600),
    ] {
        let model = two_wide_fans_and_a_fault(faulting);
        let alone = explore(&model, &with_states(max_states));
        assert_eq!(alone.status, Status::Fail, "the last start state faults");
        assert_eq!(kinds_of(&alone), [FindingKind::Fault], "{faulting:?}");
        for (case, outcome) in every_worker_count(&model, max_states) {
            let case = format!("{faulting:?} faults, room {max_states}, {case}");
            assert_eq!(outcome.status, alone.status, "{case}");
            assert_eq!(outcome.states, alone.states, "{case}");
            assert_eq!(outcome.reason, alone.reason, "{case}");
            assert_eq!(outcome.findings, alone.findings, "{case}");
        }
    }
}

const STARTS_OF_WIDE_UNIQUE_AND_SHARED_CHILDREN: i64 = 6000;
const WIDE_SHARED_CHILDREN: i64 = 1500;

#[test]
fn every_state_of_a_wide_level_held_back_at_a_limit_never_reached_is_expanded() {
    let (mut sketch, n, flipped) = every_start_up_to(STARTS_OF_WIDE_UNIQUE_AND_SHARED_CHILDREN);
    let shared_up_to = 2 * WIDE_SHARED_CHILDREN;
    sketch.action(
        "flipOwn",
        both(not(&flipped), not(&below(&n, shared_up_to))),
        &[(&flipped, yes())],
    );
    sketch.action(
        "flipLow",
        both(not(&flipped), below(&n, WIDE_SHARED_CHILDREN)),
        &[(&flipped, yes())],
    );
    sketch.action(
        "flipHigh",
        both(
            not(&flipped),
            both(
                not(&below(&n, WIDE_SHARED_CHILDREN)),
                below(&n, shared_up_to),
            ),
        ),
        &[(&flipped, yes()), (&n, plus(&n, -WIDE_SHARED_CHILDREN))],
    );
    sketch.end("flipped", flipped.clone());
    sketch.rule("neverFlipped5990", not(&both(flipped, is(&n, 5990))));
    let starts = usize::try_from(STARTS_OF_WIDE_UNIQUE_AND_SHARED_CHILDREN).unwrap_or(0);
    let children =
        usize::try_from(STARTS_OF_WIDE_UNIQUE_AND_SHARED_CHILDREN - WIDE_SHARED_CHILDREN)
            .unwrap_or(0);
    let outcome = same_outcome_on_every_worker_count(&sketch.model(), starts + children + 1);
    assert_eq!(outcome.states, starts + children);
    assert_eq!(outcome.reason, None);
    assert_eq!(outcome.rules[0].verdict, Verdict::Broken);
}

const STARTS_OF_ONE_CHILD_EACH_IN_A_WIDE_LEVEL: i64 = 6000;
const CHILDREN_OF_A_WIDE_LEVEL_THAT_FIT: i64 = 2500;

#[test]
fn a_time_stop_in_the_last_pass_over_a_wide_level_keeps_every_state_judged_before_it_on_every_worker_count()
 {
    let (mut sketch, n, flipped) = every_start_up_to(STARTS_OF_ONE_CHILD_EACH_IN_A_WIDE_LEVEL);
    sketch.action("flip", not(&flipped), &[(&flipped, yes())]);
    let slow = every_of(2000, every_of(2000, every_of(2000, yes())));
    let slow_child = CHILDREN_OF_A_WIDE_LEVEL_THAT_FIT - 1;
    sketch.rule(
        "slowAtTheLastChild",
        not(&both(both(flipped, is(&n, slow_child)), slow)),
    );
    let model = sketch.model();
    let starts = usize::try_from(STARTS_OF_ONE_CHILD_EACH_IN_A_WIDE_LEVEL).unwrap_or(0);
    let children = usize::try_from(CHILDREN_OF_A_WIDE_LEVEL_THAT_FIT).unwrap_or(0);
    for jobs in [1, 4] {
        let limits = Limits {
            max_states: starts + children,
            time_limit: Some(Duration::from_secs(1)),
            jobs,
            ..Limits::default()
        };
        let outcome = explore(&model, &limits);
        assert_eq!(
            outcome.reason.as_deref(),
            Some("time limit of 1 s"),
            "{jobs} workers"
        );
        assert_eq!(
            outcome.states,
            starts + children - 1,
            "the starts and every child but the slow last one had their rules judged: {jobs} \
             workers"
        );
    }
}

const STARTS_OF_A_LEVEL_TAKEN_IN_ROUNDS: i64 = 60_000;
const FOLDED_OF_A_LEVEL_TAKEN_IN_ROUNDS: i64 = STARTS_OF_A_LEVEL_TAKEN_IN_ROUNDS / 4;
const STATES_OF_A_LEVEL_TAKEN_IN_ROUNDS: usize = 150_000;
const STATES_BEFORE_THE_SECOND_WIDE_LEVEL: usize = 60_000;
const STATES_BEFORE_THE_LAST_LEVEL: usize = 135_000;
const STARTS_WITH_A_SLOW_STATE_IN_THE_THIRD_ROUND: i64 = 40_000;

fn a_model_whose_wide_levels_are_taken_in_rounds() -> Model {
    let starts = STARTS_OF_A_LEVEL_TAKEN_IN_ROUNDS;
    let folded = FOLDED_OF_A_LEVEL_TAKEN_IN_ROUNDS;
    let mut sketch = Sketch::new();
    let every_start: Vec<i64> = (0..starts).collect();
    let n = sketch.number("n", 0, starts - 1, &every_start);
    let k = sketch.number("k", 0, 3, &[0]);
    let fold = apply(Op::Math(Math::Remainder), &n, int(folded));
    sketch.action("fold", is(&k, 0), &[(&n, fold.clone()), (&k, int(1))]);
    sketch.action("mark", is(&k, 0), &[(&k, int(2))]);
    sketch.action("deepen", is(&k, 1), &[(&k, int(3))]);
    sketch.action("refold", is(&k, 2), &[(&n, fold), (&k, int(3))]);
    sketch.end("deep", is(&k, 3));
    sketch.rule("neverMarkedLate", not(&both(is(&k, 2), is(&n, starts - 5))));
    sketch.rule("neverDeepSeven", not(&both(is(&k, 3), is(&n, 7))));
    sketch.rule("neverFoldedLast", not(&both(is(&k, 1), is(&n, folded - 1))));
    sketch.model()
}

#[test]
fn wide_levels_a_lone_thread_takes_in_rounds_give_the_same_outcome_as_taken_at_once() {
    let model = a_model_whose_wide_levels_are_taken_in_rounds();
    let outcome = same_outcome_with_every_crew(&model, &with_states(usize::MAX), &[4]);
    assert_eq!(outcome.states, STATES_OF_A_LEVEL_TAKEN_IN_ROUNDS);
    assert_eq!(outcome.reason, None);
    let paths: Vec<Vec<String>> = outcome
        .rules
        .iter()
        .map(|rule| rule.path.as_ref().map_or_default(steps))
        .collect();
    assert_eq!(paths.len(), 3, "every rule is broken: {paths:?}");
    assert!(
        outcome
            .rules
            .iter()
            .all(|rule| rule.verdict == Verdict::Broken),
        "{paths:?}"
    );
    assert_eq!(
        paths,
        [
            vec!["mark  k: 0 -> 2"],
            vec!["fold  k: 0 -> 1", "deepen  k: 1 -> 3"],
            vec!["fold  k: 0 -> 1"],
        ],
        "each from the start the rule names, the first action of each state first"
    );
}

#[test]
fn a_state_limit_inside_a_wide_level_a_lone_thread_takes_in_rounds_gives_the_same_outcome() {
    let model = a_model_whose_wide_levels_are_taken_in_rounds();
    let starts = STATES_BEFORE_THE_SECOND_WIDE_LEVEL;
    let folded = usize::try_from(FOLDED_OF_A_LEVEL_TAKEN_IN_ROUNDS).unwrap_or(0);
    for limit in [
        starts + 1,
        starts + folded - 1,
        starts + folded,
        starts + folded + 1,
        starts + 2 * folded + 7,
        STATES_BEFORE_THE_LAST_LEVEL - 1,
        STATES_BEFORE_THE_LAST_LEVEL,
        STATES_BEFORE_THE_LAST_LEVEL + 1,
        STATES_BEFORE_THE_LAST_LEVEL + folded / 2,
        STATES_OF_A_LEVEL_TAKEN_IN_ROUNDS,
    ] {
        let outcome = same_outcome_with_every_crew(&model, &with_states(limit), &[4]);
        let expected = Some(format!("state limit of {limit}"));
        let full = limit < STATES_OF_A_LEVEL_TAKEN_IN_ROUNDS;
        assert_eq!(outcome.states, limit, "a limit of {limit}");
        assert_eq!(
            outcome.reason,
            expected.filter(|_| full),
            "a limit of {limit}"
        );
    }
}

#[test]
fn a_time_stop_in_the_third_round_of_a_level_a_lone_thread_takes_in_rounds_leaves_nothing_behind() {
    a_time_stop_in_a_later_round_of_a_level_of(STARTS_WITH_A_SLOW_STATE_IN_THE_THIRD_ROUND);
}
