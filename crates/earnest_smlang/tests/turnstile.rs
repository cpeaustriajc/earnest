//! A coin-operated turnstile written with smlang's `statemachine!`, searched by
//! `earnest_smlang`. Its cash box holds two coins, and an unlocked gate takes a second coin it
//! gives no second pass for.

use earnest_smlang::{
    DEFAULT_MAX_STATES, Exploration, Rule, Snapshot, Status, Verdict, explore, machine,
};

mod turnstile {
    use smlang::statemachine;

    statemachine! {
        derive_states: [Clone, Debug, PartialOrd, Ord, Eq],
        derive_events: [Clone, Debug],
        transitions: {
            *Locked + Coin [box_has_room] / take_coin = Unlocked,
            Unlocked + Coin [box_has_room] / take_coin = Unlocked,
            Unlocked + Push / let_through = Locked,
            Locked + Push = Locked,
        }
    }
}

use turnstile::{Events, StateMachine, StateMachineContext, States};

const BOX_SIZE: u8 = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct Gate {
    coins: u8,
    passes: u8,
}

impl StateMachineContext for Gate {
    fn box_has_room(&self) -> Result<bool, ()> {
        Ok(self.coins < BOX_SIZE)
    }

    fn take_coin(&mut self) -> Result<(), ()> {
        self.coins += 1;
        Ok(())
    }

    fn let_through(&mut self) -> Result<(), ()> {
        self.passes += 1;
        Ok(())
    }
}

machine!(StateMachine<Gate>);

const EVENTS: [Events; 2] = [Events::Coin, Events::Push];

fn one_pass_per_coin(now: &Snapshot<StateMachine<Gate>>) -> bool {
    matches!(now.state, States::Unlocked) || now.context.coins == now.context.passes
}

fn check() -> Exploration<Snapshot<StateMachine<Gate>>, Events> {
    explore(
        &StateMachine::new(Gate::default()),
        &EVENTS,
        &[Rule::new(
            "one_pass_per_coin",
            "A locked gate has let through one person for each coin",
            one_pass_per_coin,
        )],
        |_| false,
        DEFAULT_MAX_STATES,
    )
}

#[test]
fn seven_states_with_up_to_two_coins_and_no_more_passes_than_coins() {
    let found = check();
    let mut reached: Vec<(States, u8, u8)> = found
        .reached()
        .map(|now| (now.state.clone(), now.context.coins, now.context.passes))
        .collect();
    reached.sort();
    assert_eq!(
        reached,
        [
            (States::Locked, 0, 0),
            (States::Locked, 1, 1),
            (States::Locked, 2, 1),
            (States::Locked, 2, 2),
            (States::Unlocked, 1, 0),
            (States::Unlocked, 2, 0),
            (States::Unlocked, 2, 1),
        ],
        "three locked with every coin passed, two unlocked with one coin waiting, \
         two after a coin that bought no pass"
    );
    assert_eq!(found.states, 7, "each state above once");
}

#[test]
fn a_second_coin_on_an_unlocked_gate_buys_no_second_pass() {
    let found = check();
    assert_eq!(found.status, Status::Fail, "the rule is broken");
    let Verdict::Broken(path) = &found.rules[0].verdict else {
        panic!("coin, coin, push locks the gate with two coins and one pass");
    };
    let events: Vec<&Events> = path.steps.iter().map(|(event, _)| event).collect();
    assert!(
        matches!(events[..], [Events::Coin, Events::Coin, Events::Push]),
        "the shortest path is three events: {events:?}"
    );
    let broken = &path.steps[2].1;
    assert!(
        matches!(broken.state, States::Locked),
        "the gate locks again"
    );
    assert_eq!(
        broken.context,
        Gate {
            coins: 2,
            passes: 1
        },
        "two coins bought one pass"
    );
}

#[test]
fn a_full_box_refuses_coins_but_the_gate_still_takes_a_push() {
    let found = check();
    assert!(
        found.reached().all(|now| now.context.coins <= BOX_SIZE),
        "box_has_room refuses a third coin"
    );
    assert!(
        found.stuck.is_none(),
        "pushing a locked gate leaves it locked, so no state refuses every event"
    );
}

#[test]
fn every_path_replays_on_a_fresh_machine() {
    let found = check();
    for target in found.reached() {
        let path = found.path_to(target).expect("the search reached it");
        let mut machine = StateMachine::new(Gate::default());
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
