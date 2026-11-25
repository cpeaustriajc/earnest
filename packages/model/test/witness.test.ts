import { expect, test } from "vitest";
import { type RuleResult, explore, m } from "@earnestjs/model";

const lift = () => {
  const Lift = m.model("Lift", { floor: m.int(0, 2), doors: m.boolean().initial(false) });
  const up = m.action(Lift, "the lift goes up a floor", {
    when: (s) => !s.doors && s.floor < 2,
    changes: (s) => ({ floor: s.floor + 1 }),
  });
  const open = m.action(Lift, "the doors open", {
    when: (s) => !s.doors,
    changes: () => ({ doors: true }),
  });
  const shut = m.action(Lift, "the doors shut", {
    when: (s) => s.doors,
    changes: () => ({ doors: false }),
  });
  return { Lift, up, open, shut };
};

const only = (rules: readonly RuleResult[], id: string): RuleResult => {
  const found = rules.find((rule) => rule.id === id);
  if (found === undefined) {
    throw new Error(`no result for ${id}`);
  }
  return found;
};

const floorOne = [
  { name: "floor", value: 1 },
  { name: "doors", value: false },
];

const upFromOne = {
  action: "up",
  label: "the lift goes up a floor",
  input: [],
  changes: [{ field: "floor", before: 1, after: 2 }],
  state: [
    { name: "floor", value: 2 },
    { name: "doors", value: false },
  ],
};

test("a witness that can be played from every start state holds, after the rules", () => {
  const { Lift, ...parts } = lift();
  const low = m.rule(Lift, "The lift stays in the building", (s) => s.floor <= 2);
  const rides = m.witness(Lift, "The doors open and shut again", {
    steps: [parts.open, parts.shut],
    expect: (s) => !s.doors,
  });
  const found = explore(Lift, { exports: { ...parts, low, rides } });
  expect(found.status).toBe("pass");
  expect(found.states).toBe(6);
  expect(found.rules).toStrictEqual([
    { id: "low", says: "The lift stays in the building", verdict: "holds", path: null },
    {
      id: "rides",
      says: "The doors open and shut again",
      verdict: "holds",
      path: null,
      kind: "witness",
    },
  ]);
});

test("a step that can't happen breaks the witness, with the path played before it", () => {
  const { Lift, ...parts } = lift();
  const climbs = m.witness(Lift, "Two floors up", {
    steps: [parts.up, parts.up],
    expect: (s) => s.floor === 2,
  });
  const found = explore(Lift, { exports: { ...parts, climbs } });
  expect(found.status).toBe("fail");
  expect(found.findings).toStrictEqual([]);
  expect(only(found.rules, "climbs")).toStrictEqual({
    id: "climbs",
    says: "Two floors up",
    verdict: "broken",
    path: { start: floorOne, steps: [upFromOne] },
    kind: "witness",
    reason: "step 2, `up`, can't happen here",
  });
});

test("an expectation that is false at the end breaks the witness", () => {
  const { Lift, ...parts } = lift();
  const arrives = m.witness(Lift, "One floor up", {
    steps: [parts.up],
    expect: (s) => s.floor === 1,
  });
  const found = only(explore(Lift, { exports: { ...parts, arrives } }).rules, "arrives");
  expect(found.reason).toBe("what the witness expects is false here");
  expect(found.path).toStrictEqual({ start: floorOne, steps: [upFromOne] });
});

test("the path comes from the first start state that breaks the witness", () => {
  const { Lift, ...parts } = lift();
  const grounded = m.witness(Lift, "The lift starts on the ground floor", {
    steps: [],
    expect: (s) => s.floor === 0,
  });
  const found = only(explore(Lift, { exports: { ...parts, grounded } }).rules, "grounded");
  expect(found.verdict).toBe("broken");
  expect(found.path).toStrictEqual({ start: floorOne, steps: [] });
  expect(found.reason).toBe("what the witness expects is false here");
});

test("a step with an input is played with the value m.step gives it", () => {
  const By = m.enum(["key", "mouse"]);
  const Panel = m.model("Panel", { last: By.optional().initial(null) });
  const more = m.action(Panel, "the reader shows more", {
    input: { by: By },
    changes: (_, { by }) => ({ last: by }),
  });
  const byMouse = m.witness(Panel, "More by mouse", {
    steps: [m.step(more, { by: "mouse" })],
    expect: (s) => s.last === "key",
  });
  const found = only(explore(Panel, { exports: { more, byMouse } }).rules, "byMouse");
  expect(found.path).toStrictEqual({
    start: [{ name: "last", value: null }],
    steps: [
      {
        action: "more",
        label: "the reader shows more",
        input: [{ name: "by", value: "mouse" }],
        changes: [{ field: "last", before: null, after: "mouse" }],
        state: [{ name: "last", value: "mouse" }],
      },
    ],
  });
});

