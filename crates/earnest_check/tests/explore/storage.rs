use crate::build::{
    CUBE_TOP, Moves, Sketch, apply, at_corner, below, both, cube_sketch, is, message_queue, not,
    plus, size_of,
};
use earnest_check::{FindingKind, Limits, Outcome, Status, Verdict, explore};
use earnest_ir::{Expr, Model, Op, Order, SeqOp};
use earnest_value::Value;

fn queue_of_up_to(most: i64, max: usize) -> Model {
    let mut sketch = Sketch::new();
    let (queue, messages) = message_queue(&mut sketch, max);
    let below_three = apply(
        Op::Order(Order::Less),
        &size_of(&queue),
        Expr::Const(Value::Int(most)),
    );
    let sent = apply(Op::Seq(SeqOp::Push), &queue, Expr::Local(0));
    sketch.action_with(
        "send",
        &[("message", messages)],
        below_three.clone(),
        &[(&queue, sent)],
    );
    sketch.rule("neverThree", below_three);
    sketch.model()
}

fn cube() -> Model {
    cube_sketch(CUBE_TOP, Moves::Up).0.model()
}

fn with_jobs_and_graph(jobs: usize) -> Limits {
    Limits {
        jobs,
        keep_graph: true,
        ..Limits::default()
    }
}

fn with_jobs_only(jobs: usize) -> Limits {
    Limits {
        jobs,
        ..Limits::default()
    }
}

fn alone_and_with_workers(model: &Model, max_states: usize, worker_counts: &[usize]) -> Outcome {
    let limited = |jobs: usize| Limits {
        max_states,
        ..with_jobs_only(jobs)
    };
    let alone = explore(model, &limited(1));
    for jobs in worker_counts {
        assert_eq!(
            explore(model, &limited(*jobs)),
            alone,
            "{jobs} jobs, {max_states} states"
        );
    }
    alone
}

fn broken_after(outcome: &Outcome, steps: usize) {
    let rule = &outcome.rules[0];
    assert_eq!(rule.verdict, Verdict::Broken, "{outcome:?}");
    let path = rule.path.as_ref().expect("a broken rule has a path");
    assert_eq!(path.steps.len(), steps, "the shortest path");
}

#[test]
fn a_field_with_too_many_values_to_list_is_searched_like_one_with_few() {
    for limits in [
        Limits::default(),
        with_jobs_and_graph(1),
        with_jobs_and_graph(4),
    ] {
        let listed = explore(&queue_of_up_to(3, 3), &limits);
        let grown = explore(&queue_of_up_to(3, 20), &limits);
        assert_eq!(listed.states, 15, "queues of up to three messages");
        broken_after(&listed, 3);
        assert_eq!(grown, listed, "the same states, verdicts and paths");
    }
}

#[test]
fn a_value_that_leaves_its_domain_is_kept_and_shown_on_its_path() {
    let outcome = explore(&queue_of_up_to(3, 2), &Limits::default());
    assert_eq!(outcome.status, Status::Fail);
    assert_eq!(outcome.states, 15, "the queues of three are counted");
    assert_eq!(
        outcome.rules[0].verdict,
        Verdict::Unknown,
        "a state out of range is not judged"
    );
    let finding = &outcome.findings[0];
    assert_eq!(finding.kind, FindingKind::OutOfRange);
    assert_eq!(finding.path.steps.len(), 3, "the shortest path");
}

#[test]
fn workers_find_the_same_states_when_a_value_has_no_code_yet() {
    let listed = explore(&queue_of_up_to(10, 10), &Limits::default());
    assert_eq!(listed.states, 2047, "queues of up to ten messages");
    for limits in [Limits::default(), with_jobs_and_graph(4), with_jobs_only(4)] {
        let grown = explore(&queue_of_up_to(10, 20), &limits);
        assert_eq!(grown.states, listed.states);
        assert_eq!(grown.rules, listed.rules, "the same verdicts and paths");
    }
}

