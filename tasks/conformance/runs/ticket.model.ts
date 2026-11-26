import { m } from "@earnestjs/model";

const Stage = m.enum(["open", "fixed", "closed"]);

export const Ticket = m.model("Ticket", {
	stage: Stage.initial("open"),
});

export const fix = m.action(Ticket, "a developer fixes the bug", {
	when: (s) => s.stage === "open",
	changes: () => ({ stage: "fixed" }),
});

export const close = m.action(Ticket, "the ticket is closed", {
	when: (s) => s.stage === "fixed",
	changes: () => ({ stage: "closed" }),
});

export const decline = m.action(Ticket, "the ticket is closed without a fix", {
	when: (s) => s.stage === "open",
	changes: () => ({ stage: "closed" }),
});

export const done = m.end(Ticket, "The ticket is closed", (s) => s.stage === "closed");

export const closes = m.eventually(Ticket, "Every ticket is closed", {
	reaches: (s) => s.stage === "closed",
});

export const fixed = m.eventually(Ticket, "Every ticket is fixed", {
	reaches: (s) => s.stage === "fixed",
});

export type State = m.Infer<typeof Ticket>;
