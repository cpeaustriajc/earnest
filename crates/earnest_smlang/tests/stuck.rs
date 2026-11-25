//! A state where every listed event is refused is stuck unless the test marks it as an end.

use earnest_smlang::{DEFAULT_MAX_STATES, Snapshot, Status, explore, machine};

mod door {
    use smlang::statemachine;

    statemachine! {
        derive_states: [Clone, Debug, PartialOrd, Ord, Eq],
        derive_events: [Clone, Debug],
        transitions: {
            *Closed + Open [has_key] = Opened,
            Opened + Close = Closed,
        }
    }
}

use door::{Events, StateMachine, StateMachineContext, States};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Door {
    key: bool,
}

impl StateMachineContext for Door {
    fn has_key(&self) -> Result<bool, ()> {
        Ok(self.key)
    }
}

machine!(StateMachine<Door>);

#[test]
fn a_state_with_no_transition_for_any_listed_event_is_stuck() {
    let found = explore(
        &StateMachine::new(Door { key: true }),
        &[Events::Open],
        &[],
        |_| false,
        DEFAULT_MAX_STATES,
    );
    assert_eq!(found.status, Status::Fail, "a stuck state fails the search");
    assert_eq!(found.states, 2, "closed and opened");
    let stuck = found
        .stuck
        .expect("Close is not listed, so an open door takes no event");
    let [(Events::Open, opened)] = &stuck.steps[..] else {
        panic!("the door is stuck after one Open: {stuck:?}");
    };
    assert!(matches!(opened.state, States::Opened), "the door is open");
}

#[test]
fn a_state_where_the_guard_refuses_every_event_is_stuck() {
    let found = explore(
        &StateMachine::new(Door { key: false }),
        &[Events::Open, Events::Close],
        &[],
        |_| false,
        DEFAULT_MAX_STATES,
    );
    assert_eq!(found.status, Status::Fail, "a stuck state fails the search");
    assert_eq!(found.states, 1, "without the key the door stays closed");
    let stuck = found
        .stuck
        .expect("has_key is false and Close has no transition");
    assert!(stuck.steps.is_empty(), "the start state is stuck");
}

#[test]
fn a_stuck_state_marked_as_an_end_passes() {
    let found = explore(
        &StateMachine::new(Door { key: true }),
        &[Events::Open],
        &[],
        |now: &Snapshot<StateMachine<Door>>| matches!(now.state, States::Opened),
        DEFAULT_MAX_STATES,
    );
    assert_eq!(
        found.status,
        Status::Pass,
        "an open door is where the test ends"
    );
    assert_eq!(found.states, 2, "closed and opened");
    assert!(found.stuck.is_none(), "the end is not stuck");
}

#[test]
fn a_state_limit_leaves_the_search_unfinished() {
    let found = explore(
        &StateMachine::new(Door { key: true }),
        &[Events::Open, Events::Close],
        &[],
        |_| false,
        1,
    );
    assert_eq!(
        found.status,
        Status::Unfinished,
        "Open leads past the limit"
    );
    assert_eq!(found.reason.as_deref(), Some("state limit of 1"));
}

mod press {
    use smlang::statemachine;

    statemachine! {
        derive_states: [Clone, Debug, PartialOrd, Ord, Eq],
        derive_events: [Clone, Debug],
        transitions: {
            *Ready + Check [inspect] = Done,
            Ready + Force / bend = Done,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Sheet {
    bent: bool,
}

impl press::StateMachineContext for Sheet {
    fn inspect(&self) -> Result<bool, ()> {
        Err(())
    }

    fn bend(&mut self) -> Result<(), ()> {
        self.bent = true;
        Err(())
    }
}

machine!(press::StateMachine<Sheet>, press::States, press::Events);

#[test]
fn a_guard_or_an_action_that_fails_refuses_the_event_and_drops_its_changes() {
    let found = explore(
        &press::StateMachine::new(Sheet { bent: false }),
        &[press::Events::Check, press::Events::Force],
        &[],
        |_| false,
        DEFAULT_MAX_STATES,
    );
    assert_eq!(found.states, 1, "neither event leaves Ready");
    assert!(
        found.reached().all(|now| !now.context.bent),
        "the failed bend changed only the machine that was dropped"
    );
    let stuck = found.stuck.expect("both events are refused in Ready");
    assert!(stuck.steps.is_empty(), "the start state is stuck");
}
