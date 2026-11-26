import { m } from "@earnestjs/model";

export const Bag = m.model("Bag", {
	items: m.set(m.int(0, 2)).initial(m.setOf()),
});

export const put = m.action(Bag, "a number goes into the bag", {
	input: { x: m.int(0, 2) },
	when: (s, { x }) => !s.items.has(x),
	changes: (s, { x }) => ({ items: s.items.add(x) }),
});

export const full = m.end(Bag, "every number is in the bag", (s) => s.items.size === 3);

export const small = m.rule(Bag, "the bag holds at most three numbers", (s) => s.items.size <= 3);

export const holdsTwo = m.eventually(Bag, "the bag comes to hold a two", {
	reaches: (s) => s.items.has(2),
});

export const two = m.witness(Bag, "a two goes in", {
	steps: [m.step(put, { x: 2 })],
	expect: (s) => s.items.has(2),
});
