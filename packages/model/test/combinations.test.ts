import { MOST_VALUES_LISTED, combinationsOf, explore, m } from "@earnestjs/model";
import { expect, test } from "vitest";

test("combinationsOf varies the first field fastest", () => {
  expect(combinationsOf({ coins: m.int(1, 2), kind: m.enum(["a", "b"]) })).toStrictEqual([
    { coins: 1, kind: "a" },
    { coins: 2, kind: "a" },
    { coins: 1, kind: "b" },
    { coins: 2, kind: "b" },
  ]);
});

test("combinationsOf refuses more combinations than it lists, before listing any", () => {
  const wide = m.int(1, 1000);
  expect(() => combinationsOf({ x: wide, y: wide, z: m.int(1, 2) })).toThrow(
    "these fields have 2000000 combinations of values, too many to list; declare fewer values",
  );
  expect(combinationsOf({ x: wide, y: wide })).toHaveLength(MOST_VALUES_LISTED);
});

test("combinationsOf of no fields is one empty row", () => {
  expect(combinationsOf({})).toStrictEqual([{}]);
});

test("combinationsOf gives the inputs in the order explore tries them", () => {
  const Till = m.model("Till", {
    taken: m.boolean().initial(false),
    last: m.int(0, 40).initial(0),
  });
  const inputs = { coins: m.int(1, 2), times: m.int(1, 3) };
  const take = m.action(Till, "the till takes coins", {
    input: inputs,
    when: (s) => !s.taken,
    changes: (_, { coins, times }) => ({ taken: true, last: coins + 10 * times }),
  });
  const twelveOrTwentyOne = m.rule(
    Till,
    "The till never holds 12 or 21",
    (s) => s.last !== 12 && s.last !== 21,
  );
  const found = explore(Till, { exports: { take, twelveOrTwentyOne } });
  const firstInput = Object.fromEntries(
    (found.rules[0]?.path?.steps[0]?.input ?? []).map(({ name, value }) => [name, value]),
  );
  expect(firstInput).toStrictEqual(combinationsOf(inputs)[1]);
  expect(firstInput).toStrictEqual({ coins: 2, times: 1 });
});

test("libraryKind is null for declarations of plain values", () => {
  for (const declared of [
    m.boolean(),
    m.int(0, 3),
    m.enum(["a", "b"]),
    m.oneOf([1, 2]),
    m.int(0, 3).optional(),
    m.record({ on: m.boolean(), level: m.int(0, 2).optional() }),
    m.record({}),
  ]) {
    expect(declared.libraryKind).toBeNull();
  }
});

test("libraryKind names the first part that holds library values", () => {
  expect(
    [
      m.set(m.boolean()),
      m.map(m.boolean(), m.int(0, 1)),
      m.seq(m.boolean(), { max: 1 }),
      m.queue(m.boolean(), { capacity: 1 }),
      m.lock(),
      m.lock(m.enum(["a"])),
      m.timer(),
      m.request(),
      m.retry({ max: 1 }),
      m.set(m.boolean()).optional(),
      m.record({ on: m.boolean(), tags: m.set(m.boolean()), wait: m.timer() }),
    ].map((declared) => declared.libraryKind),
  ).toStrictEqual([
    "set",
    "map",
    "sequence",
    "queue",
    "lock",
    "owned lock",
    "timer",
    "request",
    "retry",
    "set",
    "set",
  ]);
});
