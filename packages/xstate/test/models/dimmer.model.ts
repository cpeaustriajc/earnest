import { m } from "@earnestjs/model";

const States = m.enum([
  "on.low",
  "on.high",
  "off (on.last: on.low)",
  "off (on.last: on.high)",
  "on.low (on.last: on.low)",
  "on.high (on.last: on.high)",
  "on.high (on.last: on.low)",
]);

const UP_when = (s: State): boolean => {
  if (s.state === "on.low" && s.ticks === 0) return true;
  if (s.state === "on.low (on.last: on.low)" && s.ticks === 0) return true;
  return false;
};

const UP_next_state = (s: State): State["state"] => {
  if (s.state === "on.low (on.last: on.low)") return "on.high (on.last: on.low)";
  return "on.high";
};

const OFF_when = (s: State): boolean => {
  if (s.state === "off (on.last: on.low)") return false;
  if (s.state === "off (on.last: on.high)") return false;
  return true;
};

const OFF_next_state = (s: State): State["state"] => {
  if (s.state === "on.low") return "off (on.last: on.low)";
  if (s.state === "on.low (on.last: on.low)") return "off (on.last: on.low)";
  return "off (on.last: on.high)";
};

const ON_when = (s: State): boolean => {
  if (s.state === "off (on.last: on.low)") return true;
  if (s.state === "off (on.last: on.high)") return true;
  return false;
};

const ON_next_state = (s: State): State["state"] => {
  if (s.state === "off (on.last: on.high)") return "on.high (on.last: on.high)";
  return "on.low (on.last: on.low)";
};

const TICK_when = (s: State): boolean => {
  if (s.state === "off (on.last: on.low)" && s.ticks === 0) return true;
  if (s.state === "off (on.last: on.high)" && s.ticks === 0) return true;
  return false;
};

const fresh_holds = (s: State): boolean => {
  if (s.state === "on.high" && s.ticks === 1) return false;
  if (s.state === "on.high (on.last: on.high)" && s.ticks === 1) return false;
  if (s.state === "on.high (on.last: on.low)" && s.ticks === 1) return false;
  return true;
};

export const Dimmer = m.model("Dimmer", {
  state: States.initial("on.low"),
  ticks: m.int(0, 1).initial(0),
});

export const UP = m.action(Dimmer, "UP", {
  when: (s) => UP_when(s),
  changes: (s) => ({ state: UP_next_state(s) }),
});

export const OFF = m.action(Dimmer, "OFF", {
  when: (s) => OFF_when(s),
  changes: (s) => ({ state: OFF_next_state(s) }),
});

export const ON = m.action(Dimmer, "ON", {
  when: (s) => ON_when(s),
  changes: (s) => ({ state: ON_next_state(s) }),
});

export const TICK = m.action(Dimmer, "TICK", {
  when: (s) => TICK_when(s),
  changes: () => ({ ticks: 1 }),
});

export const fresh = m.rule(Dimmer, "The light is never high after a tick", (s) => fresh_holds(s));

export type State = m.Infer<typeof Dimmer>;
