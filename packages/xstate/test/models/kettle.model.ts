import { m } from "@earnestjs/model";

const States = m.enum([
  "on.lid.shut, on.water.cold",
  "on.lid.open, on.water.cold",
  "on.lid.shut, on.water.hot",
  "broken",
  "on.lid.open, on.water.hot",
]);

const FILL_cups = m.int(1, 2);

const OPEN_when = (s: State): boolean => {
  if (s.state === "on.lid.shut, on.water.cold") return true;
  if (s.state === "on.lid.shut, on.water.hot") return true;
  return false;
};

const OPEN_next_state = (s: State): State["state"] => {
  if (s.state === "on.lid.shut, on.water.hot") return "on.lid.open, on.water.hot";
  return "on.lid.open, on.water.cold";
};

const SHUT_when = (s: State): boolean => {
  if (s.state === "on.lid.open, on.water.cold") return true;
  if (s.state === "on.lid.open, on.water.hot") return true;
  return false;
};

const SHUT_next_state = (s: State): State["state"] => {
  if (s.state === "on.lid.open, on.water.hot") return "on.lid.shut, on.water.hot";
  return "on.lid.shut, on.water.cold";
};

const FILL_when = (s: State, cups: m.Infer<typeof FILL_cups>): boolean => {
  if (s.state === "on.lid.open, on.water.cold" && s.cups === 0 && s.size === "small" && cups === 1)
    return true;
  if (s.state === "on.lid.open, on.water.cold" && s.cups === 0 && s.size === "large") return true;
  if (s.state === "on.lid.open, on.water.cold" && s.cups === 1 && s.size === "large" && cups === 1)
    return true;
  if (s.state === "on.lid.open, on.water.hot" && s.cups === 0 && s.size === "small" && cups === 1)
    return true;
  if (s.state === "on.lid.open, on.water.hot" && s.cups === 0 && s.size === "large") return true;
  if (s.state === "on.lid.open, on.water.hot" && s.cups === 1 && s.size === "large" && cups === 1)
    return true;
  return false;
};

const FILL_next_cups = (s: State, cups: m.Infer<typeof FILL_cups>): State["cups"] => {
  if (s.cups === 1) return 2;
  return cups;
};

const HEAT_when = (s: State): boolean => {
  if (s.state === "on.lid.shut, on.water.cold" && s.cups === 1) return true;
  if (s.state === "on.lid.shut, on.water.cold" && s.cups === 2) return true;
  if (s.state === "on.lid.open, on.water.cold") return true;
  return false;
};

const HEAT_next_state = (s: State): State["state"] => {
  if (s.state === "on.lid.open, on.water.cold") return "broken";
  return "on.lid.shut, on.water.hot";
};

const POUR_when = (s: State): boolean => {
  if (s.state === "on.lid.shut, on.water.hot") return true;
  if (s.state === "on.lid.open, on.water.hot") return true;
  return false;
};

const POUR_next_state = (s: State): State["state"] => {
  if (s.state === "on.lid.open, on.water.hot") return "on.lid.open, on.water.cold";
  return "on.lid.shut, on.water.cold";
};

const POUR_next_cups = (s: State): State["cups"] => {
  if (s.cups === 1) return 0;
  if (s.cups === 2) return 1;
  return -1;
};

export const Kettle = m.model("Kettle", {
  state: States.initial("on.lid.shut, on.water.cold"),
  cups: m.int(0, 2).initial(0),
  size: m.enum(["small", "large"]).initial("large"),
});

export const OPEN = m.action(Kettle, "OPEN", {
  when: (s) => OPEN_when(s),
  changes: (s) => ({ state: OPEN_next_state(s) }),
});

export const SHUT = m.action(Kettle, "SHUT", {
  when: (s) => SHUT_when(s),
  changes: (s) => ({ state: SHUT_next_state(s) }),
});

export const FILL = m.action(Kettle, "FILL", {
  input: { cups: FILL_cups },
  when: (s, input) => FILL_when(s, input.cups),
  changes: (s, input) => ({ cups: FILL_next_cups(s, input.cups) }),
});

export const HEAT = m.action(Kettle, "HEAT", {
  when: (s) => HEAT_when(s),
  changes: (s) => ({ state: HEAT_next_state(s) }),
});

export const POUR = m.action(Kettle, "POUR", {
  when: (s) => POUR_when(s),
  changes: (s) => ({ state: POUR_next_state(s), cups: POUR_next_cups(s) }),
});

export const shut = m.rule(
  Kettle,
  "The lid is shut while the water is hot",
  (s) => s.state !== "on.lid.open, on.water.hot",
);

export type State = m.Infer<typeof Kettle>;
