//! Checks every state an [smlang](https://docs.rs/smlang/0.8) state machine can reach, from a
//! Rust test, with the search of [`earnest::model::search`].
//!
//! The test keeps its `statemachine!` block, adds the derives the search needs, connects the
//! generated types with [`machine!`], and hands [`explore`] a machine and the events to try. A
//! searched state is a [`Snapshot`]: the machine's state and its context. A fuse that blows
//! once and is replaced by an event the test does not list is stuck after `Blow`:
//!
//! ```
//! use earnest_smlang::{DEFAULT_MAX_STATES, Status, explore, machine};
//! use smlang::statemachine;
//!
//! statemachine! {
//!     derive_states: [Clone, Debug, PartialOrd, Ord, Eq],
//!     derive_events: [Clone, Debug],
//!     transitions: {
//!         *Intact + Blow = Blown,
//!         Blown + Replace = Intact,
//!     }
//! }
//!
//! #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//! struct Fuse;
//!
//! impl StateMachineContext for Fuse {}
//!
//! machine!(StateMachine<Fuse>);
//!
//! let fuse = StateMachine::new(Fuse);
//! let found = explore(&fuse, &[Events::Blow], &[], |_| false, DEFAULT_MAX_STATES);
//! assert_eq!(found.status, Status::Fail);
//! assert_eq!(found.states, 2);
//! let stuck = found.stuck.expect("a blown fuse takes no Blow");
//! assert!(matches!(stuck.steps[..], [(Events::Blow, _)]));
//! ```

use earnest::model::search;
use std::fmt::{self, Write as _};
use std::hash::{Hash, Hasher};
use std::mem;

pub use earnest::model::{
    DEFAULT_MAX_STATES, Exploration, Path, Rule, RuleResult, Status, Verdict,
};

/// The parts of an smlang `StateMachine` that [`explore`] uses. smlang generates its types
/// anew in each `statemachine!` call and gives them no shared trait, so write this impl with
/// [`machine!`] rather than by hand.
pub trait Machine {
    /// The generated `States` enum.
    type State;
    /// The context struct that implements the generated `StateMachineContext`.
    type Context;
    /// The generated `Events` enum.
    type Event;

    /// A machine in `state` with `context`, as `StateMachine::new_with_state` builds it.
    fn resume(context: Self::Context, state: Self::State) -> Self;

    /// The state the machine is in.
    fn state(&self) -> &Self::State;

    /// The machine's context.
    fn context(&self) -> &Self::Context;

    /// Processes `event` as `process_event` does, and gives whether it gave `Ok`. A machine that
    /// gave `Err` is dropped, so what a failed action changed in the context does not matter.
    fn take(&mut self, event: Self::Event) -> bool;
}

/// Implements [`Machine`] for a `StateMachine` that smlang 0.8's `statemachine!` generated.
///
/// `machine!(StateMachine<Context>)` takes the `States` and `Events` in scope. A machine named
/// with `name: Door` generates `DoorStateMachine`, `DoorStates` and `DoorEvents`, so name the
/// types: `machine!(DoorStateMachine<Context>, DoorStates, DoorEvents)`. A path works for each
/// name, `door::StateMachine<Context>, door::States, door::Events`. Machines with async guards
/// or actions, a temporary context, or states or events that hold references are not
/// supported.
#[macro_export]
macro_rules! machine {
    ($($machine:ident)::+ < $context:ty >) => {
        $crate::machine!($($machine)::+ <$context>, States, Events);
    };
    ($($machine:ident)::+ < $context:ty >, $state:ty, $event:ty) => {
        impl $crate::Machine for $($machine)::+ <$context> {
            type State = $state;
            type Context = $context;
            type Event = $event;

            fn resume(context: $context, state: $state) -> Self {
                Self::new_with_state(context, state)
            }

            fn state(&self) -> &$state {
                $($machine)::+::state(self)
            }

            fn context(&self) -> &$context {
                $($machine)::+::context(self)
            }

            fn take(&mut self, event: $event) -> bool {
                self.process_event(event).is_ok()
            }
        }
    };
}

