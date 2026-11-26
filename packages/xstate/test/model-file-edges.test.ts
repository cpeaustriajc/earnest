import { m } from "@earnestjs/model";
import { checkMachine, modelFileOf } from "@earnestjs/xstate";
import { type AnyMachineSnapshot, assign, setup } from "xstate";
import { beforeAll, describe, expect, test } from "vitest";
import {
  NO_CARGO,
  SLOW,
  buildEarnest,
  eventsOf,
  hasCargo,
  summaryOfCheck,
  summaryOfVerify,
  verify,
} from "./earnest.ts";
import { quietScope } from "../src/machine.ts";
import { dimmer } from "./machines/dimmer.ts";

const LAST = 3;

const fragile = (throwsAt: number) =>
  setup({
    types: { context: {} as { n: number }, events: {} as { type: "INC" } },
    guards: {
      below: ({ context }) => {
        if (context.n === throwsAt) {
          throw new Error(`the guard can't judge ${throwsAt}`);
        }
        return context.n < 2;
      },
    },
  }).createMachine({
    context: { n: 0 },
    on: { INC: { guard: "below", actions: assign({ n: ({ context }) => context.n + 1 }) } },
  });

const booking = setup({
  types: {
    context: {} as { seat: { row: number; aisle: boolean } | null; note?: "late" },
    events: {} as
      | { type: "BOOK"; seat: { row: number; aisle: boolean } }
      | { type: "CANCEL" }
      | { type: "user.mark" },
  },
}).createMachine({
  id: "booking",
  initial: "open",
  context: { seat: null },
  states: {
    open: {
      on: {
        BOOK: {
          guard: ({ context }) => context.seat === null,
          actions: assign({ seat: ({ event }) => ({ ...event.seat }) }),
        },
        CANCEL: {
          guard: ({ context }) => context.seat !== null,
          actions: assign({ seat: null }),
        },
        "user.mark": {
          guard: ({ context }) => context.seat?.aisle === true && context.note === undefined,
          target: "closed",
          actions: assign({ note: "late" as const }),
        },
      },
    },
    closed: { type: "final" },
  },
});

const bookingOptions = {
  name: "Booking",
  events: {
    BOOK: { seat: m.record({ row: m.int(1, 2), aisle: m.boolean() }) },
    CANCEL: {},
    "user.mark": {},
  },
  context: {
    seat: m.record({ row: m.int(1, 2), aisle: m.boolean() }).optional(),
    note: m.enum(["late"]).optional(),
  },
  rules: {
    "seat.kept": {
      says: "A closed booking has a seat",
      holds: (snapshot: AnyMachineSnapshot) =>
        !snapshot.matches("closed") || snapshot.context.seat !== null,
    },
  },
} as const;

const switcher = setup({
  types: {
    context: {} as { type: "a" | "b" },
    events: {} as { type: "SET"; s: "a" | "b" | "c" } | { type: "FLIP"; s: "a" | "b" },
  },
}).createMachine({
  context: { type: "a" },
  on: {
    SET: {
      guard: ({ event }) => event.s !== "c",
      actions: assign({ type: ({ event }) => (event.s === "c" ? "a" : event.s) }),
    },
    FLIP: {
      guard: ({ context, event }) => context.type !== event.s,
      actions: assign({ type: ({ event }) => event.s }),
    },
  },
});

const switcherOptions = {
  name: "Switcher",
  events: { SET: { s: m.enum(["a", "b", "c"]) }, FLIP: { s: m.enum(["a", "b"]) } },
  context: { type: m.enum(["a", "b"]) },
};

test("a field may be named type, and a payload field s is renamed where it is a parameter", () => {
  const { text } = modelFileOf(switcher, switcherOptions);
  expect(text).toContain('  type: m.enum(["a", "b"]).initial("a"),');
  expect(text).toContain("s_: m.Infer<typeof SET_s>");
  expect(text).toContain("changes: (_, input) => ({ type: input.s }),");
});

test("a payload is copied into a field only where every value it declares fits the field", () => {
  const { text } = modelFileOf(switcher, switcherOptions);
  const set = text.slice(text.indexOf("export const SET"), text.indexOf("export const FLIP"));
  expect(set).not.toContain("input.s })");
  const flip = text.slice(text.indexOf("export const FLIP"));
  expect(flip).toContain("({ type: input.s })");
});

const lampLeaving = (dim: null | undefined) =>
  setup({
    types: {
      context: {} as { flag: boolean | null; spot: { lit: boolean; dim: boolean | null } },
      events: {} as { type: "SET" } | { type: "CLEAR" },
    },
  }).createMachine({
    context: { flag: null, spot: { lit: false, dim: null } },
    on: {
      SET: {
        guard: ({ context }) => context.flag !== false,
        actions: assign({ flag: false, spot: () => ({ lit: true, dim: dim as null }) }),
      },
      CLEAR: { guard: ({ context }) => context.flag === false, actions: assign({ flag: null }) },
    },
  });

const lampOptions = {
  name: "Lamp",
  events: [{ type: "SET" as const }, { type: "CLEAR" as const }],
  context: {
    flag: m.boolean().optional(),
    spot: m.record({ lit: m.boolean(), dim: m.boolean().optional() }),
  },
};

