//! States and events that carry data. smlang's `PartialEq` compares only the variant, so the
//! search compares states by `Ord`, which keeps `Showing(1)` and `Showing(2)` apart.

use earnest_smlang::{DEFAULT_MAX_STATES, Snapshot, Status, explore, machine};

mod dial {
    use smlang::statemachine;

    statemachine! {
        name: Dial,
        derive_states: [Clone, Debug, PartialOrd, Ord, Eq],
        derive_events: [Clone, Debug],
        transitions: {
            *Idle + Set(u8) [in_range] / show = Showing(u8),
            Showing(u8) + Clear = Idle,
        }
    }

    pub(crate) const TOP: u8 = 5;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct Face;

    impl DialStateMachineContext for Face {
        fn in_range(&self, value: &u8) -> Result<bool, ()> {
            Ok(*value < TOP)
        }

        fn show(&mut self, value: u8) -> Result<u8, ()> {
            Ok(value)
        }
    }
}

use dial::{DialEvents, DialStateMachine, DialStates, Face, TOP};

machine!(DialStateMachine<Face>, DialStates, DialEvents);

const EVENTS: [DialEvents; 4] = [
    DialEvents::Set(1),
    DialEvents::Set(2),
    DialEvents::Set(TOP),
    DialEvents::Clear,
];

fn check() -> earnest_smlang::Exploration<Snapshot<DialStateMachine<Face>>, DialEvents> {
    explore(
        &DialStateMachine::new(Face),
        &EVENTS,
        &[],
        |_| false,
        DEFAULT_MAX_STATES,
    )
}

#[test]
fn three_states_idle_and_one_for_each_value_in_range() {
    let found = check();
    assert_eq!(found.status, Status::Pass, "every state takes an event");
    let reached: Vec<&DialStates> = found.reached().map(|now| &now.state).collect();
    assert!(
        matches!(
            reached[..],
            [
                DialStates::Idle,
                DialStates::Showing(1),
                DialStates::Showing(2)
            ]
        ),
        "Set({TOP}) is out of range and refused: {reached:?}"
    );
    assert_eq!(found.states, 3, "Showing(1) and Showing(2) are not merged");
}

#[test]
fn the_path_to_a_state_names_the_value_it_was_set_to() {
    let found = check();
    let target = Snapshot {
        state: DialStates::Showing(2),
        context: Face,
    };
    let path = found.path_to(&target).expect("Set(2) reaches it");
    assert!(
        matches!(path.steps[..], [(DialEvents::Set(2), _)]),
        "one event: {path:?}"
    );
}

#[test]
fn every_path_replays_on_a_fresh_machine_with_its_data() {
    let found = check();
    for target in found.reached() {
        let path = found.path_to(target).expect("the search reached it");
        let mut machine = DialStateMachine::new(Face);
        assert_eq!(Snapshot::of(&machine), path.start, "the path starts at new");
        for (event, after) in &path.steps {
            machine
                .process_event(event.clone())
                .expect("the machine takes each event on the path");
            assert_eq!(Snapshot::of(&machine), *after, "after {event:?}");
        }
        assert_eq!(
            Snapshot::of(&machine),
            *target,
            "the path ends at its state"
        );
    }
}
