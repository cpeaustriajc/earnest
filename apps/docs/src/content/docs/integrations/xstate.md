---
title: XState
description: Check every state an XState v5 machine can reach from a test, and run each shortest path through your own model-based tests.
---

The `@earnestjs/xstate` package puts Earnest's exhaustive search under a machine written with [XState](https://stately.ai/docs) v5. The machine and `createTestModel(...)` from `@xstate/graph` stay as written. The package has two parts:

- `earnestPaths`, a path generator: `model.getPaths(earnestPaths)` takes the place of `model.getShortestPaths()` and gives one shortest path to every reachable state.
- `checkMachine(machine, { events, rules })`, which gives what a path generator can't report: the count of states, each rule's verdict with the shortest path of events that breaks it, and the first stuck state.

XState's own model-based tests walk the events a person listed and the paths its generator chose. The search here tries every listed event in every reachable state, so its state count shows how much it covered, and `checkMachine` judges every rule in every state it reaches.

## Add it

The package is not on npm yet. When it ships, the install line is:

```sh
npm install --save-dev @earnestjs/xstate xstate @xstate/graph
```

`xstate` (5.19.4 or newer) and `@xstate/graph` (3.0.4 or newer) are peer dependencies, so the test uses the copies the project already has. The package needs Node 22.18 or newer.

## Example: the paths of a turnstile

This turnstile takes a payment to unlock, a push to open, and an entry to count a visitor. It holds two visitors, then ends in a final state. The machine is `test/machines/turnstile.ts` in `packages/xstate`. It has a guard and an `assign`:

```ts title="test/machines/turnstile.ts"
import { assign, setup } from "xstate";

export const CAPACITY = 2;

export const turnstile = setup({
  types: {
    context: {} as { visitors: number },
    events: {} as { type: "PAY" } | { type: "PUSH" } | { type: "ENTER" },
  },
  guards: {
    isLastVisitor: ({ context }) => context.visitors + 1 === CAPACITY,
  },
  actions: {
    countVisitor: assign({ visitors: ({ context }) => context.visitors + 1 }),
  },
}).createMachine({
  id: "turnstile",
  initial: "locked",
  context: { visitors: 0 },
  states: {
    locked: {
      on: {
        PAY: { target: "paid" },
      },
    },
    paid: {
      on: {
        PUSH: { target: "open" },
      },
    },
    open: {
      on: {
        ENTER: [
          { guard: "isLastVisitor", target: "full", actions: "countVisitor" },
          { target: "locked", actions: "countVisitor" },
        ],
      },
    },
    full: {
      type: "final",
    },
  },
});
```

`Gate` is a class in `test/gate.ts` that claims to work as the machine says, and the test checks it against every path:

```ts title="test/turnstile.test.ts"
import { createTestModel } from "@xstate/graph";
import { earnestPaths } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { turnstile } from "./machines/turnstile.ts";
import { Gate } from "./gate.ts";

const model = createTestModel(turnstile);

for (const path of model.getPaths(earnestPaths)) {
  test(path.description, async () => {
    const gate = new Gate();
    await path.test({
      events: { PAY: () => gate.pay(), PUSH: () => gate.push(), ENTER: () => gate.enter() },
      states: {
        locked: () => expect(gate.state).toBe("locked"),
        paid: () => expect(gate.state).toBe("paid"),
        open: () => expect(gate.state).toBe("open"),
        full: () => expect(gate.state).toBe("full"),
      },
    });
  });
}
```

Only the `getPaths(earnestPaths)` call differs from a test built on `getShortestPaths()`. `path.test` runs the path's events through the functions in `events` and calls the check in `states` for each state it passes.

### The seven states

Write a state as the machine's state value and the visitors counted. Starting from `locked` with 0 visitors, the search reaches seven states:

