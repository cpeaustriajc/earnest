---
title: Rust models
description: Write Earnest models as Rust structs, state what whole runs do, handle Option and match, follow move and type rules, replay paths in tests, and keep models lint-clean.
---

A Rust model is a struct that derives `Model`, paired with an `impl` block marked `#[model]`. `earnest verify` reads the source tokens directly without running `cargo build`, while the `earnest` crate lets your tests compile and step through the exact same model.

## Example

```rust title="src/turnstile.rs"
use earnest::model::{Domain, Model, model, require};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Domain)]
pub enum Gate {
    Locked,
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
pub struct Turnstile {
    #[initial(Gate::Locked)]
    gate: Gate,
    #[initial(false)]
    paid: bool,
    #[range(0, 3)]
    #[initial(0)]
    visitors: u8,
}

#[model]
impl Turnstile {
    #[action("the visitor pays")]
    fn pay(&self) -> Option<Self> {
        require!(!self.paid);
        Some(Self { paid: true, ..self.clone() })
    }

    #[action("the visitor pushes")]
    fn push(&self) -> Option<Self> {
        require!(self.paid && self.visitors < 3);
        Some(Self { gate: Gate::Open, visitors: self.visitors + 1, ..self.clone() })
    }

    #[rule("The gate stays locked until the visitor pays")]
    fn stays_locked(&self) -> bool {
        self.gate == Gate::Locked || self.paid
    }

    #[end("Three visitors went through")]
    fn full(&self) -> bool {
        self.visitors == 3
    }
}
```

## Structs, fields, and `#[one_of]`

A field's domain comes from its Rust type and attributes:

- `bool`, integer types with `#[range(lo, hi)]` or `#[one_of(...)]`, `&'static str` with `#[one_of(...)]`, enums or structs deriving `Domain`, `Option<T>`, and the library types `Set<T>`, `Map<K, V>`, `Seq<T, MAX>`, `Queue<T, CAPACITY>`, `Lock`, `Lock<Owner>`, `Timer`, `Request`, and `Retry<MAX>`.
- A model struct does not need `pub` or doc comments: `#[derive(Model)]` references the type so `rustc` emits no dead-code warnings, and the file passes `clippy`.
- `#[one_of(a, b)]` is the Rust form of `m.oneOf([a, b])`. Use it on a field, an action parameter, or a field of a `Domain` struct:

```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Pick {
    #[one_of(1, 5, 9)]
    size: u8,
    #[one_of("Kim", "Lee")]
    #[initial(None)]
    buyer: Option<&'static str>,
}

#[model]
impl Pick {
    #[action("a buyer takes the pick")]
    fn sell(&self, #[one_of("Kim", "Lee")] who: &'static str) -> Option<Self> {
        require!(self.buyer.is_none());
        Some(Self { buyer: Some(who), ..self.clone() })
    }
}
```

Rules for `#[one_of(...)]`:

- Works on any integer type (instead of `#[range]`) or `&'static str` (including inside `Option`).
- Listed numbers must fit in the integer type, and arithmetic keeps that type's bounds.
- A list must be non-empty, contain unique values of one kind (no mixing numbers and text), and cannot be combined with `#[range]`.
- Without `#[initial]`, the field starts at every listed value in the order written; action parameters are tried in the order written.
- Exports to TLA+ identically to `m.oneOf` (`size \in {1, 5, 9}`, `who \in {"Kim", "Lee"}`).

### Starting at specific values

`#[initial(value)]` starts a field at one value. `#[initial(A, B)]` starts it at each listed value in order, creating one start state per value (list each value once):

```rust
#[initial(Stage::Cold, Stage::Warm)]
stage: Stage,
```

An enum field without `#[initial]` starts at every variant. `#[one_of]` is for integers and `&'static str`, not enums.

## Actions, rules, ends, and naming