#[test]
fn workers_keep_the_states_that_leave_their_domain() {
    let alone = explore(&queue_of_up_to(10, 9), &Limits::default());
    assert_eq!(alone.states, 2047, "the queues of ten are counted");
    assert_eq!(alone.findings[0].kind, FindingKind::OutOfRange);
    assert_eq!(explore(&queue_of_up_to(10, 9), &with_jobs_only(4)), alone);
    assert_eq!(
        explore(&queue_of_up_to(10, 9), &with_jobs_and_graph(4)).findings,
        alone.findings
    );
}

#[test]
fn a_limit_above_four_billion_states_changes_nothing() {
    let model = cube();
    for jobs in [1, 4] {
        let usual = explore(&model, &with_jobs_and_graph(jobs));
        let huge = explore(
            &model,
            &Limits {
                max_states: usize::MAX,
                ..with_jobs_and_graph(jobs)
            },
        );
        assert_eq!(usual.states, 27_000);
        assert_eq!(huge, usual, "{jobs} jobs");
    }
}

fn cube_with_corners(corners: &[[i64; 3]]) -> Model {
    cube_on_axes_up_to(CUBE_TOP, corners)
}

fn cube_on_axes_up_to(highest: i64, corners: &[[i64; 3]]) -> Model {
    let (mut sketch, axes) = cube_sketch(highest, Moves::UpAndDown);
    for (number, corner) in corners.iter().enumerate() {
        sketch.rule(&format!("notAt{number}"), not(&at_corner(&axes, *corner)));
    }
    sketch.model()
}

#[test]
fn many_workers_find_the_same_paths_through_states_met_from_several_parts() {
    let corners = [
        [15, 15, 14],
        [20, 20, 5],
        [29, 1, 28],
        [10, 10, 10],
        [29, 29, 29],
    ];
    let model = cube_with_corners(&corners);
    let alone = explore(&model, &with_jobs_only(1));
    assert_eq!(alone.states, 27_000);
    for rule in &alone.rules {
        assert_eq!(rule.verdict, Verdict::Broken);
    }
    for jobs in [2, 3, 4, 7, 10] {
        assert_eq!(explore(&model, &with_jobs_only(jobs)), alone, "{jobs} jobs");
    }
}

#[test]
fn many_workers_stop_at_the_same_state_limit() {
    let model = cube_with_corners(&[[29, 29, 29]]);
    for max_states in [600, 4_000, 26_999] {
        alone_and_with_workers(&model, max_states, &[2, 4, 10]);
    }
}

fn queue_and_clock() -> Model {
    let mut sketch = Sketch::new();
    let (queue, messages) = message_queue(&mut sketch, 20);
    let clock = sketch.number("clock", 0, 3, &[0]);
    let size = size_of(&queue);
    let short = apply(Op::Order(Order::Less), &size, Expr::Const(Value::Int(12)));
    let sent = apply(Op::Seq(SeqOp::Push), &queue, Expr::Local(0));
    sketch.action("tick", below(&clock, 3), &[(&clock, plus(&clock, 1))]);
    sketch.action_with("send", &[("message", messages)], short, &[(&queue, sent)]);
    let late = both(is(&size, 10), is(&clock, 0));
    sketch.rule("neverTenUnticked", not(&late));
    sketch.model()
}

#[test]
fn workers_keep_the_order_of_states_with_and_without_a_code() {
    let model = queue_and_clock();
    let alone = explore(&model, &with_jobs_only(1));
    assert_eq!(alone.states, 8191 * 4);
    broken_after(&alone, 10);
    for max_states in (1_916..3_836).step_by(3) {
        alone_and_with_workers(&model, max_states, &[2, 4]);
    }
}

#[test]
fn workers_keep_rows_too_wide_for_one_word_the_same_as_one_thread() {
    let model = cube_on_axes_up_to(1 << 30, &[[29, 29, 29]]);
    for max_states in [600, 27_000] {
        let alone = alone_and_with_workers(&model, max_states, &[2, 4, 10]);
        assert_eq!(alone.states, max_states, "the walk stays in the cube");
    }
}