| State | `PAY` | `PUSH` | `ENTER` |
|---|---|---|---|
| `locked`, 0 | `paid`, 0 | no event | no event |
| `paid`, 0 | no event | `open`, 0 | no event |
| `open`, 0 | no event | no event | `locked`, 1 |
| `locked`, 1 | `paid`, 1 | no event | no event |
| `paid`, 1 | no event | `open`, 1 | no event |
| `open`, 1 | no event | no event | `full`, 2 |
| `full`, 2 | no event | no event | no event |

`full` is a final state, so it takes no event. The test `reaches every state once, as getShortestPaths does` in `test/paths.test.ts` asserts 7 paths with `allowDuplicatePaths: true`.

`getPaths` drops a path that a longer one contains, as it does for `getShortestPaths`. Every state of the turnstile lies on the path to `full`, so the loop above runs one test, of six events: `PAY`, `PUSH`, `ENTER`, `PAY`, `PUSH`, `ENTER`. Pass `{ allowDuplicatePaths: true }` to get all seven paths.

### What a path holds

- A path has the shape of XState's own shortest paths: a first step with the start snapshot and the event `xstate.init`, one step per event with the snapshot after it, `state` (the last snapshot) and `weight` (the number of events).
- The options of `getPaths` work as they do for `getShortestPaths`. The search starts at `fromState`, or at the initial snapshot made with `input`. It sends the events `events` lists, does not go on from a state for which `stopWhen` is true (by default `toState`), and gives only the paths that end in a state for which `toState` is true. `serializeState` tells states apart.
- A finite `limit` is the most distinct states the search may hold. Without one, the limit is the 5,000,000 of `search` in `@earnestjs/model`. When the states do not fit, `getPaths` throws an error that names the limit and the count, such as `earnestPaths stopped at the state limit of 3 after 3 states`, and gives no paths.

## Example: a faulty turnstile

`checkMachine` takes a machine, the events to try in every state, and the rules to judge. This turnstile mirrors the Earnest conformance model: the gate is `locked` or `open`, and the context holds `paid`. `PUSH` opens a locked gate without asking for payment. The machine is `test/machines/open-turnstile.ts`:

```ts title="test/machines/open-turnstile.ts"
import { assign, setup } from "xstate";

export const openTurnstile = setup({
  types: {
    context: {} as { paid: boolean },
    events: {} as { type: "PAY" } | { type: "PUSH" } | { type: "ENTER" },
  },
}).createMachine({
  id: "openTurnstile",
  initial: "locked",
  context: { paid: false },
  states: {
    locked: {
      on: {
        PAY: { guard: ({ context }) => !context.paid, actions: assign({ paid: true }) },
        PUSH: { target: "open" },
      },
    },
    open: {
      on: {
        ENTER: { target: "locked", actions: assign({ paid: false }) },
      },
    },
  },
});
```

The test gives `checkMachine` the three events and one rule. A rule has an id (its key in `rules`), a sentence `says`, and a predicate `holds` on the snapshot:

```ts title="test/open-turnstile.test.ts"
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { openTurnstile } from "./machines/open-turnstile.ts";

test("the gate opens unpaid after one push", () => {
  const found = checkMachine(openTurnstile, {
    events: [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }],
    rules: {
      locked: {
        says: "The gate is never open unpaid",
        holds: (snapshot) => snapshot.matches("locked") || snapshot.context.paid,
      },
    },
  });
  expect(found.status).toBe("fail");
  expect(found.states).toBe(4);
  expect(found.rules[0]?.verdict).toBe("broken");
  expect(found.rules[0]?.path?.events).toStrictEqual([{ type: "PUSH" }]);
});
```

The machine has 4 states: `locked` and `open`, each paid and unpaid. This is the count `earnest verify` gives for the same model in a `turnstile.model.ts`. The rule `locked` is broken in the third state the search reaches, after the one event `PUSH`, which leaves the gate open with `paid` false.

### Read the result

`checkMachine` gives the fields of `explore` from `@earnestjs/model`:

