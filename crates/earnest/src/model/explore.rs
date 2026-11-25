use super::{Behavior, Model, Move};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::hash::Hash;

/// The state limit to pass to [`explore`] and [`search`] for the same limit as `earnest
/// verify` uses when it is given none.
pub const DEFAULT_MAX_STATES: usize = 5_000_000;

/// A start state and the moves that lead from it, each with the state after it.
///
/// [`explore`] gives its moves as [`Move`] values, so `replay(&path.start, &moves)` walks the
/// same path again and `move.to_string()` reads as the report writes a step.
///
/// The conformance model `tasks/conformance/models/mailbox.rs` has 7 states and breaks
/// `bills` after two deliveries of a bill:
///
/// ```
/// include!("../../../../tasks/conformance/models/mailbox.rs");
///
/// use earnest::model::{DEFAULT_MAX_STATES, Status, Verdict, explore};
///
/// fn main() {
///     let found = explore::<Mailbox>(DEFAULT_MAX_STATES);
///     assert_eq!(found.status, Status::Fail);
///     assert_eq!(found.states, 7);
///     assert_eq!(found.rules[1].id, "fronted");
///     assert_eq!(found.rules[1].verdict, Verdict::Holds);
///     assert_eq!(found.rules[0].id, "bills");
///     let Verdict::Broken(path) = &found.rules[0].verdict else {
///         panic!("an inbox of two bills holds no card");
///     };
///     let moves: Vec<String> = path.steps.iter().map(|(step, _)| step.to_string()).collect();
///     assert_eq!(moves, [r#"deliver[letter = "Bill"]"#, r#"deliver[letter = "Bill"]"#]);
///     assert!(!path.steps[1].1.bills());
///     assert!(found.stuck.is_none());
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path<State, Edge> {
    /// The start state the path leaves from.
    pub start: State,
    /// Each move in order, with the state it leads to. Empty when the path ends at its start.
    pub steps: Vec<(Edge, State)>,
}

/// What the search found for one rule.
///
/// The conformance model `tasks/conformance/models/oven.rs` breaks `safe` after `start`,
/// `open` and `closed` after `open`:
///
/// ```
/// include!("../../../../tasks/conformance/models/oven.rs");
///
/// use earnest::model::{DEFAULT_MAX_STATES, Status, Verdict, explore};
///
/// fn main() {
///     let found = explore::<Oven>(DEFAULT_MAX_STATES);
///     assert_eq!(found.status, Status::Fail);
///     assert_eq!(found.states, 4);
///     let breaking = |id: &str| -> Vec<String> {
///         let rule = found.rules.iter().find(|rule| rule.id == id).expect("the oven has it");
///         match &rule.verdict {
///             Verdict::Broken(path) => path.steps.iter().map(|(step, _)| step.to_string()).collect(),
///             _ => Vec::new(),
///         }
///     };
///     assert_eq!(breaking("safe"), ["start", "open"]);
///     assert_eq!(breaking("closed"), ["open"]);
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict<State, Edge> {
    /// No state the search reached breaks the rule, and it reached every state.
    Holds,
    /// A state breaks the rule: the path is the shortest one to such a state, and the first
    /// found in breadth-first order when several are as short.
    Broken(Path<State, Edge>),
    /// No state it reached breaks the rule, but the state limit stopped it before it reached
    /// them all.
    Unknown,
}

/// One rule's verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleResult<State, Edge> {
    /// The rule's id: the name of its method for a model.
    pub id: &'static str,
    /// The rule's sentence.
    pub says: &'static str,
    /// Whether it holds, the path that breaks it, or that the search can't tell.
    pub verdict: Verdict<State, Edge>,
}

/// How the search ended, as `earnest verify` reports it.
///
/// The conformance model `tasks/conformance/models/counter.rs` passes with 4 states, and is
/// unfinished with a limit of 3:
///
/// ```
/// include!("../../../../tasks/conformance/models/counter.rs");
///
/// use earnest::model::{DEFAULT_MAX_STATES, Status, Verdict, explore};
///
/// fn main() {
///     let found = explore::<Counter>(DEFAULT_MAX_STATES);
///     assert_eq!(found.status, Status::Pass);
///     assert_eq!(found.states, 4);
///     let ids: Vec<&str> = found.rules.iter().map(|rule| rule.id).collect();
///     assert_eq!(ids, ["bounded", "even"]);
///     assert!(found.rules.iter().all(|rule| rule.verdict == Verdict::Holds));
///     assert!(found.stuck.is_none() && found.reason.is_none());
///
///     let cut = explore::<Counter>(3);
///     assert_eq!(cut.status, Status::Unfinished);
///     assert_eq!(cut.states, 3);
///     assert_eq!(cut.reason.as_deref(), Some("state limit of 3"));
///     assert!(cut.rules.iter().all(|rule| rule.verdict == Verdict::Unknown));
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// Every state was reached, no rule is broken and no state is stuck. [`explore`] does not
    /// look for a state out of range, so a model that has one can pass here and fail
    /// `earnest verify`.
    Pass,
    /// A rule is broken or a state is stuck, whether or not the limit stopped the search.
    Fail,
    /// The state limit stopped the search before it found anything wrong.
    Unfinished,
}

