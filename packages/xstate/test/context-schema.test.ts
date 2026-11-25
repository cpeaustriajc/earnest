import { m } from "@earnestjs/model";
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { assign, setup } from "xstate";
import { dial, toggle } from "./machines/modes.ts";

const holding = setup({
  types: {
    input: {} as { held: unknown },
    context: {} as { held?: unknown },
    events: {} as { type: "SET"; to: unknown },
  },
}).createMachine({
  id: "holding",
  context: ({ input }) => ({ held: input.held }),
  on: { SET: { actions: assign({ held: ({ event }) => event.to }) } },
});

const fromPlainJavaScript = (options: object): unknown =>
  Reflect.apply(checkMachine, undefined, [
    holding,
    {
      input: { held: null },
      rules: {
        never: {
          says: "Never judged",
          holds: () => {
            throw new Error("the search ran");
          },
        },
      },
      ...options,
    },
  ]);

const notDeclaration = (option: string, found: string): string =>
  `checkMachine can't take ${option}: it is ${found}, not a declaration; write one with m from @earnestjs/model, such as m.int(0, 3), or m.record({ ... }) for an object`;

const notPayload = (type: string, found: string): string =>
  `checkMachine can't take events.${type}: it is ${found}, not a record of payload fields; give a declaration written with m from @earnestjs/model for each field, such as { group: m.int(1, 2) }, or {} for none`;

const entries = [
  [1, "the number 1"],
  ["0..3", 'the text "0..3"'],
  [null, "null"],
  [m.int, "a function"],
  [m.boolean, "a function"],
  [{}, "a plain object"],
  [[m.boolean()], "an array"],
  [new Date(0), "a Date"],
] as const;

test("a context entry that is not a declaration is refused before the search, saying what it is", () => {
  for (const [entry, found] of entries) {
    const run = () => fromPlainJavaScript({ events: [], context: { held: entry } });
    expect(run).toThrow(TypeError);
    expect(run).toThrow(notDeclaration("context.held", found));
  }
});

test("a payload entry that is not a declaration is refused before the search, saying what it is", () => {
  for (const [entry, found] of entries) {
    expect(() => fromPlainJavaScript({ events: { SET: { to: entry } } })).toThrow(
      notDeclaration("events.SET.to", found),
    );
  }
});

test("an event type whose payload is not a record of fields is refused, a whole declaration too", () => {
  for (const [payload, found] of [
    [null, "null"],
    [m.record({ to: m.boolean() }), "a record declaration"],
    [m.boolean(), "a boolean declaration"],
    [[], "an array"],
    [2, "the number 2"],
  ] as const) {
    const run = () => fromPlainJavaScript({ events: { SET: payload } });
    expect(run).toThrow(TypeError);
    expect(run).toThrow(notPayload("SET", found));
  }
});

test("a whole declaration given as the context schema is refused", () => {
  expect(() =>
    fromPlainJavaScript({ events: [], context: m.record({ held: m.boolean() }) }),
  ).toThrow(
    "checkMachine can't take context: it is a record declaration, not a record of context fields; give a declaration written with m from @earnestjs/model for each field, such as { visitors: m.int(0, 3) }",
  );
});

test("a declaration from another copy of @earnestjs/model without libraryKind is refused as older", () => {
  const older = { kind: "number", count: 2, has: () => true, values: () => [0, 1] };
  expect(() => fromPlainJavaScript({ events: [], context: { held: older } })).toThrow(
    "checkMachine can't take context.held: it was made with an @earnestjs/model older than the one @earnestjs/xstate uses; install the version it depends on",
  );
});

test("an optional context field set to undefined is in range, as a missing one is", () => {
  const found = checkMachine(holding, {
    input: { held: 0 },
    events: [{ type: "SET", to: undefined }],
    context: { held: m.int(0, 1).optional() },
  });
  expect({ status: found.status, states: found.states, findings: found.findings }).toStrictEqual({
    status: "pass",
    states: 2,
    findings: [],
  });
});

test("a field that one member of a union context lacks is declared optional", () => {
  const found = checkMachine(toggle, {
    events: [{ type: "ON" }],
    context: { level: m.int(0, 3).optional() },
  });
  expect({
    states: found.states,
    findings: found.findings.map(({ kind, path }) => ({ kind, events: path.events })),
  }).toStrictEqual({
    states: 2,
    findings: [{ kind: "out-of-range", events: [{ type: "ON" }] }],
  });
});

test("a field that one member of a union context lacks, declared without optional, is out of range there", () => {
  const found = checkMachine(toggle, {
    events: [{ type: "ON" }],
    context: { level: m.int(0, 9) },
  });
  expect({
    states: found.states,
    findings: found.findings.map(({ kind, path }) => ({ kind, events: path.events })),
  }).toStrictEqual({ states: 1, findings: [{ kind: "out-of-range", events: [] }] });
});

test("an optional context field is in range while it is missing or undefined", () => {
  const found = checkMachine(dial, {
    events: [{ type: "TURN" }],
    context: { level: m.int(0, 2).optional(), note: m.enum(["low"]).optional() },
  });
  expect({
    states: found.states,
    levels: [...found.reached()].map(({ end }) => end.context.level),
    findings: found.findings.map(({ kind }) => kind),
  }).toStrictEqual({ states: 4, levels: [undefined, 1, 2, 3], findings: ["out-of-range"] });
});

test("a value checked against a record declaration must be a plain object at every level", () => {
  for (const [declared, held] of [
    [m.record({}), new Date(0)],
    [m.record({ at: m.record({}) }), { at: new Date(0) }],
    [m.record({ at: m.record({}).optional() }), { at: new Date(0) }],
  ] as const) {
    const found = checkMachine(holding, {
      input: { held },
      events: [],
      context: { held: declared },
    });
    expect(found.findings.map(({ kind }) => kind)).toStrictEqual(["out-of-range"]);
  }
});

test("a plain object, also one with no prototype, still passes a record declaration", () => {
  for (const held of [{ at: {} }, { at: Object.create(null) }]) {
    const found = checkMachine(holding, {
      input: { held },
      events: [],
      context: { held: m.record({ at: m.record({}) }) },
    });
    expect(found.findings.map(({ kind }) => kind)).toStrictEqual(["stuck"]);
  }
});

test("an events option that is neither a list, a function nor a record of event types is refused", () => {
  for (const [events, found] of [
    [m.record({ SET: m.boolean() }), "a record declaration"],
    [null, "null"],
    ["SET", 'the text "SET"'],
  ] as const) {
    expect(() => fromPlainJavaScript({ events })).toThrow(
      `checkMachine can't take events: it is ${found}, not a list of events, a function that gives one, or a record of event types, such as { ENTER: { group: m.int(1, 2) } }`,
    );
  }
});

test("an optional context field named like an Object member is missing when the context lacks it", () => {
  const found = Reflect.apply(checkMachine, undefined, [
    holding,
    { input: { held: null }, events: [], context: { constructor: m.int(0, 3).optional() } },
  ]);
  expect(found).toMatchObject({ findings: [{ kind: "stuck" }] });
});
