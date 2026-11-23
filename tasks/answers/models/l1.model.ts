import { m } from "@earnest/model";

export const G = m.model("G", {
	d: m.int(0, 10).initial(0),
	p: m.int(0, 59048).initial(0),
});

export const branch0 = m.action(G, "branch 0", {
	when: (s) => s.d < 8,
	changes: (s) => ({ d: s.d + 1, p: s.p * 3 + 0 }),
});
export const branch1 = m.action(G, "branch 1", {
	when: (s) => s.d < 8,
	changes: (s) => ({ d: s.d + 1, p: s.p * 3 + 1 }),
});
export const branch2 = m.action(G, "branch 2", {
	when: (s) => s.d < 8,
	changes: (s) => ({ d: s.d + 1, p: s.p * 3 + 2 }),
});
export const deeper = m.action(G, "deeper", {
	input: { b: m.int(0, 2) },
	when: (s) => s.d === 8,
	changes: (s, { b }) => ({ d: 9, p: s.p * 3 + b }),
});
export const up = m.action(G, "up", {
	when: (s) => s.d > 6,
	changes: (s) => ({ d: s.d - 1, p: m.div(s.p, 3) }),
});
export const sib = m.action(G, "sib", {
	input: { k: m.int(1, 4) },
	when: (s, { k }) => s.d > 6 && s.p >= k,
	changes: (s, { k }) => ({ p: s.p - k }),
});
export const home = m.action(G, "home", {
	when: (s) => s.d > 7,
	changes: (s) => ({ d: 1, p: m.rem(s.p, 3) }),
});

export const r1 = m.rule(G, "not mid", (s) => !(s.d === 9 && s.p === 200));
export const r2 = m.rule(G, "not that one", (s) => !(s.d === 8 && s.p === 5000));
export const r3 = m.rule(G, "never", (s) => s.d <= 10);