/// One searched state: the machine's state and its context.
///
/// Two snapshots are the same when their states are equal by `Ord` and their contexts by
/// `PartialEq`. smlang writes `PartialEq` for `States` by hand and compares only the variant,
/// so `Running(1)` equals `Running(2)` there; `Ord` from `derive_states: [PartialOrd, Ord, Eq]`
/// compares the data too, and keeps them apart. A hand-written `Ord` that ignores the data
/// merges such states. Clippy refuses a derived `Hash` beside smlang's `PartialEq`, so a snapshot
/// hashes the state's `Debug` text and the context: two states that `Ord` calls equal must print
/// the same. Derived `Ord` and `Debug` agree when the data's own do, which holds for integers,
/// strings, `bool` and derived types, and not for a field whose `Ord` ignores what its `Debug`
/// prints, such as a case-insensitive string.
pub struct Snapshot<M: Machine> {
    /// The state the machine is in.
    pub state: M::State,
    /// The machine's context.
    pub context: M::Context,
}

impl<M> Snapshot<M>
where
    M: Machine,
    M::State: Clone,
    M::Context: Clone,
{
    /// The state and the context `machine` holds now, to compare with a state on a path after
    /// replaying it on a real machine.
    pub fn of(machine: &M) -> Self {
        Self {
            state: machine.state().clone(),
            context: machine.context().clone(),
        }
    }
}

impl<M> Clone for Snapshot<M>
where
    M: Machine,
    M::State: Clone,
    M::Context: Clone,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            context: self.context.clone(),
        }
    }
}

impl<M> PartialEq for Snapshot<M>
where
    M: Machine,
    M::State: Ord,
    M::Context: Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.state.cmp(&other.state).is_eq() && self.context == other.context
    }
}

impl<M> Eq for Snapshot<M>
where
    M: Machine,
    M::State: Ord,
    M::Context: Eq,
{
}

impl<M> Hash for Snapshot<M>
where
    M: Machine,
    M::State: fmt::Debug,
    M::Context: Hash,
{
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        if write!(HashText(hasher), "{:?}", self.state).is_err() {
            mem::discriminant(&self.state).hash(hasher);
        }
        self.context.hash(hasher);
    }
}

struct HashText<'a, H>(&'a mut H);

impl<H: Hasher> fmt::Write for HashText<'_, H> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write(text.as_bytes());
        Ok(())
    }
}

impl<M> fmt::Debug for Snapshot<M>
where
    M: Machine,
    M::State: fmt::Debug,
    M::Context: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("state", &self.state)
            .field("context", &self.context)
            .finish()
    }
}

/// Searches every state `start` can reach by the listed `events`, breadth first, and gives the
/// state count, each rule's verdict with the shortest path of events that breaks it, the first
/// stuck state, and [`Exploration::path_to`] for a shortest path to any state reached.
///
/// From each snapshot it tries the events in the order listed: it builds a machine with
/// `new_with_state` from clones of the state and context, and an event that gives `Ok` from
/// `process_event` moves to that machine's new state and context. An `Err` (an event the state
/// has no transition for, a guard that is false or fails, a failed action) means the event
/// can't happen there. List an event that carries data once for each value to try. A snapshot
/// where every listed event is refused is stuck unless `end` is true there. The search stops
/// before it holds more than `max_states` snapshots, as [`search`] does; a panic in a guard, an
/// action or a rule is not caught. smlang's `PartialEq` on `Events` compares only the variant
/// too, so check the events of a path with `matches!`: `[Events::Set(1)]` equals
/// `[Events::Set(2)]` by `==`.
pub fn explore<M>(
    start: &M,
    events: &[M::Event],
    rules: &[Rule<'_, Snapshot<M>>],
    end: impl FnMut(&Snapshot<M>) -> bool,
    max_states: usize,
) -> Exploration<Snapshot<M>, M::Event>
where
    M: Machine,
    M::State: Clone + Ord + fmt::Debug,
    M::Context: Clone + Eq + Hash,
    M::Event: Clone,
{
    search(
        [Snapshot::of(start)],
        |now: &Snapshot<M>| {
            events
                .iter()
                .filter_map(|event| {
                    let mut machine = M::resume(now.context.clone(), now.state.clone());
                    machine
                        .take(event.clone())
                        .then(|| (event.clone(), Snapshot::of(&machine)))
                })
                .collect::<Vec<_>>()
        },
        rules,
        end,
        max_states,
    )
}
