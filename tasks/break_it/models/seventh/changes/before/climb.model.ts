import { m } from "@earnestjs/model";

export const Climb = m.model("Climb", {
	n: m.int(0, 3).initial(0),
});

export const inc = m.action(Climb, "the count goes up", {
	when: (s) => s.n < 3,
	changes: (s) => ({ n: s.n + 1 }),
});

export const top = m.end(Climb, "the count stops at three", (s) => s.n === 3);

export const climbs = m.eventually(Climb, "the count gets to three", {
	reaches: (s) => s.n === 3,
});

export const three = m.witness(Climb, "three steps reach three", {
	steps: [inc, inc, inc],
	expect: (s) => s.n === 3,
});