/// A rule for [`search`]: its id, its sentence, and the test that says whether it holds in a
/// state.
pub struct Rule<'a, State> {
    id: &'static str,
    says: &'static str,
    holds: Box<dyn Fn(&State) -> bool + 'a>,
}

impl<'a, State> Rule<'a, State> {
    /// A rule with this id and sentence that holds in a state where `holds` gives `true`.
    #[must_use]
    pub fn new(id: &'static str, says: &'static str, holds: impl Fn(&State) -> bool + 'a) -> Self {
        Self {
            id,
            says,
            holds: Box::new(holds),
        }
    }
}

impl<State> fmt::Debug for Rule<'_, State> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Rule")
            .field("id", &self.id)
            .field("says", &self.says)
            .finish_non_exhaustive()
    }
}

/// What [`explore`] or [`search`] found.
///
/// Every counted state has its rules judged, also the states the limit kept the search from
/// going on from, so a rule can be broken in an unfinished search.
///
/// The conformance model `tasks/conformance/models/download.rs` has 4 states and is stuck
/// after `start`, `fail`:
///
/// ```
/// include!("../../../../tasks/conformance/models/download.rs");
///
/// use earnest::model::{DEFAULT_MAX_STATES, Status, Verdict, explore};
///
/// fn main() {
///     let found = explore::<Download>(DEFAULT_MAX_STATES);
///     assert_eq!(found.status, Status::Fail);
///     assert_eq!(found.states, 4);
///     assert_eq!(found.rules[0].id, "whole");
///     assert_eq!(found.rules[0].verdict, Verdict::Holds);
///     let stuck = found.stuck.expect("a failed download can't go on and is not done");
///     let moves: Vec<String> = stuck.steps.iter().map(|(step, _)| step.to_string()).collect();
///     assert_eq!(moves, ["start", "fail"]);
///     assert_eq!(stuck.steps[1].1, Download { phase: Phase::Failed, saved: false });
/// }
/// ```
pub struct Exploration<State, Edge> {
    /// How the search ended.
    pub status: Status,
    /// How many distinct states it reached, start states included.
    pub states: usize,
    /// Each rule's verdict, in the order the rules were given.
    pub rules: Vec<RuleResult<State, Edge>>,
    /// The shortest path to the first state found in breadth-first order where no move can
    /// happen and the end is false, or `None` when there is no such state.
    pub stuck: Option<Path<State, Edge>>,
    /// Why an unfinished search stopped, `state limit of 5000000`, or `None` when it reached
    /// every state.
    pub reason: Option<String>,
    visits: Vec<Visit<State, Edge>>,
    index: HashMap<State, usize>,
}

impl<State, Edge> Exploration<State, Edge> {
    /// Every counted state, each once, in the order the search reached them: breadth first,
    /// start states first.
    pub fn reached(&self) -> impl Iterator<Item = &State> {
        self.visits.iter().map(|visit| &visit.state)
    }
}

impl<State: Clone + Eq + Hash, Edge: Clone> Exploration<State, Edge> {
    /// The shortest path to a counted state, the first found in breadth-first order when
    /// several are as short, or `None` when the search did not count the state.
    pub fn path_to(&self, state: &State) -> Option<Path<State, Edge>> {
        self.index
            .get(state)
            .and_then(|&at| path_to(&self.visits, at))
    }
}

impl<State, Edge> fmt::Debug for Exploration<State, Edge>
where
    State: fmt::Debug,
    Edge: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Exploration")
            .field("status", &self.status)
            .field("states", &self.states)
            .field("rules", &self.rules)
            .field("stuck", &self.stuck)
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

