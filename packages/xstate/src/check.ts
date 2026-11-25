import {
  type SearchFinding,
  type SearchFindingKind,
  type Status,
  type Trail,
  type Verdict,
  compareText,
  search,
} from "@earnestjs/model";
import type {
  AnyMachineSnapshot,
  AnyStateMachine,
  EventFromLogic,
  EventObject,
  InputFrom,
  SnapshotFrom,
} from "xstate";
import { type Arrival, stepping } from "./machine.ts";

/** A rule `checkMachine` judges in every snapshot it reaches: a sentence and its predicate. */
export interface MachineRule<TMachine extends AnyStateMachine> {
  readonly says: string;
  holds(snapshot: SnapshotFrom<TMachine>): boolean;
}

/**
 * The events to try in every snapshot: a list, or a function of the snapshot that gives one, as
 * in the traversal options of `@xstate/graph`.
 */
export type MachineEvents<TMachine extends AnyStateMachine> =
  | readonly EventFromLogic<TMachine>[]
  | ((snapshot: SnapshotFrom<TMachine>) => readonly EventFromLogic<TMachine>[]);

/**
 * What `checkMachine` is given. `events` must list every event to try: XState gives no public
 * way to list a machine's own events. `rules` gives each rule its id, the key it is listed under;
 * the results follow the record's key order, where JavaScript puts integer-like keys first.
 * `input` is the machine's input, used to make the initial snapshot. `maxStates` stops the
 * search before it holds more distinct states than this; the default is 5,000,000.
 */
export interface CheckMachineOptions<TMachine extends AnyStateMachine> {
  readonly events: MachineEvents<TMachine>;
  readonly rules?: Readonly<Record<string, MachineRule<TMachine>>>;
  readonly input?: InputFrom<TMachine>;
  readonly maxStates?: number;
}

/** One event on a path and the snapshot after it. */
export interface MachineStep<TMachine extends AnyStateMachine> {
  readonly event: EventFromLogic<TMachine>;
  readonly snapshot: SnapshotFrom<TMachine>;
}

/**
 * A shortest path from the initial snapshot: `start`, then one step per event. `events` are the
 * events of the steps in order, ready to send to an actor of the machine, and `end` is the last
 * snapshot, `start` when there are no steps.
 */
export interface MachinePath<TMachine extends AnyStateMachine> {
  readonly start: SnapshotFrom<TMachine>;
  readonly steps: readonly MachineStep<TMachine>[];
  readonly events: readonly EventFromLogic<TMachine>[];
  readonly end: SnapshotFrom<TMachine>;
}

/** One rule's result. `path` is the shortest path to a snapshot that breaks it, or `null`. */
export interface MachineRuleResult<TMachine extends AnyStateMachine> {
  readonly id: string;
  readonly says: string;
  readonly verdict: Verdict;
  readonly path: MachinePath<TMachine> | null;
}

/** A problem that is not a broken rule, with the shortest path to the first snapshot showing it. */
export interface MachineFinding<TMachine extends AnyStateMachine> {
  readonly kind: SearchFindingKind;
  readonly message: string;
  readonly path: MachinePath<TMachine>;
}

/**
 * What `checkMachine` found, in the words of `explore` from `@earnestjs/model`. `states` counts
 * distinct states. `findings` holds at most one stuck state. `reason` says why an unfinished
 * check stopped, and is `null` otherwise. `reached` gives the shortest path to every counted
 * state, in the order the search reached them; each path is built as it is asked for.
 */
export interface MachineCheck<TMachine extends AnyStateMachine> {
  readonly status: Status;
  readonly states: number;
  readonly rules: readonly MachineRuleResult<TMachine>[];
  readonly findings: readonly MachineFinding<TMachine>[];
  readonly reason: string | null;
  reached(): Iterable<MachinePath<TMachine>>;
}

type Reached = Arrival<AnyMachineSnapshot, EventObject>;

const STUCK =
  "none of the listed events can happen here, and the machine is not in a top-level final state";

const isPlain = (value: object): boolean => {
  const prototype: unknown = Object.getPrototypeOf(value);
  return prototype === null || Object.getPrototypeOf(prototype) === null;
};

const kindOf = (value: unknown): string =>
  typeof value === "object" && value !== null
    ? value.constructor?.name || Object.prototype.toString.call(value).slice(8, -1)
    : typeof value;

