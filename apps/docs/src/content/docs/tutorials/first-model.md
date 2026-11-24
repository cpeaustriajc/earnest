---
title: Write your first model
description: Model a turnstile in TypeScript, watch a rule break, read the path, and fix it.
---

In this tutorial you model a turnstile. A visitor pays, then pushes the gate. One rule says that the gate stays locked until the visitor pays. You write a model with a bug, see Earnest find it, and fix it.

You need the checker installed. See [Install](/start/install/).

## 1. Declare the fields

Make a folder, and in it a file `src/turnstile.model.ts`:

```ts title="src/turnstile.model.ts"
import { m } from "@earnestjs/model";

export const Turnstile = m.model("Turnstile", {
	gate: m.enum(["locked", "open"]).initial("locked"),
	paid: m.boolean().initial(false),
	visitors: m.int(0, 3).initial(0),
});
```

Each field declares every value it can take. `m.int(0, 3)` is a whole number from 0 to 3. `.initial(...)` gives the start value. Without it, a field starts at every value it declares, and each one is a start state.

Keep the ranges small. The checker visits every state, so the number of states is the product of the values of each field.

## 2. Add actions

An action has a sentence, an optional `when` that says when it can happen, and `changes`, which returns only the fields that change.

```ts title="src/turnstile.model.ts" ins={1-9}
export const pay = m.action(Turnstile, "the visitor pays", {
	when: (s) => !s.paid,
	changes: () => ({ paid: true }),
});

export const push = m.action(Turnstile, "the visitor pushes", {
	when: (s) => s.visitors < 3,
	changes: (s) => ({ gate: "open", visitors: s.visitors + 1 }),
});
```

The `when` of `push` forgets to check `paid`. That is the bug.

## 3. Add a rule and an end

A rule must hold in every reachable state. An end marks a state where it is correct for nothing more to happen. Any other state where no action can happen is reported as `stuck`.

```ts title="src/turnstile.model.ts" ins={1-7}
export const staysLocked = m.rule(
	Turnstile,
	"The gate stays locked until the visitor pays",
	(s) => s.gate === "locked" || s.paid,
);

export const full = m.end(Turnstile, "Three visitors went through", (s) => s.visitors === 3);
```

The id of each action, rule and end is the name of its exported constant, so `staysLocked` is the rule's id. A rule that you do not export makes the file unreadable.

## 4. Run the checker

```sh
earnest verify
```

```
fail        Turnstile  8 states  0 ms
  broken  staysLocked  "The gate stays locked until the visitor pays"
    start  gate = "locked", paid = false, visitors = 0
    1. push  gate: "locked" -> "open", visitors: 0 -> 1
    end    gate = "open", paid = false, visitors = 1
1 model: 0 pass, 1 fail, 0 unfinished
```

The `broken` line names the rule. Under it is the shortest path to the first state that breaks it:

- `start` gives every field of the start state.
- Each numbered line is one action, with each field that changed as `before -> after`.
- `end` gives every field of the state that breaks the rule.

Here the visitor pushes before paying, and the gate opens. The exit code is 1, so a script or CI job sees the failure.

## 5. Fix the model

Make `push` need a payment:

```ts title="src/turnstile.model.ts" del={2} ins={3}
export const push = m.action(Turnstile, "the visitor pushes", {
	when: (s) => s.visitors < 3,
	when: (s) => s.paid && s.visitors < 3,
	changes: (s) => ({ gate: "open", visitors: s.visitors + 1 }),
});
```

Run the checker again:

```
pass        Turnstile  5 states  0 ms
1 model: 1 pass, 0 fail, 0 unfinished
```

`pass` means the rule holds in all 5 reachable states, and every state where nothing can happen is an end.

## What a model file may contain

The checker reads a model file without running it, so a model file holds less than ordinary TypeScript. It may hold top-level `const` declarations, arrow functions with one expression or a block of `const` and `if (...) return`, comparisons, `&&`, `||`, `!`, `+`, `-`, `*` and the `? :` choice.

It may not hold loops, `let`, assignment, classes, `await`, `Date`, `Math.random` or other imports. Write `m.rem(a, b)` in place of `%`, and `m.div(a, b)` in place of `/`.

If the file uses something else, `earnest verify` prints it as `unreadable`, with the line, the column and a sentence for each problem. The full list is in [TypeScript models](/reference/typescript/#what-a-model-file-may-contain).

## Next

- [Write a model in Rust](/tutorials/rust-model/)
- [Put the result in a pull request](/tutorials/pull-request/)
- [TypeScript models reference](/reference/typescript/)
