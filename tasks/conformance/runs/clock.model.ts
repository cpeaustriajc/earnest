import { m } from "@earnestjs/model";

export const Clock = m.model("Clock", {
	skew: m.int(-10, 10).initial(0),
	hand: m.int(0, 2).initial(0),
	synced: m.boolean().initial(false),
});

export const tick = m.action(Clock, "the hand moves on", {
	changes: (s) => ({ hand: s.hand === 2 ? 0 : s.hand + 1 }),
});

export const sync = m.action(Clock, "the clock syncs with the server", {
	fair: true,
	when: (s) => !s.synced && s.skew <= 5,
	changes: () => ({ synced: true }),
});

export const behind = m.setup(Clock, "The clock runs 10 behind", { skew: -10 });

export const ahead = m.setup(Clock, "The clock runs 10 ahead", { skew: 10 });

export const close = m.rule(Clock, "The clock is at most 5 behind", (s) => s.skew >= -5);

export const syncs = m.eventually(Clock, "The clock syncs", {
	reaches: (s) => s.synced,
});

export const goesRound = m.witness(Clock, "Three ticks bring the hand back to the top", {
	steps: [tick, tick, tick],
	expect: (s) => s.hand === 0,
});

export const catchesUp = m.witness(Clock, "A clock behind can sync", {
	setup: behind,
	steps: [sync],
	expect: (s) => s.synced,
});

export const syncsAhead = m.witness(Clock, "A clock ahead can sync", {
	setup: ahead,
	steps: [sync],
	expect: (s) => s.synced,
});

export type State = m.Infer<typeof Clock>;