const keyOf = (value: unknown): string => {
  if (typeof value === "number") {
    return value === 0 ? "0" : String(value);
  }
  if (value === undefined || value === null || typeof value === "boolean") {
    return String(value);
  }
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    const items = [...value.keys()].map((index) => (index in value ? keyOf(value[index]) : "hole"));
    return `[${items.join(",")}]`;
  }
  if (typeof value === "object") {
    const shown: unknown =
      "toJSON" in value && typeof value.toJSON === "function" ? value.toJSON() : value;
    if (shown !== value) {
      return `${kindOf(value)}(${keyOf(shown)})`;
    }
    if (Object.getOwnPropertySymbols(value).length > 0) {
      throw new TypeError(
        "checkMachine tells states apart by their context, and cannot read a symbol key in it; keep the context to plain data",
      );
    }
    if (isPlain(value)) {
      const fields = Object.entries(value)
        .filter(([, field]) => field !== undefined)
        .sort(([a], [b]) => compareText(a, b));
      return `{${fields.map(([name, field]) => `${JSON.stringify(name)}:${keyOf(field)}`).join(",")}}`;
    }
  }
  throw new TypeError(
    `checkMachine tells states apart by their context, and cannot read this ${kindOf(value)} in it; keep the context to plain data`,
  );
};

const stateKey = ({ value, context, status, error }: AnyMachineSnapshot): string => {
  if (status === "error") {
    throw error;
  }
  return keyOf({ value, context, status });
};

const eventsToTry = (events: MachineEvents<AnyStateMachine>, snapshot: AnyMachineSnapshot) => {
  if (snapshot.status !== "active") {
    return [];
  }
  const listed = typeof events === "function" ? events(snapshot) : events;
  return listed.filter((event) => snapshot.can(event));
};

const toPath = ({ start, steps }: Trail<Reached, EventObject>): MachinePath<AnyStateMachine> => ({
  start: start.snapshot,
  steps: steps.map(({ move, state }) => ({ event: move, snapshot: state.snapshot })),
  events: steps.map(({ move }) => move),
  end: (steps.at(-1)?.state ?? start).snapshot,
});

const toFinding = ({
  kind,
  message,
  path,
}: SearchFinding<Reached, EventObject>): MachineFinding<AnyStateMachine> => ({
  kind,
  message: kind === "stuck" ? STUCK : message,
  path: toPath(path),
});

const check = (
  machine: AnyStateMachine,
  options: CheckMachineOptions<AnyStateMachine>,
): MachineCheck<AnyStateMachine> => {
  const found = search({
    ...stepping(machine, {
      ...(options.input === undefined ? {} : { input: options.input }),
      events: (snapshot) => eventsToTry(options.events, snapshot),
      serializeState: stateKey,
    }),
    rules: Object.entries(options.rules ?? {}).map(([id, rule]) => ({
      id,
      says: rule.says,
      holds: ({ snapshot }: Reached) => rule.holds(snapshot),
    })),
    isEnd: ({ snapshot }) => snapshot.status === "done",
    ...(options.maxStates === undefined ? {} : { maxStates: options.maxStates }),
  });
  return {
    status: found.status,
    states: found.states,
    rules: found.rules.map(({ id, says, verdict, path }) => ({
      id,
      says,
      verdict,
      path: path === null ? null : toPath(path),
    })),
    findings: found.findings.map(toFinding),
    reason: found.reason,
    *reached() {
      for (const trail of found.reached()) {
        yield toPath(trail);
      }
    },
  };
};

/**
 * Checks an XState v5 machine as Earnest checks a model: it visits every state reachable from
 * the initial snapshot, breadth first, and gives what `explore` from `@earnestjs/model` gives.
 * That is pass, fail or unfinished, the count of distinct states, each rule's verdict with the
 * shortest path of events to a snapshot that breaks it, and the first stuck state. Two
 * snapshots are one state when their state value, context and status are equal, whatever
 * event led there. The context must be plain data: plain objects with string keys, arrays,
 * strings, numbers, booleans, `null`, `undefined`, and values with a `toJSON`, such as a `Date`
 * or an actor ref, read through it. Anything else in it, such as a `Map`, a class instance or a
 * function, is an error. A field set to `undefined` is the same as no field. History, a final
 * state's output and child actors are not part of the state. An event is tried in a snapshot only when `snapshot.can(event)` is true: a guard that fails, or a
 * transition with no target and no action, is no event at all. A snapshot whose status is
 * `done`, a top-level final state, is an end. Any other snapshot where none of the events can
 * happen is stuck. Events are sent through the machine's `transition`: no action of your own
 * runs, except one made with `assign`, and no child actor starts, but the code that computes the next snapshot does, often
 * more than once: guards, `assign`, `enqueueActions` callbacks, invoke `input` and the context
 * of a child machine. Keep that code free of side effects and give it the same result for the
 * same snapshot and event. Every state found is held in memory with its whole snapshot, so a
 * large machine can run out of memory before the default `maxStates`. An
 * error thrown by a guard, an `assign`, the context factory or a rule is thrown by
 * `checkMachine`, also one XState catches into a snapshot whose status is `error`.
 */
export const checkMachine = <TMachine extends AnyStateMachine>(
  machine: TMachine,
  options: NoInfer<CheckMachineOptions<TMachine>>,
): MachineCheck<TMachine> => check(machine, options);
