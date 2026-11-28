import { explore, m } from "@earnestjs/model";
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { assign, setup } from "xstate";

const tally = setup({
  types: {
    context: {} as { count: number; note: string },
    events: {} as { type: "ADD" },
  },
}).createMachine({
  id: "tally",
  context: { count: 0, note: "" },
  on: {
    ADD: {
      actions: assign({
        count: ({ context }) => context.count + 1,
        note: ({ context }) => `${context.note}+`,
      }),
    },
  },
});

const bounded = {
  bounded: {
    says: "The count is never below zero",
    holds: (snapshot: { context: { count: number } }) => snapshot.context.count >= 0,
  },
};

test("a counter that grows forever stops at the first count out of range, as explore does", () => {
  const found = checkMachine(tally, {
    events: [{ type: "ADD" }],
    context: { count: m.int(0, 3) },
    rules: bounded,
  });
  expect({
    status: found.status,
    states: found.states,
    verdicts: found.rules.map(({ verdict }) => verdict),
    findings: found.findings.map(({ kind, message, path }) => ({
      kind,
      message,
      events: path.events.map(({ type }) => type),
      end: path.end.context.count,
    })),
    reason: found.reason,
    reached: [...found.reached()].map(({ end }) => end.context.count),
  }).toStrictEqual({
    status: "fail",
    states: 5,
    verdicts: ["unknown"],
    findings: [
      {
        kind: "out-of-range",
        message: "the context field `count` leaves the values it declares",
        events: ["ADD", "ADD", "ADD", "ADD"],
        end: 4,
      },
    ],
    reason: null,
    reached: [0, 1, 2, 3, 4],
  });

  const Tally = m.model("Tally", { count: m.int(0, 3).initial(0) });
  const add = m.action(Tally, "the clerk adds one", {
    when: (s) => s.count <= 3,
    changes: (s) => ({ count: s.count + 1 }),
  });
  const rule = m.rule(Tally, "The count is never below zero", (s) => s.count >= 0);
  const explored = explore(Tally, { exports: { add, bounded: rule } });
  expect({
    status: explored.status,
    states: explored.states,
    verdicts: explored.rules.map(({ verdict }) => verdict),
    kinds: explored.findings.map(({ kind }) => kind),
    steps: explored.findings[0]?.path.steps.length,
  }).toStrictEqual({
    status: found.status,
    states: found.states,
    verdicts: found.rules.map(({ verdict }) => verdict),
    kinds: found.findings.map(({ kind }) => kind),
    steps: found.findings[0]?.path.steps.length,
  });
});

test("without a context schema the same counter runs to the state limit", () => {
  const found = checkMachine(tally, { events: [{ type: "ADD" }], rules: bounded, maxStates: 10 });
  expect({ status: found.status, states: found.states, reason: found.reason }).toStrictEqual({
    status: "unfinished",
    states: 10,
    reason: "state limit of 10",
  });
});

test("a context field the schema does not name is not checked", () => {
  const found = checkMachine(tally, {
    events: [{ type: "ADD" }],
    context: { count: m.int(0, 9) },
    maxStates: 4,
  });
  expect({ findings: found.findings, reason: found.reason }).toStrictEqual({
    findings: [],
    reason: "state limit of 4",
  });
});

test("an initial context out of range is the out-of-range state, with an empty path", () => {
  const found = checkMachine(tally, { events: [{ type: "ADD" }], context: { count: m.int(1, 3) } });
  expect({
    states: found.states,
    events: found.findings[0]?.path.events,
  }).toStrictEqual({ states: 1, events: [] });
});

const till = setup({
  types: {
    context: {} as { paid: { coins: number; note: "cash" | "card" } | null },
    events: {} as { type: "PAY"; coins: number; note: "cash" | "card" } | { type: "CLEAR" },
  },
}).createMachine({
  id: "till",
  context: { paid: null },
  on: {
    PAY: {
      guard: ({ context }) => context.paid === null,
      actions: assign({ paid: ({ event }) => ({ coins: event.coins, note: event.note }) }),
    },
    CLEAR: { guard: ({ context }) => context.paid !== null, actions: assign({ paid: null }) },
  },
});

test("an event type is tried with every payload, the first field varying fastest", () => {
  const found = checkMachine(till, {
    events: { PAY: { coins: m.int(1, 2), note: m.enum(["cash", "card"]) }, CLEAR: {} },
  });
  expect(found.states).toBe(5);
  expect([...found.reached()].map(({ events }) => events)).toStrictEqual([
    [],
    [{ type: "PAY", coins: 1, note: "cash" }],
    [{ type: "PAY", coins: 2, note: "cash" }],
    [{ type: "PAY", coins: 1, note: "card" }],
    [{ type: "PAY", coins: 2, note: "card" }],
  ]);
});

test("a rule's path shows the payload of each event", () => {
  const found = checkMachine(till, {
    events: { PAY: { coins: m.int(1, 2), note: m.enum(["cash", "card"]) } },
    rules: {
      small: {
        says: "No card pays two coins",
        holds: (snapshot) =>
          snapshot.context.paid?.note !== "card" || snapshot.context.paid.coins < 2,
      },
    },
  });
  expect(found.rules[0]?.path?.events).toStrictEqual([{ type: "PAY", coins: 2, note: "card" }]);
});

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

