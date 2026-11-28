import { m } from "@earnestjs/model";

const Desk = m.enum(["front", "back"]);

export const Dispatch = m.model("Dispatch", {
	desk: Desk.optional().initial(null),
	tick: m.boolean().initial(false),
});

export const idle = m.action(Dispatch, "the dispatcher waits a moment", {
	when: (s) => s.desk === null,
	changes: (s) => ({ tick: !s.tick }),
});

export const route = m.action(Dispatch, "the dispatcher sends the call to a desk", {
	input: { to: Desk },
	fair: true,
	when: (s) => s.desk === null,
	changes: (_, { to }) => ({ desk: to }),
});

export const sent = m.end(Dispatch, "The call is at a desk", (s) => s.desk !== null);

export const routed = m.eventually(Dispatch, "Every call is sent to a desk", {
	reaches: (s) => s.desk !== null,
});

export const toTheBack = m.eventually(Dispatch, "Every call is sent to the back desk", {
	reaches: (s) => s.desk === "back",
});

export type State = m.Infer<typeof Dispatch>;
