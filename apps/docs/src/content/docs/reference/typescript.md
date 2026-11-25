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
- **Actions:** `m.action(Model, sentence, { input?, when?, changes })` accepts `input`, `when`, and `changes` in any order. `changes` returns only the fields that change. The checker tries every combination of inputs with the first input changing fastest: for inputs `a` and `b`, it tries `(a1, b1)`, `(a2, b1)`, `(a1, b2)`, and so on. `explore` in `@earnestjs/model` and `steps()` in Rust use the same order, so breadth-first walks produce the same shortest path.
- **Exported ids:** The id of an action, rule, or end is the name of its exported constant. Binding one to an unexported constant (neither `export const` nor `export { name }`) makes the file `unreadable`. Duplicate exports (`export { low }; export { low };`, `export { low, low }`, `export const low` plus `export { low }`, or duplicates via `as`) and inline `type` export modifiers (`export { type low }`, `export { low, type top }`) also make the file `unreadable`. Use `export type { Name }` for types.
- **Ends:** `m.end` marks states where stopping is valid. Any other state with no enabled action is reported as `stuck`.
- **Views:** `m.view` is optional. Without it, statechart boxes list every field (for example, `n = 0, q = 0, by = 1`).
  - `state` shortens the box label; `group` groups states into subgraphs.
  - `step` maps action ids to arrow labels (defaults to the action sentence). Naming an action id that does not belong to the model makes the file `unreadable`.
  - `colors` accepts `broken`, `start`, and `end`, each given as `#` with 3, 4, 6, or 8 hex digits or an alphabetic CSS color name. Any other key or color format makes the file `unreadable`.

## What a model file may contain

Allowed top-level items:

- `import { m } from "@earnestjs/model"` and `import type` from anywhere.
- Top-level `const` declarations for type declarations (`m.enum(...)`), numbers, strings, booleans, and arrow helper functions. Constant values may be literals or computed from earlier constants using the allowed operators.
- Models, actions, rules, ends, and views, each created by calling `m.model`, `m.action`, `m.rule`, `m.end`, or `m.view` directly as the initializer of a top-level `const`. Referencing a model's name anywhere other than as the first argument of those calls or in a type is refused, so parts cannot be built dynamically or via aliases. Each constant must be exported under its own name.
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

- `explore(Model, { exports })` performs breadth-first search in Node and returns `{ status, states, rules, findings, reason }` using the same shapes and verdicts as `earnest verify --json`.
- `replay(Model, path, { exports })` steps through a path from `earnest verify --json` one action at a time and returns every state along the path.

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
