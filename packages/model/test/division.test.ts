import { expect, test } from "vitest";
import { type Exploration, explore, m, replay } from "@earnestjs/model";

const FAULT = "takes a division or a remainder by zero in this state";
const STUCK = "no action can happen here, and the model does not mark it as an end";

const messages = (found: Exploration): string[] => found.findings.map((finding) => finding.message);

test("m.div is the whole number of times the divisor goes in, as Rust's / on integers", () => {
  expect([m.div(7, 2), m.div(0, 5), m.div(6, 3), m.div(2, 9), m.div(9, 1)]).toStrictEqual([
    3, 0, 2, 0, 9,
  ]);
});

test("m.div is exact up to the largest safe integer", () => {
  expect(m.div(Number.MAX_SAFE_INTEGER, 2)).toBe(4_503_599_627_370_495);
  expect(m.div(Number.MAX_SAFE_INTEGER, 3)).toBe(3_002_399_751_580_330);
  expect(m.div(Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER)).toBe(1);
  expect(m.div(Number.MAX_SAFE_INTEGER - 1, Number.MAX_SAFE_INTEGER)).toBe(0);
});

test("m.div of a negative number, or by a divisor below 1, truncates, and by zero throws the division fault", () => {
  expect([m.div(-1, 3), m.div(5, -2), m.div(-4, -2)]).toStrictEqual([0, -2, 2]);
  expect(() => m.div(5, 0)).toThrow(/by zero/);
});

test("a division by zero in a change is a fault, as the checker reports it", () => {
  const Split = m.model("Split", { n: m.int(0, 3).initial(3), q: m.int(0, 3).initial(0) });
  const cut = m.action(Split, "the number is cut", {
    input: { by: m.int(0, 2) },
    changes: (s, { by }) => ({ q: m.div(s.n, by) }),
  });
  const found = explore(Split, { exports: { cut } });
  expect(found.states).toBe(3);
  expect(found.status).toBe("fail");
  expect(messages(found)).toStrictEqual([`the change of the action \`cut\` ${FAULT}`]);
});

test("a division of a negative number in a rule, an end and a when has a value, not a fault", () => {
  const Rule = m.model("Rule", { n: m.int(-1, 1).initial(-1) });
  const rule = m.rule(Rule, "Rule", (s) => m.div(s.n, 1) !== 99);
  const always = m.end(Rule, "Always", () => true);
  expect(messages(explore(Rule, { exports: { rule, always } }))).toStrictEqual([]);
  const When = m.model("When", { n: m.int(-1, 1).initial(-1) });
  const inWhen = m.action(When, "when", { when: (s) => m.div(s.n, 1) === 0, changes: () => ({}) });
  expect(messages(explore(When, { exports: { inWhen } }))).toStrictEqual([STUCK]);
  const Ended = m.model("Ended", { n: m.int(-1, 1).initial(-1) });
  const over = m.end(Ended, "Over", (s) => m.div(s.n, 1) === 0);
  expect(messages(explore(Ended, { exports: { over } }))).toStrictEqual([STUCK]);
});

test("replay reports the division fault of an action", () => {
  const Then = m.model("Then", { n: m.int(0, 2).initial(1), d: m.int(0, 1).initial(0) });
  const share = m.action(Then, "share", { changes: (s) => ({ n: m.div(s.n, s.d) }) });
  const state = (n: number) => [
    { name: "n", value: n },
    { name: "d", value: 0 },
  ];
  const path = { start: state(1), steps: [{ action: "share", state: state(0) }] };
  expect(() => replay(Then, path, { exports: { share } })).toThrow(/division/);
});