- **Actions:** Methods inside `#[model]` taking `&self` and optional inputs, returning `Option<Self>` or `Self`.
  - Use `Option<Self>` when the action has guards (`require!` or `?`). Every path must return `Some(Self { changed_fields, ..self.clone() })`; bare `Self { .. }`, `Some(Some(..))`, or unwrapped values are refused (`E0308`).
  - Use `Self` when nothing can stop the action (avoids `clippy::unnecessary_wraps`). A `Self`-returning action cannot use `require!`, `?`, `None`, or `Some(..)` (`E0308`, `E0277`).
  - `require!(condition);` is allowed as a statement in the body or inside an early `if` block, and refused inside `match` arms or other expressions.
  - Before the final expression, `let` bindings and early `if condition { return Some(Self { .. }); }` (or `return Self { .. };`) are allowed. A `require!` after an early `return` applies only when that early condition is false.
- **Rules and ends:** Methods returning `bool`. Their ids are their method names. Enum values print by variant name (for example, `"Locked"`).
- **Language-native ids:** A Rust model and a TypeScript twin use each language's identifier spelling (`no_edit_after_approval` vs `noEditAfterApproval`). Pass `--rule` using the id as spelled in the file.
- **Raw identifiers:** Treated as their plain names everywhere (`fn r#fn` has rule id `fn`, `fn r#loop` has action id `loop`, field `r#type` is `type` in output, `--rule`, JSON, and TLA+).
- **Reserved names and imports:**
  - Do not name any function, constant, struct, enum, or local value `forall`, `exists`, `Some`, `None`, `Set`, `Seq`, or `Map`, or shadow a called helper function.
  - Import library items under their own names (`as` renames are refused except `as _`).
  - Re-exporting the library (`pub use earnest::...`, `pub(crate) use ...`, `r#earnest`) is refused.
  - `use` inside a function body, `macro_rules!`, `type` aliases, and path-qualified `#[action]`/`#[rule]`/`#[end]`/`#[eventually]`/`#[witness]`/`#[setup]` attributes are refused.
  - Duplicate items in a namespace, duplicate methods, fields, variants, or parameters are refused (matching `rustc`). `let` may shadow earlier locals.
  - No part of a model may sit under `#[cfg]` or `#[cfg_attr]`. Only non-model items (such as a `#[cfg(test)]` module, helper test function, or plain `impl`) may carry `#[cfg(test)]`, and they are skipped.

## Rules about runs, witnesses, and setups

Four more forms say what whole runs of the model do. They are the Rust forms of `fair: true`, `m.eventually`, `m.witness` with `m.step`, and `m.setup`, and `earnest verify` reports them the same way, with the same verdicts, paths, reasons, and JSON (`kind`, `reason`, `repeatsFrom`, `setup`).

```rust title="src/gate.rs"
#[model]
impl Gate {
    #[action("the visitor pays", fair)]
    fn pay(&self) -> Option<Self> {
        require!(!self.paid && self.door == Door::Locked);
        Some(Self { paid: true, ..self.clone() })
    }

    #[action("the visitor pushes")]
    fn push(&self) -> Option<Self> {
        require!(self.paid && self.door == Door::Locked);
        Some(Self { door: Door::Open, ..self.clone() })
    }

    #[end("The gate is open")]
    fn open(&self) -> bool {
        self.door == Door::Open
    }

    fn paid_up(&self) -> bool {
        self.paid
    }

    #[eventually("Once the visitor pays, the gate opens", after = paid_up)]
    fn opens(&self) -> bool {
        self.door == Door::Open
    }

    #[witness("Pay, then push: the gate opens")]
    fn pay_then_push(&self) -> Option<bool> {
        let after = self.pay()?.push()?;
        Some(after.door == Door::Open)
    }

    #[setup("The gate starts paid")]
    fn prepaid(&self) -> Self {
        Self { paid: true, ..self.clone() }
    }

    #[witness("A paid gate opens on a push", setup = prepaid)]
    fn push_when_paid(&self) -> Option<bool> {
        let after = self.push()?;
        Some(after.door == Door::Open)
    }
}
```

