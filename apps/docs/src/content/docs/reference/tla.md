---
title: TLA+ export and confirmation
description: Export models to TLA+ modules and confirm earnest's state counts and verdicts against TLC or tla-rs.
---

Every earnest model can be exported as TLA+ so an independent TLA+ checker can confirm the distinct state count and every rule verdict.

## Export TLA+

```sh
earnest tla src/cart.model.ts --out build/tla
```

`earnest tla` writes `<Model>.tla` and `<Model>.cfg` into `--out` for each matched model. A model with setups ([TypeScript](/reference/typescript/#setups), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)) also gets `<Model>_<setup>.tla` and `<Model>_<setup>.cfg` for each setup: a module of its own, named `<Model>_<setup>`, that is the model's module with the setup's values in `Init`.

## Confirm with TLC or tla-rs

`earnest confirm` exports every matched model, and each setup of a model, runs an external TLA+ checker on each one, and exits `1` if any state count or rule verdict disagrees with `earnest`. Each setup prints its own line, named like `Clock (behind)`.

```sh
earnest confirm --tlc tla2tools.jar
earnest confirm --tla-rs ~/.cargo/bin/tla-rs
earnest confirm --tlc tla2tools.jar --tla-rs ~/.cargo/bin/tla-rs
```

- **`--tlc <jar>`** runs TLC with Java.
- **`--tla-rs <path>`** runs [tla-rs](https://github.com/bbcvc/tla-rs) (crate `tla-tools`, installed via `cargo install tla-tools`), a Rust TLA+ checker that needs no Java.
- Pass both flags to run both checkers on every model. Each model gets one output line per checker, with the `tla-rs` line ending in `(tla-rs)`.
- `--jobs <n>` sets the worker count passed to each checker (`--workers`).
- Because `tla-rs` stops at its first violated invariant, each rule invariant runs in its own invocation. Differences report `tla-rs found` and `tla-rs says`.
- `--tlc` also runs TLC once for each [rule about runs](/reference/typescript/#rules-about-runs), with that rule's property alone, and compares its verdict: `Temporal properties were violated.` is `broken`, a run with no error is `holds`, and anything else is `unknown`. `tla-rs` is not run on rules about runs, so `--tla-rs` compares only the state count and the plain rules.

## How models map to TLA+

### Rules and faults

Each rule `id` is exported as an invariant `Rule_id` listed in `<Model>.cfg`:

- A [witness](/reference/typescript/#witnesses) is not exported, and `earnest confirm` leaves it out of the comparison.
- If a rule cannot be evaluated in a state (a fault), `Rule_id` evaluates to `TRUE` in that state, and actions that fault take no step, so TLC counts every reachable state without aborting.
- For any rule that can fault, the module also defines `Judged_id`, which fails in states where the rule has no value.
- `earnest confirm` checks both invariants to derive TLC's verdict: `broken` if `Rule_id` fails, else `unknown` if `Judged_id` fails, else `holds`.
- TLC cannot observe an out-of-range state or an action/end fault that truncated exploration. When `earnest` leaves a rule `unknown` solely for that reason and `Judged_id` holds in TLC, `holds` in TLC still counts as agreement.

### Rules about runs and fairness

A model with a rule about runs ([TypeScript](/reference/typescript/#rules-about-runs), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)) also gets a specification and one temporal property per rule. For a model whose action `sync` is fair:

```text
Spec ==
    Init /\ [][Next]_vars /\ WF_vars(Next) /\ WF_vars(Action_sync)

Eventually_syncs ==
    <>(synced)

Eventually_offered ==
    (approved) ~> (offers)
```

- **`Spec`:** `Init`, then `[][Next]_vars`, then `WF_vars(Next)`, which says a run never stops while some action can change the state, then `WF_vars(Action_id)` for each fair action (`fair: true` in TypeScript, `#[action("...", fair)]` in Rust), in declaration order. An action with inputs is one `Action_id` that picks its input with `\E`, so its fairness is fairness of the action as a whole. A step that keeps every variable as it is is stuttering, which `[Next]_vars` allows and weak fairness ignores, as earnest does.
- **Fairness of one argument:** a Quint rule fair to an action given an argument, or over some of the variables, gets a definition `Fair_id_k`, the action with each given input picked from a set of that one value, and its fairness is `WF_<<x, y>>(Fair_id_k)` with the variables the Quint file named, in its order.
- **Properties:** each rule `id` is `Eventually_id`: `<>(reaches)` without `after`, and `(after) ~> (reaches)` with it. A condition that reads no variable is written `vars = vars /\ condition`, because TLC misjudges a temporal property over a constant.
- **Configuration:** `<Model>.cfg` names `SPECIFICATION Spec` in place of `INIT Init` and `NEXT Next`, keeps `CHECK_DEADLOCK FALSE`, lists the invariants, and then one `PROPERTY Eventually_id` line per rule. `earnest confirm` checks the invariants from `INIT Init` and `NEXT Next` as before, and each property alone with `SPECIFICATION Spec`.
- A model without a rule about runs is exported exactly as before, whether or not it has fair actions.

### Quantifier order and short-circuiting

Quantifiers stop at the first item that decides the result in `earnest`, in Node (`every`/`some`), and in Rust (`forall`/`exists`), so a fault on a later item never triggers.

The TLA+ export preserves that exact evaluation order:

- Type quantifiers (`m.forall(T, f)`, `m.exists(T, f)`, `forall::<T>(f)`, `exists::<T>(f)`) walk values in declaration order (`m.oneOf([3, 1, 2])` tries `3`, then `1`, then `2`).
- Sequences and queues walk items in index order.
- Sets walk items in canonical set order (see [Primitives: Set and map ordering](/reference/primitives/#order-of-sets-maps-and-records)).
- For numbers and booleans, the module uses TLC's native order. For text, the module defines `TextRank` (code-point rank across every text in the model). For compound values, the module generates an item-by-item order predicate.

A quantifier body is evaluated only when no earlier item has already decided the quantifier, regardless of TLC's internal iteration order.

## TLC limits

TLC imposes two constraints that the TLA+ export cannot lift:

1. **32-bit integers:** TLC only handles integers from `-2147483648` to `2147483647`.
2. **ASCII text:** TLC treats some non-ASCII strings as equal during state deduplication (for example, `Z` and `Ś` collapse into one state).

When a model declares an integer outside 32 bits, contains non-ASCII text, or computes an integer outside 32 bits during search:

- `earnest tla` prints a warning on stderr for declared values (and notes that runtime overflows are detected only by `earnest confirm`).
- `earnest confirm` prints `cannot` instead of `differs`, prints `TLC can't confirm this model:` with the reason before any differences, and exits `1`. If TLC still matches, the model prints `same`.

These limits apply only to TLC. A model that `tla-rs` cannot confirm prints `differs`, never `cannot`.

## Running the repository confirmation tests

The repository includes optional test suites behind Cargo features:

```sh
cargo test --locked -p earnest_break_it --features tlc --test tlc
cargo test --locked -p earnest_break_it --features tla_rs --test tla_rs
```
