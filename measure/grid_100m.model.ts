import { m } from "@earnest/model";

export const Grid100m = m.model("Grid100m", {
	a: m.int(0, 9).initial(0),
	b: m.int(0, 9).initial(0),
	c: m.int(0, 9).initial(0),
	d: m.int(0, 9).initial(0),
	e: m.int(0, 9).initial(0),
	f: m.int(0, 9).initial(0),
	g: m.int(0, 9).initial(0),
	h: m.int(0, 9).initial(0),
});

export const turn_a = m.action(Grid100m, "wheel a turns", {
	when: (s) => s.a < 9,
	changes: (s) => ({ a: s.a + 1 }),
});

export const turn_b = m.action(Grid100m, "wheel b turns", {
	when: (s) => s.b < 9,
	changes: (s) => ({ b: s.b + 1 }),
});

export const turn_c = m.action(Grid100m, "wheel c turns", {
	when: (s) => s.c < 9,
	changes: (s) => ({ c: s.c + 1 }),
});

export const turn_d = m.action(Grid100m, "wheel d turns", {
	when: (s) => s.d < 9,
	changes: (s) => ({ d: s.d + 1 }),
});

export const turn_e = m.action(Grid100m, "wheel e turns", {
	when: (s) => s.e < 9,
	changes: (s) => ({ e: s.e + 1 }),
});

export const turn_f = m.action(Grid100m, "wheel f turns", {
	when: (s) => s.f < 9,
	changes: (s) => ({ f: s.f + 1 }),
});

export const turn_g = m.action(Grid100m, "wheel g turns", {
	when: (s) => s.g < 9,
	changes: (s) => ({ g: s.g + 1 }),
});

export const turn_h = m.action(Grid100m, "wheel h turns", {
	when: (s) => s.h < 9,
	changes: (s) => ({ h: s.h + 1 }),
});

export const inside = m.rule(
	Grid100m,
	"Every wheel stays on its dial",
	(s) => s.a <= 9 && s.b <= 9 && s.c <= 9 && s.d <= 9 && s.e <= 9 && s.f <= 9 && s.g <= 9 && s.h <= 9,
);

export const stopped = m.end(
	Grid100m,
	"Every wheel is at its last notch",
	(s) => s.a === 9 && s.b === 9 && s.c === 9 && s.d === 9 && s.e === 9 && s.f === 9 && s.g === 9 && s.h === 9,
);

export type State = m.Infer<typeof Grid100m>;
