import { m } from "@earnestjs/model";

export const Worker = m.model("Worker", {
	busy: m.boolean().initial(false),
	done: m.boolean().initial(false),
});

export const fidget = m.action(Worker, "the worker looks busy, or stops looking busy", {
	when: (s) => !s.done,
	changes: (s) => ({ busy: !s.busy }),
});

export const finish = m.action(Worker, "the worker finishes the job", {
	fair: true,
	when: (s) => !s.done,
	changes: () => ({ done: true }),
});

export const over = m.end(Worker, "The job is done", (s) => s.done);

export const finishes = m.eventually(Worker, "The job gets done", {
	reaches: (s) => s.done,
});

export type State = m.Infer<typeof Worker>;
