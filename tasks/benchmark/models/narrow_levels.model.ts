import { m } from "@earnest/model";

export const NarrowLevels = m.model("NarrowLevels", {
	t: m.int(0, 1999).initial(0),
	y: m.int(0, 599).initial(0),
});

export const step = m.action(NarrowLevels, "time passes", {
	when: (s) => s.t < 1999,
	changes: (s) => ({ t: s.t + 1 }),
});

export const shift = m.action(NarrowLevels, "time passes and the walker moves", {
	when: (s) => s.t < 1999 && s.y < 599,
	changes: (s) => ({ t: s.t + 1, y: s.y + 1 }),
});

export const inside = m.rule(NarrowLevels, "The walker stays in the corridor", (s) => s.y <= 599);

export const stopped = m.end(NarrowLevels, "Time is at its last tick", (s) => s.t === 1999);

export type State = m.Infer<typeof NarrowLevels>;