test("a step that takes a field out of range is the last step of the path", () => {
  const Tally = m.model("Tally", { n: m.int(0, 1).initial(0) });
  const grow = m.action(Tally, "grow", { changes: (s) => ({ n: s.n + 5 }) });
  const grows = m.witness(Tally, "It grows", { steps: [grow, grow], expect: () => true });
  const found = only(explore(Tally, { exports: { grow, grows } }).rules, "grows");
  expect(found.reason).toBe("step 1, `grow`, takes the field `n` out of the values it declares");
  expect(found.path?.steps.map((step) => step.action)).toStrictEqual(["grow"]);
});

test("a fault in a condition, a change or the expectation breaks the witness and says where", () => {
  const Jar = m.model("Jar", {
    n: m.int(-1, 1).initial(0),
    box: m
      .record({ qty: m.int(0, 1) })
      .optional()
      .initial(null),
  });
  const drop = m.action(Jar, "drop", { when: (s) => s.n > -1, changes: (s) => ({ n: s.n - 1 }) });
  const fill = m.action(Jar, "fill", {
    when: (s) => m.rem(s.n, 2) === 0,
    changes: (s) => ({ n: s.n + 1 }),
  });
  const peek = m.action(Jar, "peek", { changes: (s) => ({ n: m.some(s.box).qty }) });
  const inCondition = m.witness(Jar, "drop and fill", {
    steps: [drop, fill],
    expect: () => true,
  });
  const inChange = m.witness(Jar, "peek", { steps: [peek], expect: () => true });
  const inExpect = m.witness(Jar, "nothing", {
    steps: [],
    expect: (s) => m.some(s.box).qty === 0,
  });
  const found = explore(Jar, {
    exports: { drop, fill, peek, inCondition, inChange, inExpect },
  }).rules;
  expect(only(found, "inCondition").reason).toBe(
    "step 2: the condition of the action `fill` takes a remainder with a negative number or a divisor below 1 here",
  );
  expect(only(found, "inCondition").path?.steps.map((step) => step.action)).toStrictEqual(["drop"]);
  expect(only(found, "inChange").reason).toBe(
    "step 1: the change of the action `peek` reads a value that is null here",
  );
  expect(only(found, "inChange").path?.steps).toStrictEqual([]);
  expect(only(found, "inExpect").reason).toBe(
    "what the witness expects reads a value that is null here",
  );
});

test("a witness is unknown when there are more start states than maxStates", () => {
  const { Lift, ...parts } = lift();
  const rides = m.witness(Lift, "The doors open and shut again", {
    steps: [parts.open, parts.shut],
    expect: (s) => !s.doors,
  });
  const grounded = m.witness(Lift, "The lift starts on the ground floor", {
    steps: [],
    expect: (s) => s.floor === 0,
  });
  const exports = { ...parts, rides, grounded };
  const verdicts = (maxStates: number) =>
    explore(Lift, { exports, maxStates }).rules.map((rule) => [rule.id, rule.verdict]);
  expect(verdicts(1)).toStrictEqual([
    ["rides", "unknown"],
    ["grounded", "unknown"],
  ]);
  expect(verdicts(2)).toStrictEqual([
    ["rides", "unknown"],
    ["grounded", "broken"],
  ]);
  expect(verdicts(3)).toStrictEqual([
    ["rides", "holds"],
    ["grounded", "broken"],
  ]);
  const unknown = only(explore(Lift, { exports, maxStates: 1 }).rules, "rides");
  expect(unknown).toStrictEqual({
    id: "rides",
    says: "The doors open and shut again",
    verdict: "unknown",
    path: null,
    kind: "witness",
  });
});

test("m.witness throws for a step that is not an action declared on the model before it", () => {
  const { Lift, up } = lift();
  const Other = m.model("Other", { on: m.boolean().initial(false) });
  const flip = m.action(Other, "flip", { changes: (s) => ({ on: !s.on }) });
  expect(() =>
    m.witness(Lift, "Up and flip", { steps: [up, flip as never], expect: () => true }),
  ).toThrow(
    'Lift: step 2 of "Up and flip" is not an action declared on this model before the witness',
  );
});

test("m.witness throws for an input the action does not declare", () => {
  const By = m.enum(["key", "mouse"]);
  const Panel = m.model("Panel", { last: By.optional().initial(null) });
  const more = m.action(Panel, "the reader shows more", {
    input: { by: By },
    changes: (_, { by }) => ({ last: by }),
  });
  const stranger = { by: "pen" } as never;
  expect(() =>
    m.witness(Panel, "More by pen", { steps: [m.step(more, stranger)], expect: () => true }),
  ).toThrow(
    'Panel: step 1 of "More by pen" gives "the reader shows more" an input it does not declare',
  );
  const bare = more as never;
  expect(() => m.witness(Panel, "More", { steps: [bare], expect: () => true })).toThrow(
    'Panel: step 1 of "More" gives "the reader shows more" an input it does not declare',
  );
});

test("a witness and a step are frozen", () => {
  const { Lift, up } = lift();
  const climbs = m.witness(Lift, "Up", { steps: [up], expect: () => true });
  expect(Object.isFrozen(climbs)).toBe(true);
  expect(Object.isFrozen(climbs.steps)).toBe(true);
  expect(climbs.kind).toBe("witness");
  expect(climbs.says).toBe("Up");
});