| Form | Meaning |
|---|---|
| `#[action("...", fair)]` | A run never puts the action off forever while it stays possible |
| `#[eventually("...")]` on `fn(&self) -> bool` | Every run from every start state comes to a state where the method is true |
| `#[eventually("...", after = name)]` | The same, from every reachable state where the helper method `name` is true |
| `#[witness("...")]` on `fn(&self) -> Option<bool>` | From every start state, each step can happen in turn and the condition is true after the last |
| `#[witness("...", setup = name)]` | The same, played from the start states of the setup `name` |
| `#[setup("...")]` on `fn(&self) -> Self` | The model is checked once more with the fields written starting there, as `Model (name)` |

Ids are the method names. A run never stops while an action can still change the state.

- **`after`:** names a helper method of the same block that takes only `&self` and returns `bool`. A name that is no method (`E0599`), an action, a rule, an end, another rule about runs, a witness, a setup, or a helper with parameters or another result is refused.
- **Witness body:** either `Some(condition)` on `self`, or `let name = chain;` followed by `Some(condition)` on `name`. The chain starts at `self` and calls the model's actions in order: write `?` after an action that returns `Option<Self>` and nothing after one that returns `Self`. An action with inputs takes constant values in parameter order: `self.drift(1)?`. Anything else is refused, including a missing `?` (`E0599`), a `?` after an action that returns `Self` (`E0277`), a wrong number of inputs (`E0061`), and a condition that is not a `bool` (`E0308`).
- **Witness result:** called on a start state (of its setup, when it names one), the method gives `Some(true)` exactly where `earnest verify` says the witness holds, and `None` where a step can't happen. The one difference: a step that takes a field outside its declared values breaks the witness for the checker, where the method can still give `Some(true)`.
- **Setup body:** exactly `Self { field: value, ..self.clone() }` with at least one field, where every value is a constant. A value that reads `self`, a missing `..self.clone()`, or a statement before the struct is refused. A value outside the field's declared values is refused when the model is checked.
- **Options:** each attribute takes only its own option, once. `#[rule("...", fair)]` and `#[witness("...", after = name)]` are refused by the reader and by `#[model]`.

`#[model]` removes the attributes and leaves each method callable as written. It also implements `state.eventually()`, `state.witnesses()`, and `state.setups()` from `earnest::model::Behavior`, which give `Eventually`, `Witnessed`, and `Setup` values, so a test can read every method's result in one state. Those calls make `rustc` refuse an `after` or a `setup` that names no method, and keep the methods from being dead code. A witness method passes `clippy::unnecessary_wraps` even when no step can fail.

## Statechart views (`#[view]`)

Add a second `impl` block with `#[view(...)]` to control Mermaid statechart labels (the Rust counterpart to `m.view`):

```rust
#[view(
    title = "Add to cart focus",
    step(press = "press Add"),
    colors(broken = "#c62828")
)]
impl Cart {
    #[state]
    fn state(&self) -> String {
        format!("{:?}, focus on {:?}", self.cta, self.focus)
    }

    #[group]
    fn group(&self) -> String {
        self.surface.to_owned()
    }
}
```

- Every part is optional, and a model may have at most one `#[view]` block.
- `title`, `step(action = "label")`, and `colors(part = "color")` follow the same validation rules as TypeScript (`broken`, `start`, `end` only; valid hex or alphabetic color; `step` actions must exist on the model).
- `#[state]` labels each box (states with the same label and group merge into one box); `#[group]` names the subgraph box. Each takes `&self`, returns `String`, and has a single-expression body: `format!(...)` or `<value>.to_string()` / `<value>.to_owned()`.
- A view method may call helpers from the `#[model]` block (`self.name()`), but cannot call actions, rules, or ends. `format!` is allowed only inside `#[view]`.
- `#[view]` implements `earnest::model::View`, exposing `state.state_label()`, `state.group_label()`, `Cart::title()`, `Cart::step_labels()`, and `Cart::colors()`.

