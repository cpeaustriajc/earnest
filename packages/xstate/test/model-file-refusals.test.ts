import { m } from "@earnestjs/model";
import { MOST_TABLE_ROWS, modelFileOf } from "@earnestjs/xstate";
import { assign, setup } from "xstate";
import { expect, test } from "vitest";
import { openTurnstile } from "./machines/open-turnstile.ts";
import { visitorCount } from "./machines/visitor-count.ts";

const counter = setup({
  types: {
    context: {} as { count: number; label?: string },
    events: {} as { type: "UP" } | { type: "NAME" } | { type: "SET"; to: number },
  },
}).createMachine({
  context: { count: 0 },
  on: {
    UP: { actions: assign({ count: ({ context }) => context.count + 1 }) },
    NAME: { actions: assign({ label: "named" }) },
    SET: { actions: assign({ count: ({ event }) => event.to }) },
  },
});

test("a context field with no declaration is refused, by name, before any event is tried", () => {
  let tried = 0;
  expect(() =>
    modelFileOf(openTurnstile, {
      name: "OpenTurnstile",
      events: [{ type: "PAY" }],
      rules: {
        counted: {
          says: "Counts the rules judged",
          holds: () => {
            tried += 1;
            return true;
          },
        },
      },
    }),
  ).toThrow(
    "modelFileOf can't write the context field `paid` of the initial snapshot: it has no declaration in context; declare its values, such as context: { paid: m.int(0, 3) }",
  );
  expect(tried).toBe(0);
});

test("a table past the limit is refused from the declarations' counts, before any value is listed", () => {
  expect(MOST_TABLE_ROWS).toBe(100_000);
  expect(() =>
    modelFileOf(visitorCount, {
      name: "VisitorCount",
      events: { ENTER: { group: m.int(1, 2) } },
      context: { visitors: m.int(0, 2_000_000) },
    }),
  ).toThrow(
    "modelFileOf can't write a table of 4000002 rows or more, past its limit of 100000: 1 × 2000001 × 2, the states of the machine found, the combinations of context values, and the events with each payload and the rules; declare fewer values",
  );
});

test("a table that grows past the limit as states are found is refused with the count of states", () => {
  const chain = setup({
    types: { context: {} as { count: number }, events: {} as { type: "NEXT" } },
  }).createMachine({
    initial: "a",
    context: { count: 0 },
    states: { a: { on: { NEXT: "b" } }, b: { on: { NEXT: "c" } }, c: {} },
  });
  expect(() =>
    modelFileOf(chain, {
      name: "Chain",
      events: [{ type: "NEXT" }],
      context: { count: m.int(0, 33_999) },
    }),
  ).toThrow(
    "modelFileOf can't write a table of 102000 rows or more, past its limit of 100000: 3 × 34000 × 1, the states of the machine found, the combinations of context values, and the events with each payload and the rules; declare fewer values",
  );
});

test("events given as a function are refused", () => {
  expect(() =>
    modelFileOf(openTurnstile, {
      name: "OpenTurnstile",
      events: () => [{ type: "PAY" }],
      context: { paid: m.boolean() },
    }),
  ).toThrow(
    "modelFileOf can't take events as a function of the snapshot: a model file gives each event once, for every state; write the events as a record of event types, each with a declaration for each payload field, such as { ENTER: { group: m.int(1, 2) } }, or list the events",
  );
});

test("a listed event with a payload is refused, with the record form as the way out", () => {
  expect(() =>
    modelFileOf(visitorCount, {
      name: "VisitorCount",
      events: [{ type: "ENTER", group: 1 }],
      context: { visitors: m.int(0, 3) },
    }),
  ).toThrow(
    "modelFileOf can't take events[0], ENTER with the payload field group: a model file declares a payload's values; write the events as a record of event types",
  );
});

test("a next context the declaration's type can't hold is refused with the event and the field", () => {
  expect(() =>
    modelFileOf(counter, {
      name: "Counter",
      events: [{ type: "NAME" }],
      context: { count: m.int(0, 1), label: m.boolean().optional() as never },
    }),
  ).toThrow(
    'modelFileOf can\'t write the context field `label` after the event {"type":"NAME"}: it holds the text "named", which a field declared m.boolean().optional() can\'t hold',
  );
});

test("a field a transition adds without a declaration is refused with the event", () => {
  expect(() =>
    modelFileOf(counter, {
      name: "Counter",
      events: [{ type: "NAME" }],
      context: { count: m.int(0, 1) },
    }),
  ).toThrow(
    'modelFileOf can\'t write the context field `label` after the event {"type":"NAME"}: it has no declaration in context',
  );
});

test("an initial context outside its declaration is refused", () => {
  expect(() =>
    modelFileOf(counter, {
      name: "Counter",
      events: [{ type: "UP" }],
      context: { count: m.int(1, 2) },
    }),
  ).toThrow(
    "modelFileOf can't write the initial context field `count`: it is 0, outside m.int(1, 2), and a model starts inside its declarations",
  );
});

test("a model name that is not an identifier is refused", () => {
  expect(() =>
    modelFileOf(counter, { name: "my counter", events: [{ type: "UP" }], context: {} }),
  ).toThrow(
    'modelFileOf can\'t take name: it is the text "my counter", not an identifier that can name the model, such as Turnstile',
  );
});

test("a field name that is not an identifier is refused", () => {
  const spaced = setup({
    types: { context: {} as { "two words": number }, events: {} as { type: "GO" } },
  }).createMachine({ context: { "two words": 0 } });
  expect(() =>
    modelFileOf(spaced, {
      name: "Spaced",
      events: [{ type: "GO" }],
      context: { "two words": m.int(0, 1) },
    }),
  ).toThrow(
    "modelFileOf can't take context.two words: a model file reads a field by its name, so the name must be an identifier, such as two_words",
  );
});

test("a declaration of library values is refused as checkMachine refuses it, in modelFileOf's words", () => {
  expect(() =>
    modelFileOf(visitorCount, {
      name: "VisitorCount",
      events: { ENTER: { group: m.int(1, 2) } },
      context: { visitors: m.set(m.int(0, 1)) as never },
    }),
  ).toThrow("modelFileOf can't take context.visitors, a set declaration");
});
