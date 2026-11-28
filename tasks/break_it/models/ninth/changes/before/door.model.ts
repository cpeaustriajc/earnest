import { m } from "@earnestjs/model";

export const Door = m.model("Door", {
	pos: m.oneOf(["shut", "open"]).initial("shut"),
});

export const push = m.action(Door, "the door is pushed", {
	when: (s) => s.pos === 'shut',
	changes: () => ({ pos: 'open' as const }),
});

export const opened = m.end(Door, "the door is open", (s) => s.pos === 'open');

export const known = m.rule(Door, "the door is shut or open", s => s.pos === 'shut' || s.pos === 'open');

export const opens = m.eventually(Door, "the door opens", { reaches: s => s.pos === 'open' });

export const once = m.witness(Door, "one push opens it", { steps: [push], expect: s => s.pos === 'open' });