| Field | Meaning |
|---|---|
| `status` | `pass`, `fail` or `unfinished` |
| `states` | The count of distinct states reached, the start state included |
| `rules` | One result per rule, in the order of the `rules` keys: `id`, `says`, `verdict` and `path` |
| `findings` | A problem that is not a broken rule: at most one stuck state, with its `path` |
| `reason` | `null`, or why an unfinished check stopped, such as `state limit of 3` |
| `reached()` | The shortest path to every state reached, once each, in the order the search reached them |

A rule's `verdict` is `holds`, `broken` or `unknown`. `path` is the shortest path to a snapshot that breaks the rule, and `null` when none does. `unknown` means the state limit stopped the search before it saw a break. `rules` follows the key order of the object, where JavaScript puts integer-like keys first.

A path has the initial snapshot `start`, one step `{ event, snapshot }` per event, `events` (the events alone) and `end`, the last snapshot. Each path is built when asked for.

### Send a path to a real actor

`events` is ready to `send` to an actor of the machine. `test/check.test.ts` does this with the path of the broken rule, and checks that the rule fails on the actor too:

```ts
const actor = createActor(openTurnstile).start();
for (const event of found.rules[0]?.path?.events ?? []) {
  actor.send(event);
}
expect(openUnpaid.locked.holds(actor.getSnapshot())).toBe(false);
```

`openUnpaid` is the rule above, kept in a constant in that file. When the actor does not reach the state the search said, the search and the machine disagree about a transition.

### Stuck states and final states

A snapshot whose status is `done` (a top-level final state) is an end. Any other snapshot where none of the listed events can happen is stuck: the check fails, and `findings` holds the shortest path to the first one.

An event is tried only where `snapshot.can(event)` is true, so a guard that fails, or a transition with no target and no action, is no event at all. In `test/check.test.ts` a machine that goes `idle` to `jammed` on `START` has `RETRY` guarded by `() => false` and `WAIT` as an empty transition. Its `jammed` state is stuck, with the path `START`, and the `finished` state of type `final` is not. The working turnstile above passes with 7 states because `full` is final.

### The state limit

`maxStates` is the most distinct states the check may hold, 5,000,000 by default. When the limit stops the check before it finds anything wrong, `status` is `unfinished`, `reason` is `state limit of N`, and a rule not seen broken is `unknown`. A rule seen broken stays `broken`, and the status is then `fail`. The test `a state limit below the reachable count leaves the check unfinished, no rule holding` runs the working turnstile with `maxStates: 3`: it counts 3 states and both rules are `unknown`.

A context field that nothing bounds, such as a counter that only goes up, makes the state space grow without end, so the check runs to the limit. Bound such a field with a guard, as `isLastVisitor` bounds the visitors.

### Other options

- `events` is a list or a function of the snapshot, as in `@xstate/graph`, or a record that declares each event's payload, as in the next example.
- `input` is the machine's input, used to make the initial snapshot.

## Example: a count with no bound

A context is ordinary JavaScript, so a count that grows forever gives a new state on every event, and only `maxStates` stops the search. `checkMachine` takes declarations written with `m` from `@earnestjs/model` that bound it. This machine counts visitors who enter in groups and never stops:

```ts title="test/machines/visitor-count.ts"
import { assign, setup } from "xstate";

export const visitorCount = setup({
  types: {
    context: {} as { visitors: number },
    events: {} as { type: "ENTER"; group: number },
  },
}).createMachine({
  id: "visitorCount",
  context: { visitors: 0 },
  on: {
    ENTER: {
      actions: assign({ visitors: ({ context, event }) => context.visitors + event.group }),
    },
  },
});
```

The test declares the group sizes `ENTER` is tried with and the range of `visitors`:

