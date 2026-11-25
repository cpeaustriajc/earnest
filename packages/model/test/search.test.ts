import { expect, test } from "vitest";
import { type SearchMove, type Trail, explore, m, search } from "@earnestjs/model";

interface Gate {
  readonly gate: "Locked" | "Open";
  readonly paid: boolean;
}

type Push = "pay" | "push" | "enter";

const gateMoves = (s: Gate): SearchMove<Gate, Push>[] => {
  const moves: SearchMove<Gate, Push>[] = [];
  if (!s.paid && s.gate === "Locked") {
    moves.push({ move: "pay", next: { ...s, paid: true } });
  }
  if (s.gate === "Locked") {
    moves.push({ move: "push", next: { ...s, gate: "Open" } });
  }
  if (s.gate === "Open") {
    moves.push({ move: "enter", next: { gate: "Locked", paid: false } });
  }
  return moves;
};

const gate = {
  starts: [{ gate: "Locked", paid: false }] satisfies Gate[],
  key: (s: Gate) => `${s.gate};${s.paid}`,
  moves: gateMoves,
  rules: [
    {
      id: "locked",
      says: "The gate stays locked until the visitor pays",
      holds: (s: Gate) => s.gate === "Locked" || s.paid,
    },
  ],
};

const counter = (top: number) => ({
  starts: [0],
  key: String,
  moves: (n: number) => (n < top ? [{ move: "up", next: n + 1 }] : []),
});

const LOCKED: Gate = { gate: "Locked", paid: false };

test("a turnstile of plain objects has 4 states and breaks its rule after one push", () => {
  const found = search(gate);
  expect(found).toStrictEqual({
    status: "fail",
    states: 4,
    rules: [
      {
        id: "locked",
        says: "The gate stays locked until the visitor pays",
        verdict: "broken",
        path: { start: LOCKED, steps: [{ move: "push", state: { gate: "Open", paid: false } }] },
      },
    ],
    findings: [],
    reason: null,
    reached: found.reached,
  });
});

test("the plain turnstile agrees with explore on the model it mirrors", () => {
  const Turnstile = m.model("Turnstile", {
    gate: m.enum(["Locked", "Open"]).initial("Locked"),
    paid: m.boolean().initial(false),
  });
  const pay = m.action(Turnstile, "the visitor pays", {
    when: (s) => !s.paid && s.gate === "Locked",
    changes: () => ({ paid: true }),
  });
  const push = m.action(Turnstile, "the visitor pushes", {
    when: (s) => s.gate === "Locked",
    changes: () => ({ gate: "Open" }),
  });
  const enter = m.action(Turnstile, "the visitor walks through", {
    when: (s) => s.gate === "Open",
    changes: () => ({ gate: "Locked", paid: false }),
  });
  const locked = m.rule(
    Turnstile,
    "The gate stays locked until the visitor pays",
    (s) => s.gate === "Locked" || s.paid,
  );
  const modelled = explore(Turnstile, { exports: { pay, push, enter, locked } });
  const plain = search(gate);
  expect(plain.states).toBe(modelled.states);
  expect(plain.status).toBe(modelled.status);
  expect(plain.rules.map((rule) => [rule.id, rule.verdict])).toStrictEqual(
    modelled.rules.map((rule) => [rule.id, rule.verdict]),
  );
  expect(plain.rules[0]?.path?.steps.map((step) => step.move)).toStrictEqual(
    modelled.rules[0]?.path?.steps.map((step) => step.action),
  );
});

test("reached gives the shortest trail to every state, in the order they were reached", () => {
  const trails: Trail<Gate, Push>[] = [...search(gate).reached()];
  expect(trails).toStrictEqual([
    { start: LOCKED, steps: [] },
    { start: LOCKED, steps: [{ move: "pay", state: { gate: "Locked", paid: true } }] },
    { start: LOCKED, steps: [{ move: "push", state: { gate: "Open", paid: false } }] },
    {
      start: LOCKED,
      steps: [
        { move: "pay", state: { gate: "Locked", paid: true } },
        { move: "push", state: { gate: "Open", paid: true } },
      ],
    },
  ]);
});