Format placeholders in `#[view]` are checked against both `rustc` and chart rendering:

| Value type | `{}` | `{:?}` | `.to_string()` | `.to_owned()` |
|---|---|---|---|---|
| `bool` | allowed | allowed | allowed | refused (`E0308`) |
| Integer | allowed | allowed | allowed | refused (`E0308`) |
| Enum variant | refused (`E0277`, no `Display`) | allowed (prints variant name) | refused (`E0599`) | refused (`E0308`) |
| `&'static str` | allowed | refused (`Debug` adds quotes/escapes) | allowed | allowed |
| `Option`, record, collection | refused | refused | refused | refused |

Format specifiers with widths, names, positions, or flags are refused. `{{` and `}}` escape braces; unmatched braces or mismatched argument counts are refused.

## Rust semantics agreement

Where reading a Rust file statically could diverge from compiling and running it with `rustc`, `earnest` either matches `rustc` or refuses the file.

### Integer types and overflow

- Every integer keeps its Rust type (`u8`, `i32`, `usize`, etc.). Any arithmetic result outside that type's range is a `fault` in that state (for example, `self.n - 1` when `n: u8` is `0`, `self.items.size() - 1` on an empty list, or signed/unsigned overflow).
- Types propagate from fields, inputs, constants, helper signatures, `let` annotations, or numeric suffixes (`1_u8`). Unconstrained integer literals default to `i32`; a `let` with an otherwise untyped integer must state its type (`let most: u8 = 3;`). Different integer types cannot mix.
- `#[range(lo, hi)]` must lie inside its integer type. `u64`, `usize`, and 128-bit integers are exact up to `i64::MAX` (`"computes a number too large to hold"` past that). Negated literals such as `-128` (`i8`) and `-9223372036854775808` (`i64`) are read as single literals everywhere.
- `%` and `/` follow Rust semantics (fault on negative inputs or zero divisor).
- `n.is_multiple_of(d)` is supported on unsigned integer types (`n % d == 0`, with `n.is_multiple_of(0)` equal to `n == 0`), matching `clippy::manual_is_multiple_of`. It is refused on signed types (`E0599`), untyped literals (`E0689`), and `Option`.

### `let` scoping and `Option<Option<T>>`

- `let` follows Rust scoping: the right-hand side of `let n = n + 1;` reads the previous `n`, and uses before a later `let` read the outer binding.
- Because `get`, `first`, `last`, and `head` return `Option`, `Map` values and `Seq`/`Queue` items cannot themselves be `Option` (wrap them in a struct with an `Option` field or omit the entry). `Option<Option<T>>`, `Some(None)`, and `Lock<Option<T>>` are also refused. `Set<Option<T>>` and `Map<Option<K>, V>` are allowed.

### Reading values inside `Option`

```rust
fn bonus(&self) -> u8 {
    if let Some(x) = self.last { x * 2 } else { 1 }
}

fn weight(&self) -> u8 {
    self.last.map_or(0, |x| x * 2)
}

fn count(&self) -> u8 {
    self.last.unwrap_or(7)
}

#[action("the last number is added to the total")]
fn add(&self) -> Option<Self> {
    let x = self.last?;
    require!(self.total + x <= 9);
    Some(Self { total: self.total + x, last: None, ..self.clone() })
}
```

- `if let Some(x) = value { a } else { b }` requires `else` and a plain `Some(name)` pattern (`Some(_)`, `mut`, `ref`, nested patterns, and `&&` guards are refused; use `is_some()` for `Some(_)`).
- `value.map_or(default, |x| ...)` and `value.unwrap_or(default)` evaluate `default` eagerly, as Rust does, so a faulting `default` faults regardless of whether `value` is `Some` or `None`.
- `let x = value?;` (and `let Some(x) = value else { return None; };`) is allowed in actions returning `Option<Self>` and skips the action when `value` is `None`. Any other `else` branch, or `?` inside rules, ends, helpers, or `Self`-returning actions, is refused.
- `unwrap()` and `expect(...)` are refused because panics would abort `steps()` replays.

