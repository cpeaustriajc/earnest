---
title: TLA+ export and confirmation
description: Export models to TLA+ modules and confirm earnest's state counts and verdicts against TLC or tla-rs.
---

Every earnest model can be exported as TLA+ so an independent TLA+ checker can confirm the distinct state count and every rule verdict.

## Export TLA+

```sh
earnest tla src/cart.model.ts --out build/tla
```

`earnest tla` writes `<Model>.tla` and `<Model>.cfg` into `--out` for each matched model.

## Confirm with TLC or tla-rs

`earnest confirm` exports every matched model, runs an external TLA+ checker on each one, and exits `1` if any state count or rule verdict disagrees with `earnest`.

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

## How models map to TLA+

### Rules and faults

Each rule `id` is exported as an invariant `Rule_id` listed in `<Model>.cfg`:

- If a rule cannot be evaluated in a state (a fault), `Rule_id` evaluates to `TRUE` in that state, and actions that fault take no step, so TLC counts every reachable state without aborting.
- For any rule that can fault, the module also defines `Judged_id`, which fails in states where the rule has no value.
- `earnest confirm` checks both invariants to derive TLC's verdict: `broken` if `Rule_id` fails, else `unknown` if `Judged_id` fails, else `holds`.
- TLC cannot observe an out-of-range state or an action/end fault that truncated exploration. When `earnest` leaves a rule `unknown` solely for that reason and `Judged_id` holds in TLC, `holds` in TLC still counts as agreement.

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
