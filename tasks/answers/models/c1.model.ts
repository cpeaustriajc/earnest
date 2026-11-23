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
export const fold1 = m.action(G, "fold 1", {
	when: (s) => s.d === 8,
	changes: (s) => ({ d: 9, p: m.rem(s.p * 1 + 1, 997) }),
});
export const fold7 = m.action(G, "fold 7", {
	when: (s) => s.d === 8,
	changes: (s) => ({ d: 9, p: m.rem(s.p * 7 + 7, 1499) }),
});
export const fold13 = m.action(G, "fold 13", {
	when: (s) => s.d === 8,
	changes: (s) => ({ d: 9, p: m.rem(s.p * 13 + 13, 211) }),
});
export const again = m.action(G, "again", {
	when: (s) => s.d === 9,
	changes: (s) => ({ d: 10, p: m.rem(s.p * 3, 401) }),
});

export const r1 = m.rule(G, "not mid", (s) => !(s.d === 9 && s.p === 200));
export const r2 = m.rule(G, "not that one", (s) => !(s.d === 8 && s.p === 5000));
export const r3 = m.rule(G, "never", (s) => s.d <= 10);