### Patterns, moves, and nominal types

- **Pattern resolution:** Names in patterns resolve first against in-scope constants and `None`/`Some`/`Ok`/`Err`. A constant in a refutable pattern (`if let Some(TOP) = value`, `let Some(TOP) = value else { return None; }`, or a `match`/`matches!` arm) compares against that constant's value. Using a constant or `Some`/`Ok`/`Err`/`None` in an irrefutable binding position (`let TOP = ...`, parameters, closure parameters) is refused (`E0005`, `E0530`).
- **Move checking:** Non-`Copy` values follow Rust move semantics everywhere (`let`, arguments, primitives, record fields, `?`, `unwrap_or`, `map_or`, `is_some_and`, closures, struct update remainders). Moving out of `&self`, a reference field, or a repeated closure (`E0507`) and use-after-move (`E0382`) are refused with a suggested fix (derive `Copy`, call `.clone()`, or pass by reference).
  - `Copy` types: integers, `bool`, `&'static str`, references, `Timer`, `Request`, `Retry`, `Lock` of a `Copy` owner, enums/structs deriving `Copy`, and `Option`/tuples of `Copy` types.
  - Non-`Copy` types: `Set`, `Seq`, `Queue`, `Map`, and enums/structs without `Copy`.
  - Non-moving reads: `&self` methods, comparisons (`==`, `<`), binding-free `match`, `&x`, and `x.clone()`.
- **Nominal types:** Every enum and struct is a distinct type. Two enums with identical variant names or two structs with identical fields never compare, assign, or match across types (`E0308`, `E0631`, `E0277`).

## `match` and `matches!`

```rust
fn width(&self) -> u8 {
    match self.stage {
        Stage::Cold => 1,
        Stage::Warm | Stage::Hot => 2,
        _ => 3,
    }
}

fn speed(&self) -> u8 {
    match (self.stage, self.mode) {
        (Stage::Cold, _) => 0,
        (Stage::Warm | Stage::Hot, Mode::Fast) => 2,
        (_, Mode::Fast) => 1,
        (_, Mode::Slow) => 3,
    }
}

#[action("the stage moves on")]
fn advance(&self) -> Self {
    match self.stage {
        Stage::Cold => Self { stage: Stage::Warm, ..self.clone() },
        Stage::Warm | Stage::Hot => Self { stage: Stage::Done, ..self.clone() },
        Stage::Done => self.clone(),
    }
}
```

- Supported in rules, ends, helpers, view labels, `let` initializers, and as the final expression of an action.
- Scrutinee must be a `Domain` enum or a 2-tuple `(a, b)` of `Domain` enums.
- Arms may use `Enum::A`, `Enum::A | Enum::B`, `_`, or pairs like `(Enum::A | Enum::B, _)`. First matching arm wins. `matches!(value, Enum::A | Enum::B)` follows the same rules.
- Arms must be exhaustive or end with `_`; missing variants are refused (`E0004`).
- Guards (`if`), bindings, nested/reference patterns, mismatched enum arms (`E0308`), matching on numbers/`Option`/3-tuples, and `require!` inside arms are refused.

## Helpers and integer conversions

Methods inside `#[model]` without `#[action]`, `#[rule]`, `#[end]`, `#[eventually]`, `#[witness]`, or `#[setup]` are helpers (`self.name(..)`). Free functions in the file are also helpers (`name(..)`). Helper parameters do not take `#[range]`.

```rust
fn with_stage(&self, stage: Stage) -> Self {
    Self { stage, ..self.clone() }
}

fn name(&self) -> &'static str {
    if self.flagged { "flagged" } else { "plain" }
}

fn grown(tag: &Tag) -> Tag {
    Tag { size: tag.size + 1, ..tag.clone() }
}

fn weight(&self) -> u8 {
    u8::from(self.flagged) + u8::from(self.tag.kind == Kind::Large)
}

#[action("the tag grows and the count goes up")]
fn grow(&self) -> Option<Self> {
    require!(self.tag.size < 3 && self.count < 3);
    Some(Self { tag: grown(&self.tag), ..self.with_count(self.count + 1) })
}
```

