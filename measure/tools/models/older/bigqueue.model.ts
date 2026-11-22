import { m } from "@earnest/model";

const Color = m.enum(["red", "green", "blue"]);

export const BigQueue = m.model("BigQueue", {
	q: m.queue(Color, { capacity: 9 }),
	sum: m.int(-1, 30).initial(0),
});

export const send = m.action(BigQueue, "a color is sent", {
	input: { c: Color },
	when: (s) => !s.q.isFull,
	changes: (s, { c }) => ({ q: s.q.send(c), sum: s.sum + (c === "red" ? 3 : 1) }),
});

export const take = m.action(BigQueue, "the oldest is taken", {
	when: (s) => !s.q.isEmpty,
	changes: (s) => ({ q: s.q.receive(), sum: s.sum - 1 }),
});

export const room = m.rule(BigQueue, "The sum stays under twenty-eight", (s) => s.sum < 28);
