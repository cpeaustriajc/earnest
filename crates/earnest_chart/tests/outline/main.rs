//! The chart drawn from an outline, checked against the chart of the whole graph drawn by the
//! code that drew it before outlines.

mod models;
mod reference;

use earnest_chart::{
    Charted, Section, chart_outline, chart_with_limit, report_with_charts, report_with_limit,
};
use earnest_check::{Graph, Limits, Outcome, Outline, explore};
use earnest_ir::Model;
use earnest_ir::{Expr, Op};
use earnest_value::Value;
use models::{Dice, random_model};

const MODELS: u64 = 600;
const FEWEST_OF_EACH_KIND: usize = 10;

fn searched(model: &Model, max_states: usize) -> (Outcome, Graph, Outline) {
    let limits = Limits {
        max_states,
        keep_graph: true,
        ..Limits::default()
    };
    let mut whole = explore(model, &limits);
    assert_eq!(whole.outline, None, "no outline was asked for");
    let graph = whole.graph.take().expect("the limits asked for the graph");
    let mut outlined = explore(
        model,
        &Limits {
            outline: true,
            ..limits
        },
    );
    assert_eq!(
        outlined.graph, None,
        "the outline takes the place of the graph"
    );
    let outline = outlined
        .outline
        .take()
        .expect("a small graph has an outline");
    assert_eq!(outlined, whole, "the outline changes nothing else");
    (whole, graph, outline)
}

#[derive(Default)]
struct Kinds {
    merged: usize,
    grouped: usize,
    several_labels: usize,
    beyond: usize,
    stopped: usize,
    out_of_range: usize,
}

#[test]
fn the_chart_of_an_outline_is_the_chart_of_its_graph_at_every_arrow_limit() {
    let mut kinds = Kinds::default();
    for seed in 0..MODELS {
        let mut dice = Dice::new(seed);
        let model = random_model(&mut dice);
        let max_states = if dice.below(3) == 0 {
            dice.below(30).saturating_add(1)
        } else {
            Limits::default().max_states
        };
        let (outcome, graph, outline) = searched(&model, max_states);
        kinds.merged += usize::from(graph.nodes.iter().any(|node| node.states > 1));
        kinds.grouped += usize::from(graph.nodes.iter().any(|node| node.group.is_some()));
        kinds.several_labels += usize::from(graph.edges.iter().any(|edge| edge.labels.len() > 1));
        kinds.stopped += usize::from(outcome.reason.is_some());
        kinds.out_of_range += usize::from(graph.nodes.iter().any(|node| !node.outside.is_empty()));
        let mut beyond = false;
        for limit in 0..=graph.edges.len().saturating_add(1) {
            let expected = reference::chart_with_limit(&graph, limit);
            beyond |= expected.contains("lies beyond") || expected.contains("lie beyond");
            assert_eq!(
                chart_outline(&outline, limit),
                expected,
                "the outline of model {seed} at {limit} arrows"
            );
            assert_eq!(
                chart_with_limit(&graph, limit),
                expected,
                "the graph of model {seed} at {limit} arrows"
            );
        }
        kinds.beyond += usize::from(beyond);
    }
    for (kind, count) in [
        ("boxes merged by label", kinds.merged),
        ("groups", kinds.grouped),
        ("several labels on an arrow", kinds.several_labels),
        ("a broken box beyond the limit", kinds.beyond),
        ("a search stopped at a state limit", kinds.stopped),
        ("a box out of range", kinds.out_of_range),
    ] {
        assert!(
            count >= FEWEST_OF_EACH_KIND,
            "only {count} of {MODELS} models with {kind}"
        );
    }
}

