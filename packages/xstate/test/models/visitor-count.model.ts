import { m } from "@earnestjs/model";

const ENTER_group = m.int(1, 2);

const ENTER_next_visitors = (s: State, group: m.Infer<typeof ENTER_group>): State["visitors"] => {
  if (s.visitors === 0) return group;
  if (s.visitors === 1 && group === 1) return 2;
  if (s.visitors === 2 && group === 2) return 4;
  if (s.visitors === 3 && group === 1) return 4;
  if (s.visitors === 3 && group === 2) return 5;
  return 3;
};

export const VisitorCount = m.model("VisitorCount", {
  state: m.enum(["visitorCount"]).initial("visitorCount"),
  visitors: m.int(0, 3).initial(0),
});

export const ENTER = m.action(VisitorCount, "ENTER", {
  input: { group: ENTER_group },
  changes: (s, input) => ({ visitors: ENTER_next_visitors(s, input.group) }),
});

export const counted = m.rule(VisitorCount, "The count is never below zero", () => true);

export type State = m.Infer<typeof VisitorCount>;
