import { m } from "@earnestjs/model";

export const Climb = m.model("Climb", {
	n: m.int(0, 3).initial(0),
});

export const up = m.action(Climb, "the count goes up by one", {
	when: (s) => s.n < 3,
	changes: (s) => ({ n: s.n + 1 }),
});

export const top = m.end(Climb, "the count is at the top", (s) => s.n === 3);

export const low = m.rule(Climb, "the count stays at most three", (s) => s.n <= 3);

export const half = m.setup(Climb, "the count starts half way", { n: 2 });

export const last = m.witness(Climb, "one step from half way reaches the top", {
	setup: half,
	steps: [up],
	expect: (s) => s.n === 3,
});
