use crate::support::{edge, graph, node, read, start};
use earnest_chart::{ARROW_LIMIT, chart};
use earnest_check::{Edge, Graph, Node, Premise, PremiseKind};

const SIDE: usize = 6;
const FAR_CORNER: usize = SIDE * SIDE - 1;

fn dial() -> Graph {
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    for x in 0..SIDE {
        for y in 0..SIDE {
            nodes.push(node(&format!("x{x} y{y}")));
            if x + 1 < SIDE {
                edges.push(edge(x * SIDE + y, (x + 1) * SIDE + y, &["east"]));
            }
            if y + 1 < SIDE {
                edges.push(edge(x * SIDE + y, x * SIDE + y + 1, &["north"]));
            }
        }
    }
    nodes[0].start = true;
    graph(nodes, edges)
}

fn two_branches(length: usize) -> Graph {
    let mut nodes = vec![start("start")];
    let mut edges = Vec::new();
    for (name, first) in [("a", 1), ("b", 1 + length)] {
        for position in 0..length {
            nodes.push(node(&format!("{name}{position}")));
            let from = if position == 0 {
                0
            } else {
                first + position - 1
            };
            edges.push(edge(from, first + position, &["next"]));
        }
    }
    graph(nodes, edges)
}

fn premise(rule: &str, kind: PremiseKind) -> Premise {
    Premise {
        rule: rule.to_owned(),
        kind,
    }
}

fn holds(graph: &mut Graph, rule: &str, kind: PremiseKind, at: &[usize]) {
    let position = graph.premises.len();
    graph.premises.push(premise(rule, kind));
    for box_at in at {
        graph.nodes[*box_at].premises.push(position);
    }
}

fn pairs(arrows: &[(String, String, String)]) -> Vec<(&str, &str)> {
    arrows
        .iter()
        .map(|(from, to, _)| (from.as_str(), to.as_str()))
        .collect()
}

fn after_block(chart: &str) -> Vec<&str> {
    chart
        .split_once("\n```\n")
        .map_or_else(Vec::new, |(_, rest)| rest.split('\n').collect())
}

#[test]
fn the_route_to_a_premise_far_from_the_start_is_drawn_when_nothing_is_broken() {
    let mut dial = dial();
    holds(&mut dial, "paid", PremiseKind::Implies, &[FAR_CORNER]);
    let text = chart(&dial);
    let drawn = read(&text);
    assert_eq!(drawn.arrows.len(), ARROW_LIMIT);
    assert_eq!(drawn.label_of("s35"), Some("x5 y5".to_owned()));
    assert_eq!(
        pairs(&drawn.arrows)[..4],
        [("s0", "s6"), ("s0", "s1"), ("s1", "s7"), ("s1", "s2")],
        "the route goes first and the nearest arrows fill the rest"
    );
    assert!(
        drawn.reached().contains("s35"),
        "the premise is on a route from the start"
    );
    assert!(
        drawn.members("broken").is_empty(),
        "a premise is not a broken state"
    );
    assert_eq!(
        after_block(&text),
        [
            "",
            "Not shown: 15 of 36 states and 40 of 60 arrows.",
            "",
            "The premise of `paid` first holds in `x5 y5`."
        ]
    );
}

#[test]
fn without_a_premise_the_nearest_states_take_every_arrow() {
    let drawn = read(&chart(&dial()));
    assert_eq!(drawn.label_of("s35"), None, "the far corner is not reached");
    assert_eq!(
        drawn.note.as_deref(),
        Some("Not shown: 19 of 36 states and 40 of 60 arrows.")
    );
}

#[test]
fn broken_states_take_their_routes_before_a_premise() {
    let mut branches = two_branches(15);
    branches.nodes[15].broken = vec!["first".to_owned()];
    holds(&mut branches, "later", PremiseKind::Implies, &[30]);
    let text = chart(&branches);
    let drawn = read(&text);
    assert_eq!(drawn.arrows.len(), ARROW_LIMIT);
    assert_eq!(
        drawn.members("broken"),
        ["s15"],
        "the broken route is whole"
    );
    assert_eq!(
        drawn.label_of("s20"),
        Some("b4".to_owned()),
        "five arrows are left"
    );
    assert_eq!(drawn.label_of("s30"), None, "the premise does not fit");
    assert_eq!(
        after_block(&text).last(),
        Some(&"The premise of `later` first holds beyond what is shown.")
    );
}

