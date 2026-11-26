---
title: TypeScript models
description: Write Earnest models in TypeScript, understand allowed syntax and limits, and run models in Node with explore and replay.
---

A TypeScript model is an ordinary TypeScript file that imports `m` from `@earnestjs/model`. `earnest verify` reads the file statically without executing JavaScript or resolving `node_modules`.

## Example

```ts title="src/add-to-cart.model.ts"
import { m } from "@earnestjs/model";

const Press = m.enum(["keyboard", "virtual", "mouse"]);

export const AddToCartFocus = m.model("AddToCartFocus", {
	surface: m.enum(["results", "shortlist"]),
	cta: m.enum(["Add", "Adding", "InCart"]).initial("Add"),
	focus: m.enum(["Add", "InCart", "Heart", "Away", "Lost"]).initial("Add"),
	pressBy: Press.optional().initial(null),
	keyboardAdd: m.boolean().initial(false),
});

export const press = m.action(AddToCartFocus, "the buyer presses Add", {
	input: { by: Press },
	when: (s, { by }) => s.cta === "Add" && (by === "mouse" || s.focus === "Add"),
	changes: (_, { by }) => ({ cta: "Adding", pressBy: by, keyboardAdd: by === "keyboard" }),
});

export const focusLandsOnInCart = m.rule(
	AddToCartFocus,
	"After a keyboard add lands, focus is on In cart",
	(s) => s.pressBy !== "keyboard" || s.cta !== "InCart" || s.focus === "InCart",
);

export const inCart = m.end(AddToCartFocus, "The item is in the cart", (s) => s.cta === "InCart");

export const view = m.view(AddToCartFocus, {
	title: "Add to cart focus",
	state: (s) => `${s.cta}, focus on ${s.focus}`,
	group: (s) => s.surface,
	step: { press: "press Add" },
	colors: { broken: "#c62828" },
});

export type State = m.Infer<typeof AddToCartFocus>;
```

## Model parts

