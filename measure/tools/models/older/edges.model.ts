import { m } from "@earnest/model";

const Color = m.enum(["red", "green", "blue"]);
const Spot = m.record({ x: m.int(-2, 1), on: m.boolean() });

export const Edges = m.model("Edges", {
	n: m.int(-3, 2),
	flag: m.boolean().initial(false),
	color: Color.optional().initial(null),
	spot: Spot.initial({ x: -2, on: false }),
	bag: m.set(m.int(-1, 2)).initial(m.setOf()),
	tags: m.map(Color, m.int(0, 1)).initial(m.mapOf()),
});

export const step = m.action(Edges, "n steps by a choice", {
	input: { k: m.int(0, 3) },
	when: (s) => s.n < 3,
	changes: (s, { k }) => ({ n: s.n + (k > 1 ? 1 : 0) }),
});

export const down = m.action(Edges, "n goes down", {
	when: (s) => s.n > -4,
	changes: (s) => ({ n: s.n - 1 }),
});

export const flip_a = m.action(Edges, "the flag flips one way", {
	when: (s) => s.n === 0,
	changes: (s) => ({ flag: !s.flag }),
});

export const flip_b = m.action(Edges, "the flag flips the other way", {
	when: (s) => s.n === 0,
	changes: (s) => ({ flag: !s.flag }),
});

export const paint = m.action(Edges, "a color is chosen", {
	input: { c: Color },
	when: (s) => s.flag,
	changes: (s, { c }) => ({ color: c, tags: s.tags.set(c, 1) }),
});

export const move = m.action(Edges, "the spot moves", {
	input: { d: m.int(0, 1) },
	when: (s) => s.spot.x < 2,
	changes: (s, { d }) => ({ spot: { x: s.spot.x + d, on: !s.spot.on }, bag: s.bag.add(s.spot.x) }),
});

export const calm = m.rule(Edges, "The flag is down while blue is chosen", (s) => !(s.flag && s.color === "blue"));
export const small = m.rule(Edges, "The bag never holds 1", (s) => !s.bag.has(1));
