use crate::support::{edge, graph, node, read, start};
use earnest_chart::{ARROW_LIMIT, chart, chart_with_limit};
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
fn an_eventually_rule_names_where_it_starts_and_where_what_it_must_reach_holds() {
    let mut dial = dial();
    holds(&mut dial, "settles", PremiseKind::After, &[SIDE + 1]);
    holds(&mut dial, "settles", PremiseKind::Reaches, &[FAR_CORNER]);
    let text = chart(&dial);
    let drawn = read(&text);
    assert_eq!(drawn.label_of("s7"), Some("x1 y1".to_owned()));
    assert_eq!(drawn.label_of("s35"), Some("x5 y5".to_owned()));
    assert_eq!(
        drawn.premises.as_deref(),
        Some("`settles` starts in `x1 y1`; what it must reach holds in `x5 y5`.")
    );
}

fn line(boxes: usize) -> Graph {
    let mut nodes = vec![start("n = 0")];
    nodes.extend((1..boxes).map(|at| node(&format!("n = {at}"))));
    let edges = (1..boxes).map(|at| edge(at - 1, at, &["inc"])).collect();
    graph(nodes, edges)
}

#[test]
fn what_holds_only_before_a_rule_starts_and_a_start_that_holds_nowhere_are_not_named() {
    let mut counter = line(7);
    holds(&mut counter, "late", PremiseKind::After, &[4]);
    holds(&mut counter, "late", PremiseKind::Reaches, &[1]);
    holds(&mut counter, "never", PremiseKind::After, &[]);
    holds(&mut counter, "never", PremiseKind::Reaches, &[2]);
    let drawn = read(&chart(&counter));
    assert_eq!(
        drawn.premises.as_deref(),
        Some("`late` starts in `n = 4`."),
        "n = 1 can't be reached from n = 4, and `never` starts nowhere"
    );
}

#[test]
fn a_start_box_that_leads_to_no_box_of_what_must_be_reached_names_only_the_start() {
    let mut branches = two_branches(3);
    holds(&mut branches, "split", PremiseKind::After, &[2]);
    holds(&mut branches, "split", PremiseKind::Reaches, &[6]);
    let drawn = read(&chart_with_limit(&branches, 2));
    assert_eq!(pairs(&drawn.arrows), [("s0", "s1"), ("s1", "s2")]);
    assert_eq!(drawn.premises.as_deref(), Some("`split` starts in `a1`."));
}

#[test]
fn the_path_from_the_start_box_is_drawn_and_not_the_route_from_the_start_state() {
    let detour = graph(
        vec![start("home"), node("start"), node("goal"), node("detour")],
        vec![
            edge(0, 1, &["begin"]),
            edge(0, 2, &["shortcut"]),
            edge(1, 3, &["turn"]),
            edge(3, 2, &["arrive"]),
        ],
    );
    let mut detour = detour;
    holds(&mut detour, "goes", PremiseKind::After, &[1]);
    holds(&mut detour, "goes", PremiseKind::Reaches, &[2]);
    let drawn = read(&chart_with_limit(&detour, 3));
    assert_eq!(
        pairs(&drawn.arrows),
        [("s0", "s1"), ("s1", "s3"), ("s3", "s2")],
        "the shortcut from home is not the way from the start box"
    );
    assert_eq!(
        drawn.premises.as_deref(),
        Some("`goes` starts in `start`; what it must reach holds in `goal`.")
    );
}

#[test]
fn the_path_from_the_start_box_may_go_back_through_an_earlier_box() {
    let mut reset = line(3);
    reset.edges.push(edge(2, 0, &["reset"]));
    reset.nodes.push(node("aside"));
    reset.edges.push(edge(0, 3, &["aside"]));
    holds(&mut reset, "again", PremiseKind::After, &[2]);
    holds(&mut reset, "again", PremiseKind::Reaches, &[1]);
    let drawn = read(&chart_with_limit(&reset, 3));
    assert_eq!(
        pairs(&drawn.arrows),
        [("s0", "s1"), ("s1", "s2"), ("s2", "s0")],
        "two arrows to the start box and the reset back to n = 0, then on to n = 1"
    );
    assert_eq!(
        drawn.premises.as_deref(),
        Some("`again` starts in `n = 2`; what it must reach holds in `n = 1`.")
    );
}

