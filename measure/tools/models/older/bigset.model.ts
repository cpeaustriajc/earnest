import { m } from "@earnest/model";

export const BigSet = m.model("BigSet", {
	s: m.set(m.int(-6, 7)).initial(m.setOf()),
	last: m.int(-6, 7).optional().initial(null),
});

export const add = m.action(BigSet, "an item is added", {
	input: { i: m.int(-6, 7) },
	when: (s, { i }) => !s.s.has(i),
	changes: (s, { i }) => ({ s: s.s.add(i), last: i }),
});

export const drop = m.action(BigSet, "an item is dropped", {
	input: { i: m.int(-6, 7) },
	when: (s, { i }) => s.s.has(i),
	changes: (s, { i }) => ({ s: s.s.delete(i), last: null }),
});

export const notfull = m.rule(BigSet, "The set never holds thirteen items", (s) => s.s.size < 13);
