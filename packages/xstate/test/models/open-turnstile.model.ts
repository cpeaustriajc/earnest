import { m } from "@earnestjs/model";

export const OpenTurnstile = m.model("OpenTurnstile", {
  state: m.enum(["locked", "open"]).initial("locked"),
  paid: m.boolean().initial(false),
});

export const PAY = m.action(OpenTurnstile, "PAY", {
  when: (s) => s.state === "locked" && !s.paid,
  changes: () => ({ paid: true }),
});

export const PUSH = m.action(OpenTurnstile, "PUSH", {
  when: (s) => s.state === "locked",
  changes: () => ({ state: "open" }),
});

export const ENTER = m.action(OpenTurnstile, "ENTER", {
  when: (s) => s.state === "open",
  changes: () => ({ state: "locked", paid: false }),
});

export const locked = m.rule(
  OpenTurnstile,
  "The gate is never open unpaid",
  (s) => !(s.state === "open" && !s.paid),
);

export type State = m.Infer<typeof OpenTurnstile>;
