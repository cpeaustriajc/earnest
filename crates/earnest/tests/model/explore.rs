use earnest::model::{
    DEFAULT_MAX_STATES, Model, Path, RuleResult, Status, Verdict, explore, model, replay, require,
};

fn moves_of<State, Edge: ToString>(path: &Path<State, Edge>) -> Vec<String> {
    path.steps
        .iter()
        .map(|(edge, _)| edge.to_string())
        .collect()
}

fn verdicts<State, Edge: ToString>(rules: &[RuleResult<State, Edge>]) -> Vec<(&str, String)> {
    rules
        .iter()
        .map(|rule| {
            let verdict = match &rule.verdict {
                Verdict::Holds => "holds".to_owned(),
                Verdict::Broken(path) => format!("broken by {}", moves_of(path).join(", ")),
                _ => "unknown".to_owned(),
            };
            (rule.id, verdict)
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Lamp {
    on: bool,
    #[range(0, 2)]
    #[initial(0)]
    level: u8,
}

#[model]
impl Lamp {
    #[action("the lamp is set to a level")]
    fn set(&self, #[range(1, 2)] to: u8) -> Option<Self> {
        require!(self.on && self.level != to);
        Some(Self {
            level: to,
            ..self.clone()
        })
    }

    #[action("the switch is flipped")]
    fn flip(&self) -> Self {
        Self {
            on: !self.on,
            ..self.clone()
        }
    }

    #[rule("A lamp that is off gives no light")]
    fn dark(&self) -> bool {
        self.on || self.level == 0
    }
}

fn off(level: u8) -> Lamp {
    Lamp { on: false, level }
}

fn lit(level: u8) -> Lamp {
    Lamp { on: true, level }
}

#[test]
fn a_model_is_searched_from_every_start_in_order_and_each_step_in_order() {
    let found = explore::<Lamp>(DEFAULT_MAX_STATES);
    assert_eq!(found.states, 6);
    let reached: Vec<Lamp> = found.reached().cloned().collect();
    assert_eq!(reached, [off(0), lit(0), lit(1), lit(2), off(1), off(2),]);
    assert_eq!(
        verdicts(&found.rules),
        [("dark", "broken by set[to = 1], flip".to_owned())]
    );
    assert_eq!(found.rules[0].says, "A lamp that is off gives no light");
    assert_eq!(found.status, Status::Fail);
    assert_eq!(found.reason, None);
    assert!(found.stuck.is_none(), "the switch can always be flipped");
}

#[test]
fn a_path_of_a_model_replays_to_the_states_it_holds() {
    let found = explore::<Lamp>(DEFAULT_MAX_STATES);
    let path = found.path_to(&off(2)).expect("the lamp can be off at 2");
    assert_eq!(path.start, lit(0));
    assert_eq!(moves_of(&path), ["set[to = 2]", "flip"]);
    let moves: Vec<_> = path.steps.iter().map(|(step, _)| step.clone()).collect();
    let states = replay(&path.start, &moves).expect("the path can be replayed");
    let held: Vec<Lamp> = std::iter::once(path.start.clone())
        .chain(path.steps.iter().map(|(_, state)| state.clone()))
        .collect();
    assert_eq!(states, held);
}

#[test]
fn a_limit_below_the_count_leaves_the_search_unfinished_and_no_rule_holding() {
    let found = explore::<Lamp>(4);
    assert_eq!(found.states, 4);
    assert_eq!(found.status, Status::Unfinished);
    assert_eq!(found.reason.as_deref(), Some("state limit of 4"));
    assert_eq!(found.rules[0].verdict, Verdict::Unknown);
    assert_eq!(found.path_to(&off(1)), None, "not counted");
}

#[test]
fn a_state_the_limit_kept_the_search_from_going_on_from_has_its_rules_judged() {
    let found = explore::<Lamp>(5);
    assert_eq!(found.states, 5);
    assert_eq!(found.reason.as_deref(), Some("state limit of 5"));
    assert_eq!(
        verdicts(&found.rules),
        [("dark", "broken by set[to = 1], flip".to_owned())]
    );
    assert_eq!(found.status, Status::Fail, "a broken rule fails the search");
}

#[test]
fn a_limit_of_zero_counts_no_state_and_still_lists_the_rules() {
    let found = explore::<Lamp>(0);
    assert_eq!(found.states, 0);
    assert_eq!(found.status, Status::Unfinished);
    assert_eq!(found.reason.as_deref(), Some("state limit of 0"));
    assert_eq!(verdicts(&found.rules), [("dark", "unknown".to_owned())]);
}

mod values {
    use super::moves_of;
    use earnest::model::{DEFAULT_MAX_STATES, Exploration, Rule, Status, Verdict, search};

    const TOP: u8 = 5;

    fn next(number: u8) -> Vec<(&'static str, u8)> {
        let mut moves = Vec::new();
        if number < TOP {
            moves.push(("add one", number + 1));
        }
        if number > 0 && number * 2 <= TOP {
            moves.push(("double", number * 2));
        }
        moves
    }

    fn numbers(
        starts: &[u8],
        is_end: impl FnMut(&u8) -> bool,
        limit: usize,
    ) -> Exploration<u8, &'static str> {
        let rules = [
            Rule::new("small", "The number is at most five", |n: &u8| *n <= TOP),
            Rule::new("four", "The number is never four", |n: &u8| *n != 4),
        ];
        search(starts.iter().copied(), |n| next(*n), &rules, is_end, limit)
    }

    #[test]
    fn a_search_over_numbers_counts_them_and_gives_the_shortest_path_to_each() {
        let found = numbers(&[0], |n| *n == TOP, DEFAULT_MAX_STATES);
        assert_eq!(found.states, 6);
        let reached: Vec<u8> = found.reached().copied().collect();
        assert_eq!(reached, [0, 1, 2, 3, 4, 5]);
        let paths: Vec<Vec<String>> = reached
            .iter()
            .map(|n| moves_of(&found.path_to(n).expect("every reached state has a path")))
            .collect();
        assert_eq!(
            paths,
            [
                vec![],
                vec!["add one"],
                vec!["add one", "add one"],
                vec!["add one", "add one", "add one"],
                vec!["add one", "add one", "double"],
                vec!["add one", "add one", "double", "add one"],
            ]
        );
        let to_five = found.path_to(&5).expect("five is reached");
        assert_eq!(to_five.start, 0);
        let after: Vec<u8> = to_five.steps.iter().map(|(_, n)| *n).collect();
        assert_eq!(after, [1, 2, 4, 5]);
        assert_eq!(found.path_to(&6), None, "six is never reached");
    }

    #[test]
    fn each_rule_gets_its_verdict_and_the_shortest_path_that_breaks_it() {
        let found = numbers(&[0], |n| *n == TOP, DEFAULT_MAX_STATES);
        assert_eq!(found.status, Status::Fail, "four is reached");
        assert_eq!(found.rules[0].verdict, Verdict::Holds);
        assert_eq!(found.rules[1].id, "four");
        assert_eq!(found.rules[1].says, "The number is never four");
        assert_eq!(
            found.rules[1].verdict,
            Verdict::Broken(found.path_to(&4).expect("four is reached"))
        );
        assert!(found.stuck.is_none(), "five is an end");
    }

    #[test]
    fn a_state_with_no_move_and_no_end_is_stuck() {
        let found = numbers(&[0], |_| false, DEFAULT_MAX_STATES);
        assert_eq!(found.status, Status::Fail);
        let stuck = found.stuck.expect("five has no move");
        assert_eq!(
            moves_of(&stuck),
            ["add one", "add one", "double", "add one"]
        );
    }

    #[test]
    fn an_end_is_asked_only_where_no_move_can_happen() {
        let mut asked = Vec::new();
        let found = numbers(
            &[0],
            |n| {
                asked.push(*n);
                true
            },
            DEFAULT_MAX_STATES,
        );
        assert!(found.stuck.is_none(), "five is an end");
        assert_eq!(asked, [TOP]);
    }

    #[test]
    fn start_states_are_searched_in_the_order_given() {
        let found = numbers(&[3, 0], |n| *n == TOP, DEFAULT_MAX_STATES);
        let reached: Vec<u8> = found.reached().copied().collect();
        assert_eq!(reached, [3, 0, 4, 1, 5, 2]);
        let to_four = found.path_to(&4).expect("four is reached");
        assert_eq!(
            (to_four.start, moves_of(&to_four)),
            (3, vec!["add one".to_owned()])
        );
    }

    #[test]
    fn a_limit_below_the_count_stops_before_more_states_than_the_limit() {
        let found = numbers(&[0], |n| *n == TOP, 4);
        assert_eq!(found.states, 4);
        assert_eq!(found.status, Status::Unfinished);
        assert_eq!(found.reason.as_deref(), Some("state limit of 4"));
        assert_eq!(found.rules[0].verdict, Verdict::Unknown);
        assert_eq!(found.rules[1].verdict, Verdict::Unknown);
        assert_eq!(found.path_to(&4), None, "four was not counted");
    }

    #[test]
    fn a_search_with_no_rules_and_no_start_state_passes_with_no_states() {
        let found = search(
            Vec::<u8>::new(),
            |n| next(*n),
            &[],
            |_: &u8| true,
            DEFAULT_MAX_STATES,
        );
        assert_eq!(found.states, 0);
        assert_eq!(found.status, Status::Pass);
        assert_eq!(found.rules.len(), 0);
    }

    #[test]
    fn a_rule_prints_its_id_and_sentence() {
        let rule = Rule::new("small", "The number is small", |n: &u8| *n < 3);
        let printed = format!("{rule:?}");
        assert!(
            printed.contains("small") && printed.contains("The number is small"),
            "{printed}"
        );
    }
}