- **Initial values:** A field with `.initial(value)` starts at that value. Without `.initial`, a boolean, number, enum, `oneOf`, optional, or record field starts at every value it declares (in the example above, `surface` creates two start states). Sets, maps, sequences, queues, locks, timers, requests, and retries without `.initial` start empty, free, idle, or at zero.
- **Actions:** `m.action(Model, sentence, { input?, when?, changes, fair? })` accepts `input`, `when`, `changes`, and `fair` in any order. `fair: true` matters only to [rules about runs](#rules-about-runs). `changes` returns only the fields that change. The checker tries every combination of inputs with the first input changing fastest: for inputs `a` and `b`, it tries `(a1, b1)`, `(a2, b1)`, `(a1, b2)`, and so on. `explore` in `@earnestjs/model` and `steps()` in Rust use the same order, so breadth-first walks produce the same shortest path.
- **Exported ids:** The id of an action, rule, end, rule about runs, setup, or witness is the name of its exported constant. Binding one to an unexported constant (neither `export const` nor `export { name }`) makes the file `unreadable`. Duplicate exports (`export { low }; export { low };`, `export { low, low }`, `export const low` plus `export { low }`, or duplicates via `as`) and inline `type` export modifiers (`export { type low }`, `export { low, type top }`) also make the file `unreadable`. Use `export type { Name }` for types.
- **Ends:** `m.end` marks states where stopping is valid. Any other state with no enabled action is reported as `stuck`.
- **Views:** `m.view` is optional. Without it, statechart boxes list every field (for example, `n = 0, q = 0, by = 1`).
  - `state` shortens the box label; `group` groups states into subgraphs.
  - `step` maps action ids to arrow labels (defaults to the action sentence). Naming an action id that does not belong to the model makes the file `unreadable`.
  - `colors` accepts `broken`, `start`, and `end`, each given as `#` with 3, 4, 6, or 8 hex digits or an alphabetic CSS color name. Any other key or color format makes the file `unreadable`.
- **Witnesses:** `m.witness(Model, sentence, { setup?, steps, expect })` declares a path the model must be able to take. See [Witnesses](#witnesses).
- **Rules about runs:** `m.eventually(Model, sentence, { after?, reaches })` declares a state every run comes to. See [Rules about runs](#rules-about-runs).
- **Setups:** `m.setup(Model, sentence, { field: value })` checks the model once more from other start values. See [Setups](#setups).

## Witnesses

A witness names steps the model must be able to take from every start state, and what must be true after the last one. A rule is checked in every reachable state; a witness is checked on the one path its steps describe.

```ts title="src/panel.model.ts"
export const reopens = m.witness(Panel, "Open, show more, close, open: the panel opens on the first page", {
	steps: [open, m.step(more, { by: "mouse" }), close, open],
	expect: (s) => s.open && s.page === 1,
});
```

- **Steps:** `steps` is a list written out in the call. Each step is the exported constant of an action of the same model declared earlier in the file. An action with inputs is written `m.step(action, { name: value })`, with a value for every input it declares. `steps: []` checks only the start states.
- **Expect:** `expect` is a function of the state, like a rule's condition. It must give a boolean.
- **Playing:** `earnest verify` plays the steps in order from every start state of the model, in start-state order. The witness holds when, from every start state, each step can happen in turn and `expect` is true after the last step.
- **Broken:** the result is the path played until the witness broke, from the first start state that breaks it, and a reason: a step that can't happen, a step that takes a field out of the values it declares (that step is in the path), a fault in a step's condition or change, a fault in `expect`, or `expect` false in the last state. A broken witness makes the model `fail`. See [Output, JSON, and reports](/reference/output/) for how it is printed.
- **Unknown:** a model with more start states than `--max-states` plays only that many. The witness is `unknown` unless one of them broke it.
- **Listing:** witnesses are listed after the model's rules and rules about runs, in file order, and `--rule <id>` selects one by its id. `earnest confirm` leaves witnesses out of its comparison and `earnest tla` does not export them.
- **Setup:** with `setup: behind`, the steps are played from the start states of that [setup](#setups) instead of the model's, and the witness is listed only with that setup's result. Without `setup`, it is listed only with the model's own result.

A witness makes the file `unreadable` when a step is not an action of this model declared above the witness (another model's action, a rule, an unknown name, a spread, a hole, or any other expression), when `steps` is not a list written out, when `steps` or `expect` is missing, when it has another key, or when it is not exported. `earnest verify` also refuses an input the action does not declare, an input given twice, an input left without a value (including a bare action that takes inputs), an input value outside the input's declaration, and an `expect` that does not give a boolean.

A Rust model writes a witness as a method marked `#[witness("...")]`: see [Rust models](/reference/rust/#rules-about-runs-witnesses-and-setups).

## Rules about runs

A rule about runs says that every run comes to a state where a condition is true. A rule is checked in each state alone; a rule about runs is checked on whole runs.

```ts title="src/cart.model.ts"
export const offered = m.eventually(Cart, "Once approved, the cart offers checkout", {
	after: (s) => s.approved,
	reaches: (s) => s.offers,
});
```

- **Where it starts:** with `after`, the rule starts in every reachable state where `after` is true. Without `after`, it starts in every start state.
- **What it wants:** every run from a state where the rule starts comes to a state where `reaches` is true. That state may be the one the rule starts in. `after` and `reaches` are functions of the state, like a rule's condition, and must give a boolean.
- **How a run goes on:** a run never stops while some action can still change the state. An action whose change keeps the state exactly as it is counts as no step. So a run can stay forever in a state where no action can happen (such as an `end`), or where every action that can happen changes nothing.
- **Fair actions:** `fair: true` on an action adds that the action is not put off forever while it stays possible. A loop in which a fair action can happen in every state, and is never taken, is not a run. A fair action with inputs is fair as a whole: a run must take it with some input, not with each one.
- **Broken:** the result is the shortest path from a start state into a run that never reaches what the rule wants, and a reason. When the run loops, the path goes once round the loop and the reason is `steps A to B can repeat forever`, counted from 1. When the run stays in the last state of the path, the reason is `nothing changes the state from here, so the run can stay in it forever`. A broken rule about runs makes the model `fail`. See [Output, JSON, and reports](/reference/output/) for how it is printed.
- **Unknown:** the verdict is `unknown`, never `holds`, when the search stopped at `--max-states` or `--time-limit`, when a state went out of range, when an action or an `end` faulted, or when `after` or `reaches` faulted in some state. A fault in `after` or `reaches` is also a `fault` finding that names the rule, unless the model already has a fault.
- **Listing:** rules about runs are listed after the plain rules and before the witnesses, in file order, and `--rule <id>` selects one by its id.
- **Memory:** to judge these rules, the search keeps every arrow between states until it ends. A model without a rule about runs is searched as before.

A rule about runs makes the file `unreadable` when `reaches` is missing, when it has another key, when `after` or `reaches` is not a function of the state, or when it is not exported. `fair` must be `true` or `false` written out. `earnest verify` also refuses an `after` or `reaches` that does not give a boolean.

`earnest tla` exports each rule about runs as a temporal property and each fair action as weak fairness, so `earnest confirm --tlc` can compare the verdicts with TLC. See [TLA+ export](/reference/tla/#rules-about-runs-and-fairness).

A Rust model writes a rule about runs as a method marked `#[eventually("...")]`, and a fair action as `#[action("...", fair)]`: see [Rust models](/reference/rust/#rules-about-runs-witnesses-and-setups).

## Setups

A setup checks the same model once more, with some fields starting at other values. A field no action changes works as a parameter of the model this way, so one model covers several configurations.

```ts title="src/clock.model.ts"
export const behind = m.setup(Clock, "The clock runs 10 behind", { skew: -10 });
export const ahead = m.setup(Clock, "The clock runs 10 ahead", { skew: 10 });
```

- **Start values:** each field the setup names starts at that one value. Every other field starts where the model declares, so a field without `.initial` still gives one start state per value.
- **Results:** the model as declared is still checked. Each setup is one more result, after it in declaration order, named `Clock (behind)` in text output, reports, and `earnest confirm`, and as a `models` entry with a `setup` key in JSON. Each result has its own state count, rules, rules about runs, witnesses, paths, and findings, and the summary counts each one.
- **Witnesses:** a witness with `setup: behind` is played from that setup's start states and listed only with that setup. A witness without `setup` is listed only with the model as declared.
- **Selecting:** `--model Clock` selects the model and all its setups. `--rule <id>` selects each result that lists that id: a rule selects all of them, and a witness with a setup selects only that setup.
- **Cache and rule changes:** the cache keeps each result apart. `--base` lists each rule change of the model once, not once per setup.
- **TLA+:** `earnest tla` writes one module per setup, named `Clock_behind`, with the setup's values in `Init`.

A setup makes the file `unreadable` when it names no field, when it is not exported, or when it is not written as `m.setup(Model, "sentence", { field: value })`. `earnest verify` also refuses a field the model lacks, a field named twice, and a value outside the field's declaration. A witness's `setup` must be the exported constant of a setup of the same model declared above the witness.

A Rust model writes a setup as a method marked `#[setup("...")]` that gives back `Self { field: value, ..self.clone() }`: see [Rust models](/reference/rust/#rules-about-runs-witnesses-and-setups).

## What a model file may contain

Allowed top-level items:

- `import { m } from "@earnestjs/model"` and `import type` from anywhere.
- Top-level `const` declarations for type declarations (`m.enum(...)`), numbers, strings, booleans, and arrow helper functions. Constant values may be literals or computed from earlier constants using the allowed operators.
- Models, actions, rules, ends, views, witnesses, rules about runs, and setups, each created by calling `m.model`, `m.action`, `m.rule`, `m.end`, `m.view`, `m.witness`, `m.eventually`, or `m.setup` directly as the initializer of a top-level `const`. `m.step` is written only inside the `steps` of a witness, and the constant of a setup only as the `setup` of a witness. Referencing a model's name anywhere other than as the first argument of those calls or in a type is refused, so parts cannot be built dynamically or via aliases. Each constant must be exported under its own name.
- `type` and `interface` declarations.

Allowed inside functions:

- Literals, `null`, state field reads, and input reads.
- Operators: `===`, `!==`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `!`, `+`, `-` (binary and unary), `*`, `? :`.
- `m.rem(a, b)`, `m.div(a, b)`, and `m.some(value)`.
- Object literals with spread (`{ ...r, field: v }`), calls to [primitives](/reference/primitives/) and top-level helper functions, `as const`, and template literals in `view` labels.
- Function bodies may be a single expression, or a block of `const` declarations and `if (...) return ...;` statements ending in a `return` (with nothing after that `return`).

### Scoping, numbers, records, and nesting

- **Unique names per scope:** Duplicate `const` declarations in the same scope, a `const` colliding with a function parameter, and duplicate parameter names (`(n, n) => ...`) are refused. Nested blocks and function parameters may shadow outer names.
- **Lexical scoping:** An action input is in scope only inside a function that declares it, either destructured (`(s, { by }) => ...`) or as an object (`(s, input) => input.by`). `when` and `changes` may take different inputs; where a function does not take an input, that identifier refers to any top-level constant of the same name. A block `const` is scoped to the whole block, so reading it before its declaration or inside its own initializer (`const a = a + 1;`) is refused.
- **Numbers:** Numbers are exact JavaScript safe integers, from `-9007199254740991` to `9007199254740991` (`-(2^53 - 1)` to `2^53 - 1`). Literals, `m.int` bounds, and `m.oneOf` numbers outside this range are refused (and throw in `@earnestjs/model`). Computing a number outside this range during search is a `fault` (`"computes a number too large to hold"`) in rules, ends, `when`, `changes`, and helpers.
- **Records:** A record literal matches a declared record when each field matches (optional fields accept a value or `null`). Records from conditional branches, `const` bindings, spread expressions, helper returns, or entries in `m.mapOf`, `m.setOf`, and `m.seqOf` unify into the declared record type. Unlike Rust, TypeScript records are structural: two records with the same fields have the same type.
- **Nesting depth:** Expressions are parsed up to a nesting depth of `1000` (brackets, prefix operators, operator chains, and nested statements; characters inside strings, template literal text, or comments do not count). Deeper files are reported as `unreadable` without blocking other models in the run.

### Disallowed constructs

Any construct outside the allowed subset makes the file `unreadable`, including:

- `await`, loops, `let`, assignments, `new`, classes, `fetch`, `Date`, `Math.random`, value imports other than `@earnestjs/model`, duplicate keys in an object literal, duplicate values in `m.enum` or `m.oneOf`, and undefined names (even inside unused constants).
- **`%` and `%=`:** Use `m.rem(a, b)` instead. Languages disagree on signed remainder, so `m.rem` faults when `a < 0` or `b < 1` in both `earnest verify` and `@earnestjs/model`.
- **`/` and `/=`:** Use `m.div(a, b)` instead. JavaScript `/` produces fractions while Rust `/` truncates; `m.div` computes whole-number division and faults when `a < 0` or `b < 1` in both `earnest verify` and `@earnestjs/model`.
- **Type assertions (`as`, `satisfies`, `<Type>value`) except `as const`, and postfix `!`:** These erase `null` at compile time without checking at runtime (`(s.o as number) < 3` evaluates `null < 3` as `true` in JavaScript). Use `m.some(value)` instead, which faults on `null` in both `earnest verify` and `@earnestjs/model`.

## Running a model in Node

`@earnestjs/model` also executes models directly in Node:

- `explore(Model, { exports })` performs breadth-first search in Node and returns `{ status, states, rules, findings, reason }` using the same shapes and verdicts as `earnest verify --json`. It judges each rule about runs and plays each witness after the rules, as `earnest verify` does, with the same paths, reasons, and `repeatsFrom`.
- `explore(Model, { exports, setup: behind })` checks the model in one setup, as the setup's own result from `earnest verify` does. Without `setup`, it checks the model as declared. It throws when the setup is not one declared on this model.
- `m.witness` throws when a step is not an action declared on the model before the witness, when a step gives an input the action does not declare, or when `setup` is not a setup declared on the model before the witness. TypeScript does not compile a bare action that takes inputs in `steps`, nor `m.step` with an input the action does not declare.
- `m.setup` throws when it names no field, a name that is not a field of the model, or a value the field does not declare. TypeScript does not compile a setup that names a field the model lacks or gives a value of the wrong type, nor an `after` or `reaches` that does not give a boolean.
- `replay(Model, path, { exports })` steps through a path from `earnest verify --json` one action at a time and returns every state along the path.
- `replay(Model, path, { exports, setup: behind })` replays a path of a setup's result: the path must begin at one of the setup's start states, as `explore` with that `setup` starts. Without `setup`, the path must begin at a start state of the model as declared.

- `search({ starts, key, moves, rules })` runs the same breadth-first search on states that are not a model, such as plain objects or a snapshot of a state machine.

One difference exists between `explore` in Node and `earnest verify`: JavaScript operators cannot be intercepted from a library, so an intermediate number exceeding `2^53 - 1` inside a rule, `end`, or `when` evaluates using JavaScript's rounded arithmetic in Node instead of faulting. Only a next state storing a value past `2^53 - 1` faults in Node. `earnest verify` is the authority on integer overflow faults. See [`packages/model/README.md`](https://github.com/cpeaustriajc/earnest/blob/main/packages/model/README.md) for the full Node API.

### Searching states that are not a model

`search` takes the start states, a `key` that gives two states the same text exactly when they are the same state, and `moves`, which lists `{ move, next }` for each move that can happen from a state, in the order to try:

```ts
import { search } from "@earnestjs/model";

const found = search({
  starts: [{ gate: "Locked", paid: false }],
  key: (s) => `${s.gate};${s.paid}`,
  moves: (s) =>
    s.gate === "Locked"
      ? [
          ...(s.paid ? [] : [{ move: "pay", next: { ...s, paid: true } }]),
          { move: "push", next: { ...s, gate: "Open" } },
        ]
      : [{ move: "enter", next: { gate: "Locked", paid: false } }],
  rules: [
    {
      id: "locked",
      says: "The gate stays locked until the visitor pays",
      holds: (s) => s.gate === "Locked" || s.paid,
    },
  ],
});
```

The search counts 4 states and fails: `locked` is `broken` after one `push`. The result has the fields of `explore` (`status`, `states`, `rules`, `findings`, `reason`), but each path is a trail of your own values, `{ start, steps: [{ move, state }] }`.

- A state with no move is `stuck` unless `isEnd` gives `true` for it. `isEnd` is asked only of a state with no move.
- When `outside` gives a message for a state, the state is out of range: it is counted, is not checked against the rules and is not searched from. The first one is an `out-of-range` finding, and a rule not seen broken is `unknown`.
- `maxStates` works as in `explore`, and the default is 5,000,000.
- `reached()` gives the shortest trail to every counted state, in the order the search reached them.
- `search` has no faults: an error thrown by `key`, `moves`, a rule, `isEnd` or `outside` is thrown by `search`.

## Next

- [Primitives and ordering](/reference/primitives/): every built-in type, operation, and deterministic walk order
- [CLI and model discovery](/reference/cli/): commands, flags, and test-file rules
- [Output, JSON, and reports](/reference/output/): text output, JSON schema, and PR reports
