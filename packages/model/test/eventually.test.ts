import { expect, test } from "vitest";
import { type RuleResult, explore, m } from "@earnestjs/model";

const STAYS = "nothing changes the state from here, so the run can stay in it forever";

const only = (rules: readonly RuleResult[], id: string): RuleResult => {
  const found = rules.find((rule) => rule.id === id);
  if (found === undefined) {
    throw new Error(`no result for ${id}`);
  }
  return found;
};

const cart = (cancels: boolean) => {
  const Cart = m.model("Cart", {
    approved: m.boolean().initial(false),
    offers: m.boolean().initial(false),
  });
  const approve = m.action(Cart, "the shop approves the cart", {
    when: (s) => !s.approved,
    changes: () => ({ approved: true }),
  });
  const cancel = m.action(Cart, "the shop takes the approval back", {
    when: (s) => cancels && s.approved && !s.offers,
    changes: () => ({ approved: false }),
  });
  const offer = m.action(Cart, "the cart offers checkout", {
    when: (s) => s.approved && !s.offers,
    changes: () => ({ offers: true }),
  });
  const done = m.end(Cart, "Checkout is offered", (s) => s.offers);
  const offered = m.eventually(Cart, "Once approved, the cart offers checkout", {
    after: (s) => s.approved,
    reaches: (s) => s.offers,
  });
  const approvedSoon = m.eventually(Cart, "The cart is approved", {
    reaches: (s) => s.approved,
  });
  return {
    Cart,
    exports: { approve, cancel, offer, done, offered, approvedSoon },
  };
};

const shown = (approved: boolean) => [
  { name: "approved", value: approved },
  { name: "offers", value: false },
];

const approval = (action: string, label: string, before: boolean) => ({
  action,
  label,
  input: [],
  changes: [{ field: "approved", before, after: !before }],
  state: shown(!before),
});

const worker = (fair: boolean) => {
  const Worker = m.model("Worker", {
    busy: m.boolean().initial(false),
    done: m.boolean().initial(false),
  });
  const fidget = m.action(Worker, "the worker fidgets", {
    when: (s) => !s.done,
    changes: (s) => ({ busy: !s.busy }),
  });
  const finish = m.action(Worker, "the worker finishes", {
    fair,
    when: (s) => !s.done,
    changes: () => ({ done: true }),
  });
  const over = m.end(Worker, "The job is done", (s) => s.done);
  const finishes = m.eventually(Worker, "The job gets done", { reaches: (s) => s.done });
  return explore(Worker, { exports: { fidget, finish, over, finishes } });
};

test("rules about runs that every run meets hold, after the rules", () => {
  const { Cart, exports } = cart(false);
  const low = m.rule(Cart, "Checkout needs approval", (s) => !s.offers || s.approved);
  const found = explore(Cart, { exports: { ...exports, low } });
  expect(found.status).toBe("pass");
  expect(found.states).toBe(3);
  expect(found.rules).toStrictEqual([
    { id: "low", says: "Checkout needs approval", verdict: "holds", path: null },
    {
      id: "offered",
      says: "Once approved, the cart offers checkout",
      verdict: "holds",
      path: null,
      kind: "eventually",
    },
    {
      id: "approvedSoon",
      says: "The cart is approved",
      verdict: "holds",
      path: null,
      kind: "eventually",
    },
  ]);
});

test("a loop that never reaches breaks the rule, with the path once round it", () => {
  const { Cart, exports } = cart(true);
  const found = explore(Cart, { exports });
  expect(found.status).toBe("fail");
  expect(found.findings).toStrictEqual([]);
  expect(only(found.rules, "offered")).toStrictEqual({
    id: "offered",
    says: "Once approved, the cart offers checkout",
    verdict: "broken",
    path: {
      start: shown(false),
      steps: [
        approval("approve", "the shop approves the cart", false),
        approval("cancel", "the shop takes the approval back", true),
        approval("approve", "the shop approves the cart", false),
      ],
    },
    kind: "eventually",
    reason: "steps 2 to 3 can repeat forever",
    repeatsFrom: 1,
  });
  expect(only(found.rules, "approvedSoon").verdict).toBe("holds");
});

test("a run that stays in an end or in a state no action changes breaks the rule", () => {
  const Kettle = m.model("Kettle", {
    heat: m.int(0, 1).initial(0),
    on: m.boolean().initial(true),
  });
  const warm = m.action(Kettle, "the water warms up", {
    when: (s) => s.heat === 0,
    changes: () => ({ heat: 1 }),
  });
  const simmer = m.action(Kettle, "the water simmers", {
    fair: true,
    when: (s) => s.heat === 1,
    changes: () => ({ heat: 1 }),
  });
  const switchesOff = m.eventually(Kettle, "Once warm, the kettle switches off", {
    after: (s) => s.heat === 1,
    reaches: (s) => !s.on,
  });
  const found = explore(Kettle, { exports: { warm, simmer, switchesOff } });
  expect(found.findings).toStrictEqual([]);
  const stays = only(found.rules, "switchesOff");
  expect(stays.reason).toBe(STAYS);
  expect(stays.repeatsFrom).toBeUndefined();
  expect(stays.path?.steps.map((step) => step.action)).toStrictEqual(["warm"]);

  const Ticket = m.model("Ticket", { stage: m.enum(["open", "fixed", "closed"]).initial("open") });
  const fix = m.action(Ticket, "the bug is fixed", {
    when: (s) => s.stage === "open",
    changes: () => ({ stage: "fixed" }),
  });
  const decline = m.action(Ticket, "the ticket is declined", {
    when: (s) => s.stage === "open",
    changes: () => ({ stage: "closed" }),
  });
  const closed = m.end(Ticket, "The ticket is closed", (s) => s.stage !== "open");
  const fixed = m.eventually(Ticket, "Every ticket is fixed", {
    reaches: (s) => s.stage === "fixed",
  });
  const ticket = only(explore(Ticket, { exports: { fix, decline, closed, fixed } }).rules, "fixed");
  expect(ticket.reason).toBe(STAYS);
  expect(ticket.path).toStrictEqual({
    start: [{ name: "stage", value: "open" }],
    steps: [
      {
        action: "decline",
        label: "the ticket is declined",
        input: [],
        changes: [{ field: "stage", before: "open", after: "closed" }],
        state: [{ name: "stage", value: "closed" }],
      },
    ],
  });
});