/// Searches every state a model can reach, breadth first, and gives the count of distinct
/// states, each rule's verdict with the shortest path to a state that breaks it, and the first
/// stuck state.
///
/// It visits states in the order `earnest verify` does: start states in `M::starts()` order,
/// steps in `state.steps()` order, so the paths are the ones the report gives. An end is asked
/// only in a state where no action can happen. The search stops before it holds more than
/// `max_states` distinct states; pass [`DEFAULT_MAX_STATES`] for the checker's own limit. A
/// panic in an action, a rule or an end is not caught, and a field outside its `#[range]` or
/// `#[one_of]` is searched on from, where `earnest verify` reports the state as out of range.
/// The rules are read from the first start state, so a model with no start state lists none.
///
/// The conformance model `tasks/conformance/models/turnstile.rs` has 4 states and breaks
/// `locked` after one `push`:
///
/// ```
/// include!("../../../../tasks/conformance/models/turnstile.rs");
///
/// use earnest::model::{DEFAULT_MAX_STATES, Status, Verdict, explore, replay};
///
/// fn main() {
///     let found = explore::<Turnstile>(DEFAULT_MAX_STATES);
///     assert_eq!(found.status, Status::Fail);
///     assert_eq!(found.states, 4);
///     assert_eq!(found.rules[0].id, "locked");
///     let Verdict::Broken(path) = &found.rules[0].verdict else {
///         panic!("a push opens the gate without payment");
///     };
///     let moves: Vec<_> = path.steps.iter().map(|(step, _)| step.clone()).collect();
///     assert_eq!(moves.iter().map(ToString::to_string).collect::<Vec<_>>(), ["push"]);
///     let states = replay(&path.start, &moves).expect("the path is one the model can take");
///     assert!(!states[1].locked());
///
///     let cut = explore::<Turnstile>(1);
///     assert_eq!(cut.status, Status::Unfinished);
///     assert_eq!(cut.reason.as_deref(), Some("state limit of 1"));
///     assert_eq!(cut.rules[0].verdict, Verdict::Unknown);
/// }
/// ```
pub fn explore<M>(max_states: usize) -> Exploration<M, Move>
where
    M: Model + Behavior + Clone + Eq + Hash,
{
    let starts = M::starts();
    let names = starts.first().map_or_else(Vec::new, |start| {
        start
            .rules()
            .iter()
            .map(|rule| (rule.id, rule.says))
            .collect()
    });
    breadth_first(
        starts,
        |state: &M| {
            state
                .steps()
                .into_iter()
                .map(|step| (Move::taken(&step), step.state))
        },
        Judge {
            names,
            holds: |state: &M| state.rules().iter().map(|rule| rule.holds).collect(),
        },
        |state: &M| state.ends().iter().any(|end| end.holds),
        max_states,
    )
}

