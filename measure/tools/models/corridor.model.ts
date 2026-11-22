import { m } from "@earnest/model";

export const Corridor = m.model("Corridor", {
	t: m.int(0, 19999).initial(0),
	y: m.int(0, 599).initial(0),
});

export const step = m.action(Corridor, "time passes", {
	when: (s) => s.t < 19999,
	changes: (s) => ({ t: s.t + 1 }),
});

export const shift = m.action(Corridor, "time passes and the walker moves", {
	when: (s) => s.t < 19999 && s.y < 599,
	changes: (s) => ({ t: s.t + 1, y: s.y + 1 }),
});

export const inside = m.rule(Corridor, "The walker stays in the corridor", (s) => s.y <= 599);

export type State = m.Infer<typeof Corridor>;