test("an optional boolean is tested against true and false, and a record field left undefined is written null", () => {
  const { text } = modelFileOf(lampLeaving(undefined), lampOptions);
  expect(text).toContain("s.flag !== false");
  expect(text).toContain("spot: { lit: true, dim: null }");
});

test("a guard that throws where the machine never goes is listed and changes nothing", () => {
  const written = modelFileOf(fragile(LAST), {
    name: "Fragile",
    events: [{ type: "INC" }],
    context: { n: m.int(0, LAST) },
  });
  expect(written.threw.map(({ part, id, state, context }) => [part, id, state, context])).toEqual([
    ["transition", "INC", "(machine)", { n: LAST }],
  ]);
  expect(written.text).toContain("threw: m.int(0, 0).initial(0),");
});

test("a rule that throws where the machine never goes is listed and is false there", () => {
  const written = modelFileOf(fragile(-1), {
    name: "Fragile",
    events: [{ type: "INC" }],
    context: { n: m.int(0, LAST) },
    rules: {
      small: {
        says: "The count is small",
        holds: ({ context }) => {
          if (context.n === LAST) {
            throw new Error("too big to judge");
          }
          return true;
        },
      },
    },
  });
  expect(written.threw.map(({ part, id, context }) => [part, id, context])).toEqual([
    ["rule", "small", { n: LAST }],
  ]);
  expect(written.text).toContain('m.rule(Fragile, "The count is small", (s) => s.n !== 3)');
});

test("an event type and a rule id that are not names are renamed, keeping the originals", () => {
  const written = modelFileOf(booking, bookingOptions);
  expect(written.actions).toStrictEqual({ BOOK: "BOOK", CANCEL: "CANCEL", user_mark: "user.mark" });
  expect(written.rules).toStrictEqual({ seat_kept: "seat.kept" });
  expect(written.text).toContain('export const user_mark = m.action(Booking, "user.mark", {');
  expect(written.text).toContain("export const seat_kept = m.rule(");
});

test("a snapshot resolved from a reached one's parts steps exactly as the reached one does", () => {
  const events = [{ type: "UP" }, { type: "OFF" }, { type: "ON" }, { type: "TICK" }] as const;
  const scope = quietScope();
  const parts = (snapshot: AnyMachineSnapshot) => ({
    value: snapshot.value,
    context: snapshot.context,
    status: snapshot.status,
    history: Object.fromEntries(
      Object.entries(snapshot.historyValue).map(([node, held]) => [node, held.map(({ id }) => id)]),
    ),
  });
  const reached = [...checkMachine(dimmer, { events: [...events] }).reached()].map(
    ({ end }) => end,
  );
  expect(reached).toHaveLength(11);
  for (const snapshot of reached) {
    const resolved = dimmer.resolveState({
      value: snapshot.value,
      context: { ...snapshot.context },
      historyValue: snapshot.historyValue,
      status: snapshot.status,
    });
    expect(parts(resolved)).toStrictEqual(parts(snapshot));
    for (const event of events) {
      expect(resolved.can(event)).toBe(snapshot.can(event));
      expect(parts(dimmer.transition(resolved, event, scope))).toStrictEqual(
        parts(dimmer.transition(snapshot, event, scope)),
      );
    }
  }
});

describe.skipIf(!hasCargo)(`earnest verify on edge cases (${NO_CARGO})`, () => {
  beforeAll(() => {
    buildEarnest();
  }, SLOW);

  test("a guard that throws where the machine never goes gives checkMachine's numbers", () => {
    const options = {
      name: "Fragile",
      events: [{ type: "INC" as const }],
      context: { n: m.int(0, LAST) },
    };
    const written = modelFileOf(fragile(LAST), options);
    expect(summaryOfVerify(written, verify(written.text))).toStrictEqual(
      summaryOfCheck(checkMachine(fragile(LAST), options)),
    );
  });

  test("a guard that throws where the machine goes reads as out of range on the field threw", () => {
    const options = {
      name: "Fragile",
      events: [{ type: "INC" as const }],
      context: { n: m.int(0, LAST) },
    };
    expect(() => checkMachine(fragile(1), options)).toThrow("the guard can't judge 1");
    const written = modelFileOf(fragile(1), options);
    const verified = verify(written.text);
    expect(
      verified.findings.map(({ kind, path }) => [kind, eventsOf(written, path)]),
    ).toStrictEqual([["out-of-range", [{ type: "INC" }, { type: "INC" }]]]);
  });

  test("a field named type and payloads copied into it give checkMachine's numbers", () => {
    const written = modelFileOf(switcher, switcherOptions);
    expect(summaryOfVerify(written, verify(written.text))).toStrictEqual(
      summaryOfCheck(checkMachine(switcher, switcherOptions)),
    );
  });

  test("an optional boolean, also inside a record, gives checkMachine's numbers", () => {
    const written = modelFileOf(lampLeaving(null), lampOptions);
    expect(summaryOfVerify(written, verify(written.text))).toStrictEqual(
      summaryOfCheck(checkMachine(lampLeaving(null), lampOptions)),
    );
  });

  test("records, optionals and record payloads give checkMachine's numbers", () => {
    const written = modelFileOf(booking, bookingOptions);
    const verified = verify(written.text);
    const found = checkMachine(booking, bookingOptions);
    expect(summaryOfVerify(written, verified)).toStrictEqual(summaryOfCheck(found));
    expect(found.states).toBe(7);
  });
});