const kinds = [
  ["boolean", m.boolean(), [false, true], [0, "true", undefined]],
  ["int", m.int(-1, 1), [-1, 0, 1], [2, 0.5, "0", undefined]],
  ["enum", m.enum(["a", "b"]), ["a", "b"], ["c", undefined]],
  ["oneOf of numbers", m.oneOf([2, 4]), [2, 4], [3, "2"]],
  ["oneOf of booleans", m.oneOf([true]), [true], [false]],
  ["optional", m.int(0, 1).optional(), [null, 0, 1], [2]],
  [
    "record",
    m.record({ on: m.boolean(), level: m.int(0, 1).optional() }),
    [
      { on: false, level: null },
      { on: false, level: 0 },
      { on: false, level: 1 },
      { on: true, level: null },
      { on: true, level: 0 },
      { on: true, level: 1 },
    ],
    [{ level: 0 }, { on: true, level: 2 }, { on: true, level: 0, more: 1 }, null],
  ],
] as const;

for (const [name, declared, inside] of kinds) {
  test(`a payload declared as ${name} is tried with each of its values, in order`, () => {
    const found = checkMachine(holding, {
      input: { held: inside[0] },
      events: { SET: { to: declared } },
      context: { held: declared },
    });
    expect({
      findings: found.findings,
      held: [...found.reached()].map(({ end }) => end.context.held),
    }).toStrictEqual({ findings: [], held: inside });
  });
}

for (const [name, declared, inside, outside] of kinds) {
  test(`a context field declared as ${name} is out of range at a value it does not declare`, () => {
    for (const to of outside) {
      const found = checkMachine(holding, {
        input: { held: inside[0] },
        events: [{ type: "SET", to }],
        context: { held: declared },
      });
      expect({
        states: found.states,
        findings: found.findings.map(({ kind, path }) => ({ kind, events: path.events })),
      }).toStrictEqual({
        states: 2,
        findings: [{ kind: "out-of-range", events: [{ type: "SET", to }] }],
      });
    }
  });
}

test("a context field that is missing is in range when its declaration is optional", () => {
  const found = checkMachine(holding, {
    input: { held: undefined },
    events: [],
    context: { held: m.boolean().optional() },
  });
  expect(found.findings.map(({ kind }) => kind)).toStrictEqual(["stuck"]);
});

const refused = [
  [m.set(m.boolean()), "a set declaration"],
  [m.map(m.boolean(), m.boolean()), "a map declaration"],
  [m.seq(m.boolean(), { max: 1 }), "a sequence declaration"],
  [m.queue(m.boolean(), { capacity: 1 }), "a queue declaration"],
  [m.lock(), "a lock declaration"],
  [m.lock(m.boolean()), "an owned lock declaration"],
  [m.timer(), "a timer declaration"],
  [m.request(), "a request declaration"],
  [m.retry({ max: 1 }), "a retry declaration"],
  [m.set(m.boolean()).optional(), "an optional declaration that holds a set"],
  [m.record({ on: m.boolean(), wait: m.timer() }), "a record declaration that holds a timer"],
] as const;

const refusal = (option: string, kind: string): string =>
  `checkMachine can't take ${option}, ${kind}: its values are objects of @earnestjs/model, not plain data an XState context or event holds; declare booleans, numbers, texts, optionals and records of those`;

test("a context declaration of library values is refused before the search, by name", () => {
  for (const [declared, kind] of refused) {
    expect(() =>
      checkMachine(holding, {
        input: { held: null },
        events: [],
        context: { held: declared },
        rules: {
          never: {
            says: "Never judged",
            holds: () => {
              throw new Error("the search ran");
            },
          },
        },
      }),
    ).toThrow(refusal("context.held", kind));
  }
});

test("a payload declaration of library values is refused before the search, by name", () => {
  for (const [declared, kind] of refused) {
    expect(() =>
      checkMachine(holding, { input: { held: null }, events: { SET: { to: declared } } }),
    ).toThrow(refusal("events.SET.to", kind));
  }
});

test("a payload field named type is refused: an event's type is its key in events", () => {
  const payload = { to: m.boolean(), type: m.boolean() };
  expect(() => checkMachine(holding, { input: { held: null }, events: { SET: payload } })).toThrow(
    "checkMachine can't take events.SET.type: an event's type is its key in events",
  );
});

test("inside a record, a field set to undefined is no field, as it is in the state", () => {
  const declared = m.record({ on: m.boolean() });
  for (const [start, to] of [
    [{ on: true }, { on: true, extra: undefined }],
    [{ on: true, extra: undefined }, { on: true }],
  ] as const) {
    const found = checkMachine(holding, {
      input: { held: start },
      events: [{ type: "SET", to }],
      context: { held: declared },
    });
    expect({ status: found.status, states: found.states }).toStrictEqual({
      status: "pass",
      states: 1,
    });
  }
});

test("an event type with too many payloads to list is refused by name", () => {
  const wide = m.int(1, 1000);
  expect(() =>
    checkMachine(holding, {
      input: { held: null },
      events: { SET: { to: m.record({ x: wide, y: wide, z: m.boolean() }) } },
    }),
  ).toThrow(
    "checkMachine can't list the payloads of events.SET: these fields have 2000000 combinations of values, too many to list; declare fewer values",
  );
});

test("a declaration from an @earnestjs/model without libraryKind is refused by name", () => {
  const old = Object.defineProperty(m.int(0, 1), "libraryKind", { value: undefined });
  expect(() =>
    checkMachine(holding, { input: { held: 0 }, events: [], context: { held: old } }),
  ).toThrow(
    "checkMachine can't take context.held: it was made with an @earnestjs/model older than the one @earnestjs/xstate uses; install the version it depends on",
  );
});