test("a loop that only puts off a fair action breaks no rule, and without fair it does", () => {
  expect(only(worker(true).rules, "finishes").verdict).toBe("holds");
  const lazy = only(worker(false).rules, "finishes");
  expect(lazy.verdict).toBe("broken");
  expect(lazy.repeatsFrom).toBe(0);
  expect(lazy.reason).toBe("steps 1 to 2 can repeat forever");
  expect(lazy.path?.steps.map((step) => step.action)).toStrictEqual(["fidget", "fidget"]);
});

test("a fair action with inputs is fair as a whole", () => {
  const Desk = m.enum(["front", "back"]);
  const Dispatch = m.model("Dispatch", {
    desk: Desk.optional().initial(null),
    tick: m.boolean().initial(false),
  });
  const idle = m.action(Dispatch, "the dispatcher waits", {
    when: (s) => s.desk === null,
    changes: (s) => ({ tick: !s.tick }),
  });
  const route = m.action(Dispatch, "the dispatcher routes the call", {
    input: { to: Desk },
    fair: true,
    when: (s) => s.desk === null,
    changes: (_, { to }) => ({ desk: to }),
  });
  const sent = m.end(Dispatch, "The call is at a desk", (s) => s.desk !== null);
  const routed = m.eventually(Dispatch, "Every call is routed", {
    reaches: (s) => s.desk !== null,
  });
  const toTheBack = m.eventually(Dispatch, "Every call goes to the back", {
    reaches: (s) => s.desk === "back",
  });
  const found = explore(Dispatch, { exports: { idle, route, sent, routed, toTheBack } });
  expect(found.states).toBe(6);
  expect(only(found.rules, "routed").verdict).toBe("holds");
  const back = only(found.rules, "toTheBack");
  expect(back.reason).toBe(STAYS);
  expect(back.path?.steps.map((step) => [step.action, step.input])).toStrictEqual([
    ["route", [{ name: "to", value: "front" }]],
  ]);
});

test("a search stopped by the state limit leaves every rule about runs unknown", () => {
  const { Cart, exports } = cart(true);
  const found = explore(Cart, { exports, maxStates: 2 });
  expect(found.status).toBe("unfinished");
  expect(found.rules.map((rule) => [rule.id, rule.verdict])).toStrictEqual([
    ["offered", "unknown"],
    ["approvedSoon", "unknown"],
  ]);
});

test("a rule about runs that can't be judged in a state is unknown, with a fault naming it", () => {
  const Count = m.model("Count", { n: m.int(0, 2).initial(2) });
  const down = m.action(Count, "the count goes down", {
    when: (s) => s.n > 0,
    changes: (s) => ({ n: s.n - 1 }),
  });
  const zero = m.end(Count, "The count is zero", (s) => s.n === 0);
  const halves = m.eventually(Count, "The count halves cleanly", {
    reaches: (s) => m.div(2, s.n) === 2,
  });
  const low = m.eventually(Count, "The count gets low", { reaches: (s) => s.n <= 1 });
  const found = explore(Count, { exports: { down, zero, halves, low } });
  expect(found.status).toBe("fail");
  expect(found.rules.map((rule) => [rule.id, rule.verdict])).toStrictEqual([
    ["halves", "unknown"],
    ["low", "holds"],
  ]);
  expect(found.findings.map((finding) => [finding.kind, finding.message])).toStrictEqual([
    [
      "fault",
      "the rule `halves` takes a division with a negative number or a divisor below 1 in this state",
    ],
  ]);
});

test("a state out of range leaves a rule about runs unknown", () => {
  const Climb = m.model("Climb", { n: m.int(0, 2).initial(0) });
  const up = m.action(Climb, "the count goes up", { changes: (s) => ({ n: s.n + 1 }) });
  const top = m.eventually(Climb, "The count reaches the top", { reaches: (s) => s.n === 2 });
  const found = explore(Climb, { exports: { up, top } });
  expect(only(found.rules, "top").verdict).toBe("unknown");
  expect(found.findings.map((finding) => finding.kind)).toStrictEqual(["out-of-range"]);
});

test("rules about runs come before the witnesses", () => {
  const { Cart, exports } = cart(false);
  const starts = m.witness(Cart, "The cart starts unapproved", {
    steps: [],
    expect: (s) => !s.approved,
  });
  const found = explore(Cart, { exports: { ...exports, starts } });
  expect(found.rules.map((rule) => [rule.id, rule.kind])).toStrictEqual([
    ["offered", "eventually"],
    ["approvedSoon", "eventually"],
    ["starts", "witness"],
  ]);
});
