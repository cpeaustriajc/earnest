import { m } from "@earnest/model";

export const Budget = m.model("Budget", {
	a: m.int(0, 1).initial(0),
});

export const stay_one = m.action(Budget, "the first stays", {
	when: (s) => s.a === 0,
	changes: (s) => ({ a: s.a }),
});

export const stay_two = m.action(Budget, "the second stays", {
	when: (s) => s.a === 0,
	changes: (s) => ({ a: s.a }),
});

export const stay_three = m.action(Budget, "the third stays", {
	when: (s) => s.a === 0,
	changes: (s) => ({ a: s.a }),
});

export const go = m.action(Budget, "it moves", {
	when: (s) => s.a === 0,
	changes: () => ({ a: 1 }),
});

export const zero = m.rule(Budget, "It never moves", (s) => s.a === 0);

export const done = m.end(Budget, "It moved", (s) => s.a === 1);