#[test]
fn a_start_box_where_what_must_be_reached_holds_is_named_once() {
    let mut counter = line(4);
    holds(&mut counter, "there", PremiseKind::After, &[2]);
    holds(&mut counter, "there", PremiseKind::Reaches, &[2, 3]);
    assert_eq!(
        read(&chart(&counter)).premises.as_deref(),
        Some("`there` starts in `n = 2`, where what it must reach holds too.")
    );
}

#[test]
fn an_eventually_rule_without_a_start_names_where_what_it_must_reach_first_holds() {
    let mut counter = line(4);
    holds(&mut counter, "grows", PremiseKind::Reaches, &[3]);
    assert_eq!(
        read(&chart(&counter)).premises.as_deref(),
        Some("What `grows` must reach first holds in `n = 3`.")
    );
}

#[test]
fn a_path_from_the_start_box_that_does_not_fit_is_beyond_what_is_shown() {
    let mut counter = line(30);
    holds(&mut counter, "slow", PremiseKind::After, &[2]);
    holds(&mut counter, "slow", PremiseKind::Reaches, &[29]);
    holds(&mut counter, "far", PremiseKind::After, &[25]);
    holds(&mut counter, "far", PremiseKind::Reaches, &[26]);
    assert_eq!(
        read(&chart(&counter)).premises.as_deref(),
        Some(
            "`slow` starts in `n = 2`; what it must reach holds beyond what is shown. `far` \
             starts beyond what is shown."
        )
    );
}

#[test]
fn the_nearest_box_of_a_premise_is_named_even_when_a_farther_one_is_drawn_first() {
    let mut branches = two_branches(12);
    branches.nodes[12].broken = vec!["deep".to_owned()];
    holds(&mut branches, "near", PremiseKind::Implies, &[5, 15]);
    let drawn = read(&chart(&branches));
    assert_eq!(
        drawn.label_of("s5"),
        Some("a4".to_owned()),
        "on the broken route"
    );
    assert_eq!(
        drawn.premises.as_deref(),
        Some("The premise of `near` first holds in `b2`."),
        "b2 is three arrows from the start, a4 five"
    );
}

#[test]
fn a_premise_route_that_does_not_fit_takes_no_arrow_and_the_next_one_is_drawn() {
    for far_first in [true, false] {
        let mut branches = two_branches(25);
        let rules = [("far", 25), ("near", 28)];
        let ordered: Vec<(&str, usize)> = if far_first {
            rules.to_vec()
        } else {
            rules.iter().rev().copied().collect()
        };
        for (rule, at) in &ordered {
            holds(&mut branches, rule, PremiseKind::Implies, &[*at]);
        }
        let drawn = read(&chart(&branches));
        assert_eq!(drawn.arrows.len(), ARROW_LIMIT, "far first: {far_first}");
        assert_eq!(drawn.label_of("s28"), Some("b2".to_owned()));
        assert_eq!(
            drawn.label_of("s17"),
            Some("a16".to_owned()),
            "the seventeen arrows left go to the nearest states, far first: {far_first}"
        );
        assert_eq!(
            drawn.label_of("s25"),
            None,
            "the far premise takes no arrow, far first: {far_first}"
        );
        let far = "The premise of `far` first holds beyond what is shown.";
        let near = "The premise of `near` first holds in `b2`.";
        let expected = if far_first {
            format!("{far} {near}")
        } else {
            format!("{near} {far}")
        };
        assert_eq!(drawn.premises, Some(expected), "far first: {far_first}");
    }
}

#[test]
fn a_long_label_is_cut_in_the_note_and_kept_whole_in_the_chart() {
    let fields: Vec<String> = (0..40).map(|at| format!("f{at} = {at}")).collect();
    let long = fields.join(", ");
    let mut counter = graph(vec![start("home"), node(&long)], vec![edge(0, 1, &["go"])]);
    holds(&mut counter, "wide", PremiseKind::Implies, &[1]);
    let drawn = read(&chart(&counter));
    assert_eq!(
        drawn.label_of("s1"),
        Some(long.clone()),
        "the chart keeps it"
    );
    let note = drawn.premises.expect("a note");
    let span = note
        .strip_prefix("The premise of `wide` first holds in `")
        .and_then(|rest| rest.strip_suffix("`."))
        .unwrap_or_else(|| panic!("one code span in `{note}`"));
    assert_eq!(span.chars().count(), 80, "{span}");
    assert!(span.ends_with('\u{2026}'), "{span}");
    assert!(
        long.starts_with(span.trim_end_matches('\u{2026}')),
        "{span}"
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