#[test]
fn several_rules_share_what_is_left_in_the_order_they_are_declared() {
    let mut branches = two_branches(12);
    holds(&mut branches, "first", PremiseKind::Implies, &[24]);
    holds(&mut branches, "second", PremiseKind::Implies, &[12]);
    let text = chart(&branches);
    let drawn = read(&text);
    assert_eq!(drawn.arrows.len(), ARROW_LIMIT);
    assert_eq!(
        drawn.label_of("s24"),
        Some("b11".to_owned()),
        "the first gets its route"
    );
    assert_eq!(
        drawn.label_of("s8"),
        Some("a7".to_owned()),
        "the second gets the rest"
    );
    assert_eq!(drawn.label_of("s9"), None);
    assert_eq!(
        after_block(&text).last(),
        Some(
            &"The premise of `first` first holds in `b11`. The premise of `second` first holds \
              beyond what is shown."
        )
    );
}

#[test]
fn the_nearest_box_of_a_premise_is_the_one_routed_to() {
    let mut branches = two_branches(12);
    holds(&mut branches, "either", PremiseKind::Implies, &[11, 15]);
    let drawn = read(&chart(&branches));
    assert_eq!(drawn.label_of("s15"), Some("b2".to_owned()));
    assert_eq!(
        drawn.premises.as_deref(),
        Some("The premise of `either` first holds in `b2`."),
        "three arrows reach b2, eleven a10"
    );
}

#[test]
fn a_premise_that_holds_in_a_start_state_costs_no_arrow() {
    let mut dial = dial();
    holds(&mut dial, "always", PremiseKind::Implies, &[0, FAR_CORNER]);
    let text = chart(&dial);
    let plain = chart(&self::dial());
    assert_eq!(
        text,
        format!("{plain}\n\nThe premise of `always` first holds in `x0 y0`.")
    );
}

#[test]
fn a_premise_that_holds_nowhere_changes_nothing() {
    let mut dial = dial();
    holds(&mut dial, "never", PremiseKind::Implies, &[]);
    assert_eq!(chart(&dial), chart(&self::dial()));
}

#[test]
fn a_premise_only_an_unreachable_box_holds_changes_nothing() {
    let mut orphan = graph(
        vec![start("home"), node("orphan")],
        vec![edge(1, 0, &["arrive"])],
    );
    let plain = chart(&orphan);
    holds(&mut orphan, "lost", PremiseKind::Implies, &[1]);
    assert_eq!(chart(&orphan), plain);
}

#[test]
fn an_eventually_rule_names_where_it_starts_and_where_it_is_met() {
    let mut dial = dial();
    holds(&mut dial, "settles", PremiseKind::After, &[SIDE + 1]);
    holds(&mut dial, "settles", PremiseKind::Reaches, &[FAR_CORNER]);
    let text = chart(&dial);
    let drawn = read(&text);
    assert_eq!(drawn.label_of("s7"), Some("x1 y1".to_owned()));
    assert_eq!(drawn.label_of("s35"), Some("x5 y5".to_owned()));
    assert_eq!(
        after_block(&text).last(),
        Some(&"`settles` starts in `x1 y1` and is met in `x5 y5`.")
    );
}

#[test]
fn a_premise_already_on_a_broken_route_costs_nothing() {
    let mut branches = two_branches(12);
    branches.nodes[12].broken = vec!["deep".to_owned()];
    holds(&mut branches, "midway", PremiseKind::Implies, &[6, 20]);
    let drawn = read(&chart(&branches));
    assert_eq!(drawn.members("broken"), ["s12"]);
    assert_eq!(
        drawn.label_of("s20"),
        Some("b7".to_owned()),
        "the eight arrows left go to the nearest states"
    );
    assert!(chart(&branches).ends_with("The premise of `midway` first holds in `a5`."));
}
