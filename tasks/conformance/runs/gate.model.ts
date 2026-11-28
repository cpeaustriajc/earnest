import { m } from "@earnestjs/model";

export const Gate = m.model("Gate", {
	door: m.enum(["Locked", "Open"]).initial("Locked"),
	paid: m.boolean().initial(false),
	light: m.boolean().initial(false),
});

export const blink = m.action(Gate, "the light blinks while nobody has paid", {
	when: (s) => !s.paid,
	changes: (s) => ({ light: !s.light }),
});

export const pay = m.action(Gate, "the visitor pays", {
	when: (s) => !s.paid && s.door === "Locked",
	changes: () => ({ paid: true }),
	fair: true,
});

export const push = m.action(Gate, "the visitor pushes", {
	when: (s) => s.paid && s.door === "Locked",
	changes: () => ({ door: "Open" }),
});

export const enter = m.action(Gate, "the visitor walks through", {
	when: (s) => s.door === "Open",
	changes: () => ({ door: "Locked", paid: false }),
});

export const opens = m.eventually(Gate, "Every run comes to an open gate", {
	reaches: (s) => s.door === "Open",
});

export const opens_once_paid = m.eventually(Gate, "Once the visitor pays, the gate opens", {
	after: (s) => s.paid,
	reaches: (s) => s.door === "Open",
});

export const lit_and_paid = m.eventually(Gate, "Every run comes to a paid gate with the light on", {
	reaches: (s) => s.paid && s.light,
});

export const pay_then_push = m.witness(Gate, "Pay, then push: the gate opens", {
	steps: [pay, push],
	expect: (s) => s.door === "Open",
});

export const push_first = m.witness(Gate, "Push before paying: the gate opens", {
	steps: [push],
	expect: (s) => s.door === "Open",
});