test("reached can be walked more than once", () => {
  const found = search(counter(2));
  expect([...found.reached()]).toStrictEqual([...found.reached()]);
});

test("a state with no move that is not an end is stuck", () => {
  expect(search(counter(3)).findings).toStrictEqual([
    {
      kind: "stuck",
      message: "no move can happen here, and the caller does not mark it as an end",
      path: {
        start: 0,
        steps: [
          { move: "up", state: 1 },
          { move: "up", state: 2 },
          { move: "up", state: 3 },
        ],
      },
    },
  ]);
});

test("a state with no move that isEnd marks is not stuck, and isEnd is asked only there", () => {
  const asked: number[] = [];
  const found = search({
    ...counter(3),
    isEnd: (n) => {
      asked.push(n);
      return n === 3;
    },
  });
  expect(found.status).toBe("pass");
  expect(found.findings).toStrictEqual([]);
  expect(asked).toStrictEqual([3]);
});

test("a state limit leaves the search unfinished with the reason, and no rule holds", () => {
  const found = search({
    ...counter(9),
    maxStates: 2,
    rules: [{ id: "small", says: "n stays below ten", holds: (n) => n < 10 }],
  });
  expect(found.status).toBe("unfinished");
  expect(found.reason).toBe("state limit of 2");
  expect(found.states).toBe(2);
  expect(found.rules.map((rule) => rule.verdict)).toStrictEqual(["unknown"]);
});

test("a state counted at the limit has its rules judged, though no move is asked of it", () => {
  const movesAsked: number[] = [];
  const found = search({
    starts: [0],
    key: String,
    moves: (n: number) => {
      movesAsked.push(n);
      return [
        { move: "one", next: n + 1 },
        { move: "two", next: n + 2 },
      ];
    },
    maxStates: 3,
    rules: [{ id: "notTwo", says: "n is never two", holds: (n) => n !== 2 }],
  });
  expect(found.status).toBe("fail");
  expect(found.reason).toBe("state limit of 3");
  expect(found.rules[0]?.path).toStrictEqual({ start: 0, steps: [{ move: "two", state: 2 }] });
  expect(movesAsked).toStrictEqual([0, 1]);
});

test("states with the same key are one state", () => {
  const found = search({
    starts: [0, 4, 8],
    key: (n: number) => String(n % 4),
    moves: () => [],
    isEnd: () => true,
  });
  expect(found.states).toBe(1);
  expect(found.status).toBe("pass");
});

test("a state out of range is counted, not judged and not searched from", () => {
  const judged: number[] = [];
  const found = search({
    ...counter(5),
    isEnd: () => true,
    rules: [
      {
        id: "seen",
        says: "every state is judged",
        holds: (n) => {
          judged.push(n);
          return true;
        },
      },
    ],
    outside: (n) => (n === 2 ? "n passes one" : null),
  });
  expect(found.states).toBe(3);
  expect(judged).toStrictEqual([0, 1]);
  expect(found.status).toBe("fail");
  expect(found.rules.map((rule) => rule.verdict)).toStrictEqual(["unknown"]);
  expect(found.findings).toStrictEqual([
    {
      kind: "out-of-range",
      message: "n passes one",
      path: {
        start: 0,
        steps: [
          { move: "up", state: 1 },
          { move: "up", state: 2 },
        ],
      },
    },
  ]);
});

test("a search that finds nothing wrong passes and every rule holds", () => {
  const found = search({
    ...counter(3),
    isEnd: (n) => n === 3,
    rules: [{ id: "small", says: "n stays below four", holds: (n) => n < 4 }],
  });
  expect(found.status).toBe("pass");
  expect(found.states).toBe(4);
  expect(found.rules.map((rule) => rule.verdict)).toStrictEqual(["holds"]);
});

test("an error thrown by the caller's functions is thrown by search", () => {
  const broken = new Error("the machine broke");
  expect(() =>
    search({
      ...counter(3),
      moves: () => {
        throw broken;
      },
    }),
  ).toThrow(broken);
});

test("a maxStates that is not a whole number is refused", () => {
  expect(() => search({ ...counter(3), maxStates: 1.5 })).toThrow(
    "maxStates must be a whole number that is not negative, not 1.5",
  );
});