/// Searches every state reachable from `starts` with the same rules and order as [`explore`],
/// for states it can't look inside: a state needs only `Clone`, `Eq` and `Hash`.
///
/// `moves` gives every move that can happen in a state, each with the state after it, in the
/// order the search should try them; a state that has none is stuck unless `end` is true
/// there. Start states are taken in the order given. Each rule is judged in every counted
/// state. [`Exploration::path_to`] gives the shortest path to any state reached.
///
/// The conformance model `tasks/conformance/models/download.rs`, searched with its actions'
/// ids as the moves, is stuck after `start`, `fail`:
///
/// ```
/// include!("../../../../tasks/conformance/models/download.rs");
///
/// use earnest::model::{Behavior, DEFAULT_MAX_STATES, Rule, Status, Verdict, search};
///
/// fn main() {
///     let found = search(
///         Download::starts(),
///         |state: &Download| {
///             let steps = state.steps().into_iter();
///             steps.map(|step| (step.action, step.state)).collect::<Vec<_>>()
///         },
///         &[Rule::new("whole", "Only a finished download is saved", Download::whole)],
///         Download::done,
///         DEFAULT_MAX_STATES,
///     );
///     assert_eq!(found.status, Status::Fail);
///     assert_eq!(found.states, 4);
///     assert_eq!(found.rules[0].verdict, Verdict::Holds);
///     let stuck = found.stuck.expect("a failed download can't go on and is not done");
///     let moves: Vec<&str> = stuck.steps.iter().map(|(action, _)| *action).collect();
///     assert_eq!(moves, ["start", "fail"]);
/// }
/// ```
pub fn search<State, Edge, Next>(
    starts: impl IntoIterator<Item = State>,
    moves: impl FnMut(&State) -> Next,
    rules: &[Rule<'_, State>],
    end: impl FnMut(&State) -> bool,
    max_states: usize,
) -> Exploration<State, Edge>
where
    State: Clone + Eq + Hash,
    Edge: Clone,
    Next: IntoIterator<Item = (Edge, State)>,
{
    breadth_first(
        starts,
        moves,
        Judge {
            names: rules.iter().map(|rule| (rule.id, rule.says)).collect(),
            holds: |state: &State| rules.iter().map(|rule| (rule.holds)(state)).collect(),
        },
        end,
        max_states,
    )
}

struct Visit<State, Edge> {
    state: State,
    from: Option<(usize, Edge)>,
}

struct Judge<Holds> {
    names: Vec<(&'static str, &'static str)>,
    holds: Holds,
}

struct Search<State, Edge> {
    visits: Vec<Visit<State, Edge>>,
    index: HashMap<State, usize>,
    max_states: usize,
    reason: Option<String>,
    first_broken: Vec<Option<usize>>,
    stuck: Option<usize>,
}

impl<State: Clone + Eq + Hash, Edge> Search<State, Edge> {
    fn visit(&mut self, state: State, from: Option<(usize, Edge)>) -> bool {
        let count = self.visits.len();
        match self.index.entry(state) {
            Entry::Occupied(_) => true,
            Entry::Vacant(_) if count >= self.max_states => {
                self.reason = Some(format!("state limit of {}", self.max_states));
                false
            }
            Entry::Vacant(slot) => {
                self.visits.push(Visit {
                    state: slot.key().clone(),
                    from,
                });
                slot.insert(count);
                true
            }
        }
    }

    fn judge(&mut self, at: usize, holds: &mut impl FnMut(&State) -> Vec<bool>) {
        let Some(visit) = self.visits.get(at) else {
            return;
        };
        for (first, held) in self.first_broken.iter_mut().zip(holds(&visit.state)) {
            if !held && first.is_none() {
                *first = Some(at);
            }
        }
    }
}

fn breadth_first<State, Edge, Next>(
    starts: impl IntoIterator<Item = State>,
    mut moves: impl FnMut(&State) -> Next,
    mut judge: Judge<impl FnMut(&State) -> Vec<bool>>,
    mut end: impl FnMut(&State) -> bool,
    max_states: usize,
) -> Exploration<State, Edge>
where
    State: Clone + Eq + Hash,
    Edge: Clone,
    Next: IntoIterator<Item = (Edge, State)>,
{
    let mut search = Search {
        visits: Vec::new(),
        index: HashMap::new(),
        max_states,
        reason: None,
        first_broken: vec![None; judge.names.len()],
        stuck: None,
    };
    for start in starts {
        if !search.visit(start, None) {
            break;
        }
    }
    let mut searched = 0;
    while search.reason.is_none() {
        let Some(visit) = search.visits.get(searched) else {
            break;
        };
        let next: Vec<(Edge, State)> = moves(&visit.state).into_iter().collect();
        let is_stuck = next.is_empty() && !end(&visit.state);
        let at = searched;
        searched += 1;
        search.judge(at, &mut judge.holds);
        if is_stuck {
            search.stuck.get_or_insert(at);
        }
        for (edge, state) in next {
            if !search.visit(state, Some((at, edge))) {
                break;
            }
        }
    }
    for counted in searched..search.visits.len() {
        search.judge(counted, &mut judge.holds);
    }
    outcome(search, judge.names)
}

fn outcome<State: Clone, Edge: Clone>(
    search: Search<State, Edge>,
    names: Vec<(&'static str, &'static str)>,
) -> Exploration<State, Edge> {
    let path = |at: Option<usize>| at.and_then(|at| path_to(&search.visits, at));
    let rules: Vec<RuleResult<State, Edge>> = names
        .into_iter()
        .zip(&search.first_broken)
        .map(|((id, says), &first)| {
            let verdict = match path(first) {
                Some(path) => Verdict::Broken(path),
                None if search.reason.is_none() => Verdict::Holds,
                None => Verdict::Unknown,
            };
            RuleResult { id, says, verdict }
        })
        .collect();
    let stuck = path(search.stuck);
    let has_failed = stuck.is_some()
        || rules
            .iter()
            .any(|rule| matches!(rule.verdict, Verdict::Broken(_)));
    let status = if has_failed {
        Status::Fail
    } else if search.reason.is_none() {
        Status::Pass
    } else {
        Status::Unfinished
    };
    Exploration {
        status,
        states: search.visits.len(),
        rules,
        stuck,
        reason: search.reason,
        visits: search.visits,
        index: search.index,
    }
}

fn path_to<State: Clone, Edge: Clone>(
    visits: &[Visit<State, Edge>],
    target: usize,
) -> Option<Path<State, Edge>> {
    let mut steps = Vec::new();
    let mut at = visits.get(target)?;
    while let Some((parent, edge)) = &at.from {
        steps.push((edge.clone(), at.state.clone()));
        at = visits.get(*parent)?;
    }
    steps.reverse();
    Some(Path {
        start: at.state.clone(),
        steps,
    })
}
