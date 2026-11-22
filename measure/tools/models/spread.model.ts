import { m } from "@earnest/model";

export const Spread = m.model("Spread", {
	x: m.int(0, 1023),
	y: m.int(0, 1).initial(0),
	z: m.int(0, 1).initial(0),
});

export const bump = m.action(Spread, "y goes up", {
	when: (s) => s.y === 0,
	changes: () => ({ y: 1 }),
});

export const jump_one = m.action(Spread, "x jumps one way", {
	input: { k: m.int(0, 1023) },
	when: (s) => s.x === 1023 && s.z === 0,
	changes: (s, { k }) => ({ x: k, y: 0 }),
});

export const jump_two = m.action(Spread, "x jumps another way", {
	input: { k: m.int(0, 1023) },
	when: (s) => s.x === 1023 && s.z === 0,
	changes: (s, { k }) => ({ x: k, y: 0 }),
});

export const lift = m.action(Spread, "x jumps and y goes up", {
	input: { k: m.int(0, 1023) },
	when: (s) => s.x === 1023 && s.z === 0,
	changes: (s, { k }) => ({ x: k, y: 1 }),
});

export const last = m.action(Spread, "z goes up", {
	when: (s) => s.x === 1023 && s.z === 0,
	changes: () => ({ z: 1 }),
});

export const flat = m.rule(Spread, "z stays at zero", (s) => s.z === 0);

export const rest = m.end(Spread, "Anything may end", () => true);
