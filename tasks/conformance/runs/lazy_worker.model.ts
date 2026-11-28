import { m } from "@earnestjs/model";

export const LazyWorker = m.model("LazyWorker", {
	busy: m.boolean().initial(false),
	done: m.boolean().initial(false),
});

export const fidget = m.action(LazyWorker, "the worker looks busy, or stops looking busy", {
	when: (s) => !s.done,
	changes: (s) => ({ busy: !s.busy }),
});

export const finish = m.action(LazyWorker, "the worker finishes the job", {
	when: (s) => !s.done,
	changes: () => ({ done: true }),
});

export const over = m.end(LazyWorker, "The job is done", (s) => s.done);

export const finishes = m.eventually(LazyWorker, "The job gets done", {
	reaches: (s) => s.done,
});

export type State = m.Infer<typeof LazyWorker>;