```ts title="test/visitor-count.test.ts"
import { m } from "@earnestjs/model";
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { visitorCount } from "./machines/visitor-count.ts";

test("the count leaves its range after two groups of two", () => {
  const found = checkMachine(visitorCount, {
    events: { ENTER: { group: m.int(1, 2) } },
    context: { visitors: m.int(0, 3) },
    rules: {
      counted: {
        says: "The count is never below zero",
        holds: (snapshot) => snapshot.context.visitors >= 0,
      },
    },
  });
  expect(found.status).toBe("fail");
  expect(found.states).toBe(6);
  expect(found.rules[0]?.verdict).toBe("unknown");
  expect(found.findings[0]?.message).toBe(
    "the context field `visitors` leaves the values it declares",
  );
  expect(found.findings[0]?.path.events).toStrictEqual([
    { type: "ENTER", group: 2 },
    { type: "ENTER", group: 2 },
  ]);
});
```

The counts 0 to 3 are in range. Groups of 1 and 2 reach 4 and 5 from them, and those two states are out of range, so the search counts 6 states. The first state out of range is 4, after two groups of 2. Nothing is searched from 4 or 5.

### Bound the context

`context` declares some fields of the context. A snapshot where one of them holds a value its declaration does not have is out of range, as a model's state is for `earnest verify`:

- The state is counted, no rule is judged in it, and nothing is searched from it.
- `findings` gives the first one as `out-of-range`, with its shortest path and a message that names the field.
- The check fails, and a rule not seen broken is `unknown`.
- A field the schema names but the context lacks, or holds `undefined`, is out of range, unless its declaration is `.optional()`. An optional field is in range when it is missing, `undefined` or `null`, so it does not tell a stray `undefined` from `null`.
- For the union context `{ mode: "on"; level: number } | { mode: "off" }`, declare `context: { level: m.int(0, 3).optional() }`. Without `.optional()`, every snapshot of the member that lacks `level` is out of range.
- The same holds inside a record, at any depth: an optional field there is in range when it is missing, `undefined` or `null`, and any other missing field is out of range. A field set to `undefined` is the same as no field.
- A value a record declaration checks must be plain data at every level. An object that is not a plain object, such as a `Date` where `m.record({})` is declared, is out of range.
- A field the schema does not name is not checked.

### Declare event payloads

`events` can be a record of event types, each with a declaration for each payload field, such as `{ PAY: { coins: m.int(1, 2) }, PUSH: {} }`:

- Each type is tried with every combination of its fields' values, the first field varying fastest, as `earnest verify` tries an action's inputs.
- The event sent, and shown in a path, is `{ type, ...payload }`.
- A type with more than 1,000,000 payload combinations is a `RangeError`.
- A record payload arrives frozen and is shared by every snapshot it is sent from, so copy it and never change it in place.

### Which declarations a schema takes

The declarations must hold plain JavaScript values: `m.boolean()`, `m.int`, `m.enum`, `m.oneOf`, `.optional()` of those, and `m.record` of all of these. A set, map, sequence, queue, lock, timer, request or retry holds objects of `@earnestjs/model` that a context or an event does not. `checkMachine` throws a `TypeError` that names the field before it searches.

An entry that is not a declaration, which plain JavaScript or a cast lets through, is also a `TypeError` before the search. The message names the entry and says what was found, as in ``checkMachine can't take context.visitors: it is the number 1, not a declaration; write one with m from @earnestjs/model, such as m.int(0, 3), or m.record({ ... }) for an object``. An event type's entry must be a record of payload fields, `{}` for none. Anything else, a whole `m.record(...)` included, is refused the same way.

The keys of `context`, the event types and their payload fields are checked against the machine's types, so a misspelt one does not compile. `context` may name a field of any member of a union context.

## Example: write the machine out for TLC

`checkMachine` searches in the test's own process. `modelFileOf` writes the machine out as a [model file](/reference/typescript/), so `earnest verify` and TLC can confirm that search on their own. It takes the options of `checkMachine` and a `name` for the model. This test writes the faulty turnstile, with its context declared:

```ts title="test/open-turnstile-model.test.ts"
import { readFileSync } from "node:fs";
import { m } from "@earnestjs/model";
import { modelFileOf } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { openTurnstile } from "./machines/open-turnstile.ts";

test("the open turnstile is written out as a model file", () => {
  const { text, threw } = modelFileOf(openTurnstile, {
    name: "OpenTurnstile",
    events: [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }],
    context: { paid: m.boolean() },
    rules: {
      locked: {
        says: "The gate is never open unpaid",
        holds: (snapshot) => snapshot.matches("locked") || snapshot.context.paid,
      },
    },
  });
  expect(threw).toStrictEqual([]);
  const committed = new URL("models/open-turnstile.model.ts", import.meta.url);
  expect(text).toBe(readFileSync(committed, "utf8"));
});
```

`text` is the committed file:

```ts title="test/models/open-turnstile.model.ts"
import { m } from "@earnestjs/model";

export const OpenTurnstile = m.model("OpenTurnstile", {
  state: m.enum(["locked", "open"]).initial("locked"),
  paid: m.boolean().initial(false),
});

export const PAY = m.action(OpenTurnstile, "PAY", {
  when: (s) => s.state === "locked" && !s.paid,
  changes: () => ({ paid: true }),
});

export const PUSH = m.action(OpenTurnstile, "PUSH", {
  when: (s) => s.state === "locked",
  changes: () => ({ state: "open" }),
});

export const ENTER = m.action(OpenTurnstile, "ENTER", {
  when: (s) => s.state === "open",
  changes: () => ({ state: "locked", paid: false }),
});

export const locked = m.rule(
  OpenTurnstile,
  "The gate is never open unpaid",
  (s) => !(s.state === "open" && !s.paid),
);

export type State = m.Infer<typeof OpenTurnstile>;
```

`earnest verify test/models/open-turnstile.model.ts` reports what `checkMachine` reported: `fail`, 4 states, and `locked` broken after the one action `PUSH`. `earnest confirm --tlc tla2tools.jar test/models/open-turnstile.model.ts` prints `same     OpenTurnstile  4 states  fail`, so TLC finds the same. [TLA+ export and confirmation](/reference/tla/) has the commands.

### How the file is made

`modelFileOf` reads no guard and no `assign` as source. It runs the machine's own `transition` with every listed event and declared payload, on every state value the machine can take, with every combination of declared context values. It writes the results as tables. A guard may call a helper, read a constant or look up a `Map`.

- The combinations include contexts `checkMachine` never reaches, so `earnest verify` searches the whole table from the start.
- The model has the field `state`, a label for the state value such as `locked` or `on.lid.shut, on.water.cold`, and one field for each declared context field.
- Where two states differ only in what a history state remembers, the label says so, as in `off (on.last: on.high)`.
- Each event type is an action with the payload fields as its `input`. Each rule keeps its id and sentence. A top-level final state is an `m.end`.
- An id that is not a name, such as `user.click`, becomes one, such as `user_click`. `actions` and `rules` in the result map each exported id back.
- A transition that leaves a declaration is written as it is, so `earnest verify` reports the state out of range, as `checkMachine` does.
- Where `checkMachine` reaches a state, the machine is given the context `checkMachine` reaches there, so an optional field stays missing, `undefined` or `null` as the machine left it. A missing field and `undefined` read back as `null`.
- The same machine and options give the same text. `maxStates` is not written: pass `--max-states` to `earnest verify`.

### What it refuses and what `threw` holds

`modelFileOf` throws an error that names the field or the event for:

- a context field without a declaration in `context`;
- `events` given as a function, or a listed event with a payload such as `{ type: "ENTER", group: 1 }`. Write those as a record of event types;
- a table of more than `MOST_TABLE_ROWS` (100,000) rows, where a row is one state value with one combination of context values and one event with one payload, or one rule;
- a next context that the declaration's type can't hold, such as text in a field declared `m.boolean()`;
- an optional field, at any depth, left missing or `undefined` in one state and set to `null` in another that is otherwise the same. `checkMachine` counts two states there, and the file has one `null` for both.

A guard, `assign` or rule that throws at a point of the table is listed in `threw`. The file makes reaching that point visible: the event sets an extra field `threw` out of range, and the rule is broken there. Expect `threw` to be empty, as the test above does.

### What the confirmation shows

