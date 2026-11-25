# earnest_smlang

Searches every state an [smlang](https://crates.io/crates/smlang) 0.8 state machine can reach,
from a Rust test, with the search of the `earnest` crate. The `statemachine!` block stays as
written, apart from its derives.

A test adds `derive_states: [Clone, Debug, PartialOrd, Ord, Eq]` and `derive_events: [Clone,
Debug]`, derives `Clone`, `Debug`, `PartialEq`, `Eq` and `Hash` on its context, connects the
generated types with `machine!`, and hands `explore` a machine and the events to try:

```rust
machine!(StateMachine<Gate>);

let found = explore(
    &StateMachine::new(Gate::default()),
    &[Events::Coin, Events::Push],
    &[Rule::new(
        "one_pass_per_coin",
        "A locked gate has let through one person for each coin",
        one_pass_per_coin,
    )],
    |_| false,
    DEFAULT_MAX_STATES,
);
```

A searched state is a `Snapshot`: the machine's `state` and its `context`. From each snapshot the
search builds a machine with `new_with_state`, and takes an event that gives `Ok` from
`process_event` as a move to the machine's new state and context. An `Err` (an event the state has
no transition for, a guard that is false or fails, a failed action) means the event can't happen
there. A snapshot where every listed event is refused is stuck unless the end test is true there.
List an event that carries data once for each value to try: `Events::Set(1), Events::Set(2)`.

`explore` gives an `Exploration` from `earnest::model`: its `status`, the count of `states`, each
rule's `Verdict` with the shortest path of events, the first `stuck` state, and
`exploration.path_to(&snapshot)` for a shortest path to any state reached. A test replays a path on
a fresh machine with `process_event` and compares `Snapshot::of(&machine)` with each step.

smlang writes `PartialEq` for `States` and `Events` by hand and compares only the variant, so
`Showing(1) == Showing(2)` there. The search compares states with `Ord` from `derive_states`, which
compares the data too, and keeps them apart. Clippy refuses a derived `Hash` beside smlang's
`PartialEq`, so `States` derives none: a snapshot hashes the state's `Debug` text, which must print
the same for two states `Ord` calls equal, as it does for integers, strings and derived types.
Check the events of a path with `matches!`, since `==` on `Events` ignores their data.

| Item | What it is |
|---|---|
| `machine!(StateMachine<Context>)` | Implements `Machine` for a generated machine, with the `States` and `Events` in scope. For a machine with `name: Door`, `machine!(DoorStateMachine<Context>, DoorStates, DoorEvents)` |
| `Machine` | The trait `machine!` implements: `resume`, `state`, `context` and `take`, the parts of `StateMachine` the search uses |
| `explore(&machine, &events, &rules, end, max_states)` | Searches every state reachable by the listed events, breadth first, trying them in the order listed |
| `Snapshot` | One searched state, `state` and `context`. `Snapshot::of(&machine)` reads one from a real machine |
| `Rule`, `Exploration`, `Path`, `Verdict`, `Status`, `RuleResult`, `DEFAULT_MAX_STATES` | Re-exported from `earnest::model`, as its [README](../earnest/README.md) describes them |

Async guards and actions, a temporary context, and states or events that hold references are not
supported.

The turnstile in `tests/turnstile.rs` is the worked example: 7 states, a rule broken after `Coin`,
`Coin`, `Push`, and every path replayed on a fresh machine. `tests/stuck.rs` covers stuck states,
failed guards and actions, and ends, and `tests/data.rs` states and events that carry data.
