import { m } from "@earnestjs/model";

export const Clock = m.model("Clock", {
	skew: m.int(-2, 2).initial(0),
	synced: m.boolean().initial(false),
});

export const drift = m.action(Clock, "the clock drifts by a second", {
	input: { by: m.oneOf([-1, 1]) },
	when: (s, { by }) => !s.synced && s.skew + by >= -2 && s.skew + by <= 2,
	changes: (s, { by }) => ({ skew: s.skew + by }),
});

export const sync = m.action(Clock, "the clock is synced", {
	when: (s) => !s.synced,
	changes: () => ({ skew: 0, synced: true }),
});

export const done = m.end(Clock, "A synced clock is done", (s) => s.synced);

export const shows_zero = m.rule(Clock, "A synced clock shows no skew", (s) => !s.synced || s.skew === 0);

export const syncs = m.eventually(Clock, "Every run comes to a synced clock", {
	reaches: (s) => s.synced,
});

export const behind = m.setup(Clock, "The clock runs two seconds behind", { skew: -2 });

export const synced_already = m.setup(Clock, "The clock starts synced", { synced: true });

export const drifts_ahead = m.witness(Clock, "A second ahead after one drift", {
	steps: [m.step(drift, { by: 1 })],
	expect: (s) => s.skew === 1,
});

export const catches_up = m.witness(Clock, "Behind, it drifts forward and is synced", {
	setup: behind,
	steps: [m.step(drift, { by: 1 }), sync],
	expect: (s) => s.skew === 0 && s.synced,
});
