---
title: smlang
description: Check every state an smlang state machine can reach from a Rust test, and replay a shortest path to any state on the real machine.
---

The `earnest_smlang` crate puts Earnest's exhaustive search under a state machine written with [smlang](https://crates.io/crates/smlang) 0.8. The `statemachine!` block stays as written, apart from its derives. A test hands over the machine and the events to try, and `explore` gives back:

- the count of states the machine can reach,
- `Pass`, `Fail` or `Unfinished`, as `earnest verify` reports them,
- each rule's verdict, with the shortest path of events that breaks it,
- the first stuck state, with its path,
- a shortest path to every state reached, to replay on the real machine.

smlang's own tests run the events a person wrote, in the order written. The search tries every listed event in every reachable state, so it also finds the orders nobody wrote, and its state count shows how much it covered.

## Add it

The crates are not on crates.io yet. Add `earnest_smlang` from git, and `smlang`, which the test uses to write the machine:

```toml title="Cargo.toml"
[dev-dependencies]
earnest_smlang = { git = "https://github.com/cpeaustriajc/earnest" }
smlang = "0.8"
```

`earnest_smlang` re-exports what a test needs from the `earnest` crate (`Rule`, `Exploration`, `Path`, `Verdict`, `Status`, `RuleResult`, `DEFAULT_MAX_STATES`), so the test needs no `earnest` line of its own.

## Example: a turnstile

This turnstile takes a coin to unlock and a push to let one person through. Its cash box holds two coins, and an unlocked gate takes a second coin without giving a second pass. This is the code of the test in `crates/earnest_smlang/tests/turnstile.rs`:

```rust title="tests/turnstile.rs"
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
```

The test adds `derive_states` and `derive_events` to the `statemachine!` block, and derives `Clone`, `Debug`, `PartialEq`, `Eq` and `Hash` on its context, `Gate`. `machine!(StateMachine<Gate>)` connects the generated types to the search. `explore` takes the machine to start from, the events to try (`EVENTS`), the rules, a test that marks an end, and a state limit. Here no state is an end (`|_| false`), and the limit is `DEFAULT_MAX_STATES`, 5,000,000, the default of `earnest verify`.

A searched state is a `Snapshot`: the machine's `state` and its `context`. From each snapshot the search builds a machine in that state with that context, sends it each listed event in order, and takes an event that gives `Ok` from `process_event` as a move to the new snapshot.

### The seven states

Write a state as the gate, the coins in the box and the passes given. Starting from `Locked` with 0 coins and 0 passes, the search reaches seven states:

| State | `Coin` | `Push` |
|---|---|---|
| `Locked`, 0 coins, 0 passes | `Unlocked`, 1, 0 | stays |
| `Unlocked`, 1 coin, 0 passes | `Unlocked`, 2, 0 | `Locked`, 1, 1 |
| `Unlocked`, 2 coins, 0 passes | refused: the box is full | `Locked`, 2, 1 |
| `Locked`, 1 coin, 1 pass | `Unlocked`, 2, 1 | stays |
| `Unlocked`, 2 coins, 1 pass | refused: the box is full | `Locked`, 2, 2 |
| `Locked`, 2 coins, 1 pass | refused: the box is full | stays |
| `Locked`, 2 coins, 2 passes | refused: the box is full | stays |

The test `seven_states_with_up_to_two_coins_and_no_more_passes_than_coins` lists these seven and asserts `found.states == 7`. Every state takes `Push`, so no state is stuck.

### The broken rule

`one_pass_per_coin` says a locked gate has let through one person for each coin. It breaks in the sixth state: `Coin`, `Coin`, `Push` leaves the gate locked with 2 coins and 1 pass. The test reads that from the `Exploration`:

```rust
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
```

`found.rules` holds one result for each rule, in the order given. A `Verdict` is `Holds`, `Broken` with the shortest `Path`, or `Unknown` when the state limit stopped the search before it saw a break. Each step of a path is an event and the `Snapshot` after it, and `path.start` is the snapshot the path leaves from. The other fields of `found`:

| Field | Meaning |
|---|---|
| `status` | `Pass`, `Fail` or `Unfinished` |
| `states` | The count of distinct states reached, start state included |
| `rules` | Each rule's `id`, `says` and `verdict` |
| `stuck` | `None`, or the shortest path to the first stuck state found, breadth first |
| `reason` | `None`, or why an unfinished search stopped, such as `state limit of 1` |
| `reached()` | Every state reached, once each, in the order the search reached them |
| `path_to(&snapshot)` | A shortest path to a state reached, or `None` |

