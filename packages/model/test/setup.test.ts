import { expect, test } from "vitest";
import { type RuleResult, explore, m } from "@earnestjs/model";

const only = (rules: readonly RuleResult[], id: string): RuleResult => {
  const found = rules.find((rule) => rule.id === id);
  if (found === undefined) {
    throw new Error(`no result for ${id}`);
  }
  return found;
};

const clock = () => {
  const Clock = m.model("Clock", {
    skew: m.int(-10, 10).initial(0),
    hand: m.int(0, 2).initial(0),
    synced: m.boolean().initial(false),
  });
  const tick = m.action(Clock, "the hand moves on", {
    changes: (s) => ({ hand: s.hand === 2 ? 0 : s.hand + 1 }),
  });
  const sync = m.action(Clock, "the clock syncs", {
    fair: true,
    when: (s) => !s.synced && s.skew <= 5,
    changes: () => ({ synced: true }),
  });
  const behind = m.setup(Clock, "The clock runs 10 behind", { skew: -10 });
  const ahead = m.setup(Clock, "The clock runs 10 ahead", { skew: 10 });
  const close = m.rule(Clock, "The clock is at most 5 behind", (s) => s.skew >= -5);
  const syncs = m.eventually(Clock, "The clock syncs", { reaches: (s) => s.synced });
  const goesRound = m.witness(Clock, "Three ticks come back to the top", {
    steps: [tick, tick, tick],
    expect: (s) => s.hand === 0,
  });
  const catchesUp = m.witness(Clock, "A clock behind can sync", {
    setup: behind,
    steps: [sync],
    expect: (s) => s.synced,
  });
  const syncsAhead = m.witness(Clock, "A clock ahead can sync", {
    setup: ahead,
    steps: [sync],
    expect: (s) => s.synced,
  });
  return {
    Clock,
    behind,
    ahead,
    exports: { tick, sync, behind, ahead, close, syncs, goesRound, catchesUp, syncsAhead },
  };
};

const verdicts = (rules: readonly RuleResult[]) => rules.map((rule) => [rule.id, rule.verdict]);

test("without a setup the search starts where the model declares and plays the witnesses that name none", () => {
  const { Clock, exports } = clock();
  const found = explore(Clock, { exports });
  expect(found.status).toBe("pass");
  expect(found.states).toBe(6);
  expect(verdicts(found.rules)).toStrictEqual([
    ["close", "holds"],
    ["syncs", "holds"],
    ["goesRound", "holds"],
  ]);
});

test("a setup starts the fields it names at its values and plays the witnesses that name it", () => {
  const { Clock, behind, exports } = clock();
  const found = explore(Clock, { exports, setup: behind });
  expect(found.status).toBe("fail");
  expect(found.states).toBe(6);
  expect(verdicts(found.rules)).toStrictEqual([
    ["close", "broken"],
    ["syncs", "holds"],
    ["catchesUp", "holds"],
  ]);
  expect(only(found.rules, "close").path).toStrictEqual({
    start: [
      { name: "skew", value: -10 },
      { name: "hand", value: 0 },
      { name: "synced", value: false },
    ],
    steps: [],
  });
});

test("each setup has its own states and verdicts", () => {
  const { Clock, ahead, exports } = clock();
  const found = explore(Clock, { exports, setup: ahead });
  expect(found.states).toBe(3);
  expect(verdicts(found.rules)).toStrictEqual([
    ["close", "holds"],
    ["syncs", "broken"],
    ["syncsAhead", "broken"],
  ]);
  const syncs = only(found.rules, "syncs");
  expect(syncs.repeatsFrom).toBe(0);
  expect(syncs.reason).toBe("steps 1 to 3 can repeat forever");
  expect(only(found.rules, "syncsAhead").reason).toBe("step 1, `sync`, can't happen here");
});

test("a setup turns several start states into one for each value of the other fields", () => {
  const Lift = m.model("Lift", { floor: m.int(0, 2), doors: m.boolean() });
  const open = m.action(Lift, "the doors open", {
    when: (s) => !s.doors,
    changes: () => ({ doors: true }),
  });
  const shut = m.setup(Lift, "The doors start shut", { doors: false });
  const grounded = m.setup(Lift, "The lift starts on the ground floor", { floor: 0 });
  expect(explore(Lift, { exports: { open } }).states).toBe(6);
  expect(explore(Lift, { exports: { open }, setup: shut }).states).toBe(6);
  expect(explore(Lift, { exports: { open }, setup: grounded }).states).toBe(2);
});

test("m.setup throws for no field, a name that is not a field, or a value the field does not declare", () => {
  const Clock = m.model("Clock", { skew: m.int(-10, 10).initial(0) });
  expect(() => m.setup(Clock, "Nothing", {})).toThrow(
    'Clock: the setup "Nothing" starts no field at another value',
  );
  const drift = { drift: 1 } as never;
  expect(() => m.setup(Clock, "Drifting", drift)).toThrow(
    'Clock: the setup "Drifting" starts `drift`, which is not a field of the model',
  );
  expect(() => m.setup(Clock, "Far behind", { skew: -11 })).toThrow(
    'Clock: the setup "Far behind" starts `skew` at a value the field does not declare: -11 is not a value',
  );
});

test("m.witness throws for a setup of another model or one declared after it", () => {
  const Clock = m.model("Clock", { skew: m.int(-10, 10).initial(0) });
  const Watch = m.model("Watch", { skew: m.int(-10, 10).initial(0) });
  const fast = m.setup(Watch, "The watch runs fast", { skew: 5 });
  expect(() => m.witness(Clock, "Fast", { setup: fast, steps: [], expect: () => true })).toThrow(
    'Clock: the setup of "Fast" is not one declared on this model before the witness',
  );
});

test("explore throws for a setup that is not the model's", () => {
  const { Clock } = clock();
  const other = clock();
  expect(() => explore(Clock, { setup: other.behind })).toThrow(
    "Clock: the setup given to explore is not one declared on this model",
  );
  expect(() => explore(Clock, { setup: {} })).toThrow("is not one declared on this model");
});