- A helper returning `Self` may be returned from an action (`Some(self.with_stage(..))`) or used as a struct update base (`..self.with_count(..)`). Rules, guards, and other helpers cannot read a `Self`-returning helper.
- Helpers may return `&'static str`, a `Domain` struct, an enum, an integer, `bool`, or `Option`.
- Lossless `From` conversions (`u8::from(flag)`, `u16::from(small)`) work for every integer pair where Rust implements `From` (matching `clippy::bool_to_int_with_if` and `clippy::cast_lossless`). Narrowing conversions and `as` casts are refused.

## Replaying paths in `cargo test`

Because a Rust model compiles as ordinary Rust, unit and integration tests can call actions directly (`state.pay()`) or replay a path from `earnest verify --json` using `earnest::model::replay`:

```rust
let start = Mailbox::starts().remove(0);
let path = [
    Move::new("deliver").with("letter", "Bill"),
    Move::new("deliver").with("letter", "Bill"),
];
let states = replay(&start, &path).expect("the path is one the model can take");
assert!(!states[2].bills());
```

- `Turnstile::starts()` returns every start state. `state.steps()` enumerates every enabled step.
- `Move::new(action).with(input, value)` accepts values formatted as in text or JSON output: bare or quoted enum variants and strings (`Bill` or `"Bill"`), numbers (`3`), booleans (`true`), `null` for `None`, and `{row: 1, lit: true}` for records (fields in any order). Every input of the action must be supplied.
- `Step::input` is formatted via `earnest::model::Report` (implemented by `#[derive(Domain)]` and built-ins). Compare resulting states with `assert_eq!` or rule methods (`state.stays_locked()`).
- If a step cannot execute, `replay` returns a `Mismatch` naming the step index and the enabled steps at that state.

## Searching states in a test

`explore::<Model>(max_states)` runs the search of `earnest verify` inside a test. It visits start states in `Model::starts()` order and steps in `state.steps()` order, so its paths are the ones the report gives. It returns an `Exploration`:

```rust
use earnest::model::{DEFAULT_MAX_STATES, Status, explore};

let found = explore::<Turnstile>(DEFAULT_MAX_STATES);
assert_eq!(found.status, Status::Pass);
assert_eq!(found.states, 5);
assert!(found.stuck.is_none());
```

On the `Turnstile` above, the search reaches 5 states and passes. With `explore::<Turnstile>(3)` the state limit stops it: `status` is `Unfinished` and `reason` is `Some("state limit of 3")`.

| Field | Meaning |
|---|---|
| `status` | `Pass`, `Fail` or `Unfinished`, as `earnest verify` reports them |
| `states` | The count of distinct states reached, start states included |
| `rules` | One `RuleResult` per rule, in the order given: its `id`, `says` and `verdict`. A `Verdict` is `Holds`, `Broken` with the shortest `Path`, or `Unknown` when the state limit stopped the search before it saw a break |
| `stuck` | `None`, or the shortest `Path` to the first stuck state found, breadth first |
| `reason` | `None`, or why an unfinished search stopped, such as `state limit of 3` |
| `reached()` | Every state reached, once each, in the order the search reached them |
| `path_to(&state)` | A shortest path to a state reached, or `None` |

- A `Path` is a `start` state and `steps`, each a move with the state after it. From `explore` each move is a `Move`, so `replay(&path.start, &moves)` walks the path again.
- `DEFAULT_MAX_STATES` is 5,000,000, the limit of `earnest verify`. The search stops before it holds more states than `max_states`, and a state it counted but did not search from still has its rules judged.
- An end is asked only in a state where no action can happen.
- `explore` does not detect a field that leaves its `#[range]` or `#[one_of]`. It searches on from such a state, where `earnest verify` reports it as out of range. A model with such a state can pass in `explore` and fail `earnest verify`.
- A panic in an action, a rule or an end is not caught.