#[test]
fn a_report_of_charts_drawn_from_outlines_is_the_report_of_the_graphs() {
    for seed in 0..40 {
        let model = random_model(&mut Dice::new(seed));
        let (mut outcome, graph, outline) = searched(&model, Limits::default().max_states);
        for limit in [1, 7, 20] {
            let chart = chart_outline(&outline, limit);
            let without = Section {
                name: "Random",
                file: "random.model.ts",
                outcome: &outcome,
            };
            let drawn = report_with_charts(
                &[Charted {
                    section: without,
                    chart: Some(&chart),
                }],
                &[],
                None,
            );
            let mut whole = outcome.clone();
            whole.graph = Some(graph.clone());
            let from_graph = report_with_limit(
                &[Section {
                    outcome: &whole,
                    ..without
                }],
                &[],
                None,
                limit,
            );
            assert_eq!(drawn, from_graph, "model {seed} at {limit} arrows");
        }
        outcome.graph = None;
    }
}

#[test]
fn a_search_that_keeps_no_graph_leaves_no_outline() {
    let model = random_model(&mut Dice::new(1));
    let outcome = explore(
        &model,
        &Limits {
            outline: true,
            ..Limits::default()
        },
    );
    assert_eq!(outcome.graph, None, "no graph was asked for");
    assert_eq!(outcome.outline, None, "nor an outline");
}

#[test]
fn an_outline_says_nothing_of_a_box_or_arrow_it_does_not_have() {
    let model = random_model(&mut Dice::new(2));
    let (_, graph, outline) = searched(&model, Limits::default().max_states);
    let (boxes, arrows) = (outline.boxes(), outline.arrows());
    assert_eq!(boxes, graph.nodes.len(), "one box for each node");
    assert_eq!(arrows, graph.edges.len(), "one arrow for each edge");
    assert_eq!(outline.label(boxes), "", "no label");
    assert_eq!(outline.group(boxes), None, "no group");
    assert!(
        !outline.start(boxes) && !outline.end(boxes) && !outline.marked(boxes),
        "no mark"
    );
    assert_eq!(outline.states(boxes), 0, "no state");
    assert_eq!(outline.ends(arrows), None, "no ends");
    assert_eq!(outline.arrow_labels(arrows).count(), 0, "no label");
    assert_eq!(
        outline.holding(outline.premises().len()).count(),
        0,
        "no premise"
    );
}

fn with_premises(model: &mut Model, dice: &mut Dice) {
    let fields = model.fields.len();
    for rule in &mut model.rules {
        let given = Expr::Apply(
            Op::Equal,
            vec![Expr::Field(dice.below(fields)), Expr::Const(Value::Int(1))],
        );
        let implied = Expr::Or(
            Box::new(Expr::Apply(Op::Not, vec![given])),
            Box::new(rule.holds.expr().clone()),
        );
        rule.holds = implied.into();
    }
}

#[test]
fn the_premises_of_an_outline_are_those_of_its_graph_at_every_arrow_limit() {
    let mut with_a_premise = 0_usize;
    for seed in 0..MODELS {
        let mut dice = Dice::new(seed);
        let mut model = random_model(&mut dice);
        with_premises(&mut model, &mut dice);
        let (_, graph, outline) = searched(&model, Limits::default().max_states);
        assert_eq!(
            outline.premises(),
            graph.premises.as_slice(),
            "model {seed}"
        );
        for premise in 0..graph.premises.len() {
            let nodes: Vec<usize> = (0..graph.nodes.len())
                .filter(|at| graph.nodes[*at].premises.contains(&premise))
                .collect();
            assert_eq!(
                outline.holding(premise).collect::<Vec<_>>(),
                nodes,
                "model {seed}, premise {premise}"
            );
            with_a_premise += usize::from(!nodes.is_empty());
        }
        for limit in 0..=graph.edges.len().saturating_add(1) {
            assert_eq!(
                chart_outline(&outline, limit),
                chart_with_limit(&graph, limit),
                "model {seed} at {limit} arrows"
            );
        }
    }
    assert!(
        with_a_premise >= FEWEST_OF_EACH_KIND,
        "only {with_a_premise} premises that hold somewhere"
    );
}