## Replay a path on the real machine

A path is data. To check that the real machine takes it, build a fresh machine, send it each event of the path, and compare `Snapshot::of(&machine)` with the snapshot after each step. This test does that for every state the search reached:

```rust
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
```

A failure here means the search and the machine disagree about a transition. The same loop replays the path of a broken rule: take `path` from the `Verdict::Broken` above instead of `found.path_to(target)`.

## Stuck states and ends

A snapshot where every listed event is refused is stuck, unless the end test is `true` there. A stuck state fails the search, and `found.stuck` holds the path to the first one. An event the test does not list cannot rescue a state. `crates/earnest_smlang/tests/stuck.rs` covers three cases, with a door that has `Closed + Open [has_key] = Opened` and `Opened + Close = Closed`:

- With only `Events::Open` listed, the open door takes no event. The search finds 2 states and one stuck state, after `Open`.
- With `has_key` false and `Open` and `Close` both listed, the closed door refuses both. The start state is stuck, and its path has no steps (`stuck.steps.is_empty()`).
- With `|now| matches!(now.state, States::Opened)` as the end test, the open door is an end. The search passes with the same 2 states and no stuck state.

The turnstile has no stuck state, because `Locked + Push = Locked` takes a push in every locked state, and `Unlocked` takes a push too.

## The state limit

The last argument of `explore` stops the search before it holds more than that many distinct snapshots. Pass `DEFAULT_MAX_STATES` for the limit of `earnest verify`. When the limit stops the search before it finds anything wrong, the status is `Unfinished`, `reason` is `state limit of N`, and a rule not seen broken is `Unknown`. Every counted state has its rules judged, so a rule can still be `Broken` in a search that stopped early, and the status is then `Fail`.

A context field that no guard bounds, such as a counter that only goes up, makes the state space grow without end, so the search runs to the limit. Bound such a field with a guard, as `box_has_room` bounds the coins.

## Limits

- **Derives.** The `statemachine!` block needs `derive_states: [Clone, Debug, PartialOrd, Ord, Eq]` and `derive_events: [Clone, Debug]`. The search clones states and events to rebuild machines, so both need `Clone`. smlang writes `PartialEq` for `States` and `Events` by hand and compares only the variant, so `Showing(1) == Showing(2)` there. The search compares states with `Ord`, which compares the data too, and keeps them apart. Clippy refuses a derived `Hash` beside smlang's `PartialEq`, so `States` has none, and a snapshot hashes the state's `Debug` text. The context needs `Clone`, `Eq` and `Hash`, and `Debug` to print a snapshot in a failed assertion.
- **`Debug` and `Ord` must agree.** Two states that `Ord` calls equal must print the same. They do for integers, strings, `bool` and derived types, and not for a field whose `Ord` ignores what its `Debug` prints, such as a case-insensitive string. A hand-written `Ord` that ignores the data merges states that differ.
- **Every `Err` means the event can't happen.** An event the state has no transition for, a guard that is `false` or returns `Err`, and an action that returns `Err` look the same to the search. It drops the machine after a failed action, so what the action changed in the context is lost.
- **Events compare by variant only.** `[Events::Set(1)] == [Events::Set(2)]` is true, so check the events of a path with `matches!`. List an event that carries data once for each value to try: `Events::Set(1), Events::Set(2)`.
- **No async guards or actions, no temporary context, no states or events that hold references.** `machine!` does not support them.
- **No range check.** `explore` does not detect a field leaving a range, and smlang declares none. `earnest verify` detects it for an Earnest model.
- **A panic is not caught.** A panic in a guard, an action or a rule ends the test.
- **A named machine needs its types named.** A machine with `name: Door` generates `DoorStateMachine`, `DoorStates` and `DoorEvents`, so write `machine!(DoorStateMachine<Context>, DoorStates, DoorEvents)`. A path works for each name, as in `machine!(door::StateMachine<Context>, door::States, door::Events)`. `crates/earnest_smlang/tests/data.rs` covers a named machine whose states and events carry data.

## Next

- [Rust models](/reference/rust/#searching-states-in-a-test): `explore` and `search`, the same search for an Earnest model and for any state
- [Output, JSON, and reports](/reference/output/): what `pass`, `fail`, `unfinished` and `stuck` mean for `earnest verify`
- [XState](/integrations/xstate/): the same search under a TypeScript state machine library