`search(starts, moves, &rules, end, max_states)` runs the same search over states that are not models. A state needs only `Clone`, `Eq` and `Hash`. `moves` gives each move that can happen in a state with the state after it, in the order to try; a state with no move is stuck unless `end` is `true` there. Each rule is a `Rule::new(id, says, holds)`:

```rust
use earnest::model::{DEFAULT_MAX_STATES, Rule, Status, Verdict, search};

let found = search(
    [0_u8],
    |n: &u8| if *n < 3 { vec![("add", n + 1)] } else { vec![] },
    &[Rule::new("below_three", "The count stays below 3", |n: &u8| *n < 3)],
    |n| *n == 3,
    DEFAULT_MAX_STATES,
);
assert_eq!(found.status, Status::Fail);
assert_eq!(found.states, 4);
let Verdict::Broken(path) = &found.rules[0].verdict else {
    panic!("the count reaches 3");
};
assert_eq!(path.steps.len(), 3);
```

The search counts 4 states, and `below_three` breaks after three `add` moves. The count 3 is an end, so no state is stuck. The [smlang integration](/integrations/smlang/) builds on `search`.

## Keeping models fast and clippy-clean

### What a model costs

In every state, `earnest` evaluates every combination of every action's inputs. Total work is roughly `states × sum(input combinations per action)`. An action taking a `0..=9` integer and a 3-variant enum runs 30 tries per state (300,000 tries across 10,000 states).

To keep models small and fast:

- Pick only the numbers that matter: `#[one_of(0, 1, 5)]` instead of `#[range(0, 9)]`.
- Split actions whose inputs are independent into separate actions with one input each.
- Give every counter a tight `#[range]`, cap collections, and give fields an `#[initial]` when their starting state is fixed.
- Group inputs in a `Domain` struct only when they belong together; combinations still multiply across its fields.
- Use `--max-states` and `--time-limit` while iterating on a new model.

### Clippy pedantic compatibility

The Rust reader accepts the idioms that `cargo clippy -- -D warnings` and pedantic lints ask for:

- Avoid `fn_params_excessive_bools` and `struct_excessive_bools` with two-variant enums (`enum Toggle { On, Off }`) or `Domain` structs.
- Avoid `too_many_arguments` by passing a `#[derive(Domain)]` record (`fn order(&self, line: &Line)`).
- Return `Self` instead of `Option<Self>` for unguarded actions (`unnecessary_wraps`). A witness keeps `Option<bool>`: `#[model]` keeps it clear of the lint.
- Use `u8::from(flag)` and `u16::from(x)` (`bool_to_int_with_if`, `cast_lossless`).
- Use `let x = value?;` instead of `let Some(x) = value else { return None; };` (`question_mark`).
- Combine identical `match` arms with `A | B` and name single remaining variants instead of `_` (`match_same_arms`, `match_wildcard_for_single_variants`, `unnested_or_patterns`).
- Use `n.is_multiple_of(d)` on unsigned integers (`manual_is_multiple_of`).
- Pass non-`Copy` records by reference (`needless_pass_by_value`), small `Copy` types by value (`trivially_copy_pass_by_ref`), and `Option<T>` by value rather than `&Option<T>` (`ref_option`).
- Avoid `manual_range_contains` (`x >= 1 && x <= 3`): when `#[range]` already enforces one bound, check only the other side (the checker does not read `RangeInclusive::contains`).
- Never use `unwrap()` (`unwrap_used`; also refused by the checker).

## Next

- [Primitives and ordering](/reference/primitives/): every built-in type, operation, and deterministic walk order
- [CLI and model discovery](/reference/cli/): commands, flags, and Rust no-model/test-file rules
- [TLA+ export and confirmation](/reference/tla/): exporting and checking models with TLC or `tla-rs`
