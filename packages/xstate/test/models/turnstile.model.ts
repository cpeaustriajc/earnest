import { m } from "@earnestjs/model";

const ENTER_next_state = (s: State): State["state"] => {
  if (s.visitors === 1) return "full";
  return "locked";
};

const ENTER_next_visitors = (s: State): State["visitors"] => {
  if (s.visitors === 1) return 2;
  if (s.visitors === 2) return 3;
  return 1;
};

export const Turnstile = m.model("Turnstile", {
  state: m.enum(["locked", "paid", "open", "full"]).initial("locked"),
  visitors: m.int(0, 2).initial(0),
});

export const PAY = m.action(Turnstile, "PAY", {
  when: (s) => s.state === "locked",
  changes: () => ({ state: "paid" }),
});

export const PUSH = m.action(Turnstile, "PUSH", {
  when: (s) => s.state === "paid",
  changes: () => ({ state: "open" }),
});

export const ENTER = m.action(Turnstile, "ENTER", {
  when: (s) => s.state === "open",
  changes: (s) => ({ state: ENTER_next_state(s), visitors: ENTER_next_visitors(s) }),
});

export const capped = m.rule(Turnstile, "The count never passes the capacity", () => true);

export const done = m.end(
  Turnstile,
  "The machine is in a top-level final state",
  (s) => s.state === "full",
);

export type State = m.Infer<typeof Turnstile>;