It shows that the search over the machine's own transitions, on the declared values, gives the same states and verdicts in `earnest verify` and in TLC. It does not check the machine against the app. Test the app against the machine with `earnestPaths`.

## What Earnest adds

| | `getShortestPaths()` | `getPaths(earnestPaths)` | `checkMachine` |
|---|---|---|---|
| One shortest path to every state | yes | yes | `reached()` |
| State count, with finished or stopped at the limit | no | an error at the limit | `states`, `status` and `reason` |
| Rules judged in every reachable state | no | no | yes, with the shortest path to each broken one |
| Stuck states | no | no | yes, with their path |
| States where the context leaves its declared range | no | no | yes, with a `context` schema |
| Confirmation by `earnest verify` and TLC | no | no | yes, on the file `modelFileOf` writes |

## Limits

- **Path order.** `earnestPaths` gives paths in the order the search reached their states: breadth first, events in the order `events` lists them. When two paths to a state are equally short, the one found first is kept. XState's order and choice can differ, so a test must not rely on either.
- **`getSimplePaths` stays with XState.** `earnestPaths` replaces only the shortest paths.
- **The search runs in the test's process, in JavaScript.** It steps the machine's `transition`, not the `earnest` binary, so the speed figures of `earnest verify` do not apply.
- **TLA+ export needs a written model file.** `earnest verify` and `earnest tla` read model files and never run JavaScript. `modelFileOf` writes a machine out as one, when every context field has a declaration.
- **How states are counted.** `checkMachine` counts a state by its value, its context, its status and the states its history states remember, whatever event led there. A machine with a history state counts a state again once a history state remembers something. `createTestModel` by default also keys a state by the event that led to it, so the same machine can have more distinct states for `getPaths(earnestPaths)` than `states` in `checkMachine`. To compare counts, pass `getPaths` a `serializeState` that keys a state by its value and context, such as `(snapshot) => JSON.stringify({ value: snapshot.value, context: snapshot.context })`. A final state's output and child actors are not part of the state. An actor spawned into the context without an `id` gets a new one on each event, so give it an `id`.
- **The events are the ones you list.** `checkMachine` tries only the events `events` gives, with the payloads written there, because XState has no public way to list a machine's own events. List each payload to try.
- **Timers and child actors are not followed.** No timer fires and no invoked actor finishes, so a state left only by `after` or by `onDone` is reported stuck. Child actors are made but never run, and the machine has no parent. An error in a child, its `onDone` and `onError`, and events sent with `sendTo` to `self` are not followed, and `sendParent` throws. List an event sent to `self` in `events` to have it tried.
- **One actor system.** A child actor with a `systemId` that starts a second time, as when its state is entered again, makes `checkMachine` throw that the id already exists.
- **Keep guards and assigns pure.** The code that computes the next snapshot runs, often more than once for one event: guards, `assign`, `enqueueActions` callbacks, `invoke` `input` and the context of a child machine. Give it the same result for the same snapshot and event, and no side effects. No action of your own runs, except one made with `assign`.
- **The context must be plain data.** Plain objects with string keys, arrays, strings, numbers, booleans, `null`, `undefined`, and values with a `toJSON` (such as a `Date`) are read. A `Map`, a class instance or a function in the context is an error that names its type. A field set to `undefined` is the same as no field.
- **Errors are thrown.** An error thrown by a guard, an `assign`, the context factory or a rule is thrown by `checkMachine`, also one XState catches into a snapshot whose status is `error`.
- **Memory.** The check holds every state found with its whole snapshot, a few hundred bytes for a small context. A machine with millions of states can run out of memory before the default `maxStates`, so pass a lower one.

## Next

- [smlang](/integrations/smlang/): the same search under a Rust state machine library
- [TypeScript models](/reference/typescript/#searching-states-that-are-not-a-model): `search`, which both parts of this package build on
- [Output, JSON, and reports](/reference/output/): what `pass`, `fail`, `unfinished` and `stuck` mean for `earnest verify`
