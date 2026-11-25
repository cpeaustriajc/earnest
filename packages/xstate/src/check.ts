import {
  type Declaration,
  type SearchFinding,
  type SearchFindingKind,
  type Status,
  type Trail,
  type Verdict,
  combinationsOf,
  compareText,
  search,
} from "@earnestjs/model";
import type {
  AnyMachineSnapshot,
  AnyStateMachine,
  ContextFrom,
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

type EventOfType<TMachine extends AnyStateMachine, Type> = Extract<
  EventFromLogic<TMachine>,
  { readonly type: Type }
>;

/**
 * A declaration written with `m` from `@earnestjs/model` for each payload field of one event
 * type, every field but `type`. Only declarations of plain JavaScript values are taken: booleans,
 * whole numbers, texts and other `m.enum` or `m.oneOf` values, `.optional()` and `m.record` of
 * those. When several event shapes share a type, the schema declares the fields of one of them.
 * A record payload arrives frozen, as `m.record` gives it, and the same object is sent from every
 * snapshot, so the machine must copy it rather than change it in place.
 */
export type MachinePayloadSchema<TEvent extends EventObject> = TEvent extends unknown
  ? {
      readonly [Field in keyof Omit<TEvent, "type">]: Declaration<
        Omit<TEvent, "type">[Field],
        never
      >;
    }
  : never;

/**
 * The event types to try, each with a declaration for each of its payload fields, such as
 * `{ PAY: { coins: m.int(1, 2) }, PUSH: {} }`. A type is tried with every combination of the
 * values its fields declare, the first field varying fastest, as `explore` tries an action's
 * inputs; the event sent is `{ type, ...payload }`. Types are tried in the record's key order,
 * where JavaScript puts integer-like keys first. A type with more than 1,000,000 payloads is a
 * `RangeError`.
 */
export type MachineEventSchema<TMachine extends AnyStateMachine> = {
  readonly [Type in EventFromLogic<TMachine>["type"]]?: MachinePayloadSchema<
    EventOfType<TMachine, Type>
  >;
};

/**
 * The events to try in every snapshot: a list, or a function of the snapshot that gives one, as
 * in the traversal options of `@xstate/graph`, or a `MachineEventSchema` that declares each
 * event type's payload.
 */
export type MachineEvents<TMachine extends AnyStateMachine> =
  | readonly EventFromLogic<TMachine>[]
  | ((snapshot: SnapshotFrom<TMachine>) => readonly EventFromLogic<TMachine>[])
  | MachineEventSchema<TMachine>;

/**
 * A declaration written with `m` from `@earnestjs/model` for some fields of the machine's
 * context, such as `{ visitors: m.int(0, 3) }`. Only declarations of plain JavaScript values are
 * taken, as in `MachinePayloadSchema`. Fields it does not name are not checked.
 */
export type MachineContextSchema<TMachine extends AnyStateMachine> = {
  readonly [Field in keyof ContextFrom<TMachine>]?: Declaration<
    ContextFrom<TMachine>[Field],
    never
  >;
};

/**
 * What `checkMachine` is given. `events` must list every event to try: XState gives no public
 * way to list a machine's own events. `rules` gives each rule its id, the key it is listed under;
 * the results follow the record's key order, where JavaScript puts integer-like keys first.
 * `input` is the machine's input, used to make the initial snapshot. `maxStates` stops the
 * search before it holds more distinct states than this; the default is 5,000,000. `context`
 * bounds the search: a snapshot whose context has a value outside what `context` declares for
 * it, `undefined` and a missing field included, is out of range. It is counted, no rule is
 * judged in it and nothing is searched from it, as `explore` does with a model's state out of
 * range.
 */
export interface CheckMachineOptions<TMachine extends AnyStateMachine> {
  readonly events: MachineEvents<TMachine>;
  readonly rules?: Readonly<Record<string, MachineRule<TMachine>>>;
  readonly input?: InputFrom<TMachine>;
  readonly maxStates?: number;
  readonly context?: MachineContextSchema<TMachine>;
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
 * distinct states, out-of-range ones included. `findings` holds at most one stuck state, then at
 * most one state out of range. `reason` says why an unfinished check stopped, and is `null`
 * otherwise. `reached` gives the shortest path to every counted state, in the order the search
 * reached them; each path is built as it is asked for.
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

type Declared = Declaration<unknown, never>;

type Schema = Readonly<Record<string, Declared | undefined>>;

const article = (word: string): string => (/^[aeiou]/.test(word) ? `an ${word}` : `a ${word}`);

const plainFields = (option: string, schema: Schema): readonly (readonly [string, Declared])[] =>
  Object.entries(schema).flatMap(([name, declared]) => {
    if (declared === undefined) {
      return [];
    }
    const held: string | null | undefined = declared.libraryKind;
    if (held === undefined) {
      throw new TypeError(
        `checkMachine can't take ${option}.${name}: it was made with an @earnestjs/model older than the one @earnestjs/xstate uses; install the version it depends on`,
      );
    }
    if (held !== null) {
      const holding = held === declared.kind ? "" : ` that holds ${article(held)}`;
      throw new TypeError(
        `checkMachine can't take ${option}.${name}, ${article(declared.kind)} declaration${holding}: its values are objects of @earnestjs/model, not plain data an XState context or event holds; declare booleans, numbers, texts, optionals and records of those`,
      );
    }
    return [[name, declared] as const];
  });

type EventList = (snapshot: AnyMachineSnapshot) => readonly EventObject[];

const eventListOf = (events: MachineEvents<AnyStateMachine>): EventList => {
  if (typeof events === "function") {
    return events;
  }
  if (Array.isArray(events)) {
    return () => events;
  }
  const declared: readonly (readonly [string, Schema | undefined])[] = Object.entries(events);
  const listed = declared.flatMap(([type, payload]) => {
    if (payload === undefined) {
      return [];
    }
    if (Object.hasOwn(payload, "type")) {
      throw new TypeError(
        `checkMachine can't take events.${type}.type: an event's type is its key in events`,
      );
    }
    const fields = Object.fromEntries(plainFields(`events.${type}`, payload));
    try {
      return combinationsOf(fields).map((row) => ({ type, ...row }));
    } catch (error) {
      throw new RangeError(
        `checkMachine can't list the payloads of events.${type}: ${error instanceof Error ? error.message : String(error)}`,
        { cause: error },
      );
    }
  });
  return () => listed;
};

const eventsToTry = (events: EventList, snapshot: AnyMachineSnapshot) => {
  if (snapshot.status !== "active") {
    return [];
  }
  return events(snapshot).filter((event) => snapshot.can(event));
};

const withoutUndefined = (value: unknown): unknown =>
  typeof value === "object" && value !== null && !Array.isArray(value) && isPlain(value)
    ? Object.fromEntries(
        Object.entries(value).flatMap(([name, field]) =>
          field === undefined ? [] : [[name, withoutUndefined(field)]],
        ),
      )
    : value;

const outsideOf =
  (fields: readonly (readonly [string, Declared])[]) =>
  ({ snapshot }: Reached): string | null => {
    const context: unknown = snapshot.context;
    const field = fields.find(
      ([name, declared]) =>
        !declared.has(
          withoutUndefined(
            typeof context === "object" && context !== null
              ? Reflect.get(context, name)
              : undefined,
          ),
        ),
    );
    return field === undefined
      ? null
      : `the context field \`${field[0]}\` leaves the values it declares`;
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
  const events = eventListOf(options.events);
  const context = options.context === undefined ? null : plainFields("context", options.context);
  const found = search({
    ...stepping(machine, {
      ...(options.input === undefined ? {} : { input: options.input }),
      events: (snapshot) => eventsToTry(events, snapshot),
      serializeState: stateKey,
    }),
    rules: Object.entries(options.rules ?? {}).map(([id, rule]) => ({
      id,
      says: rule.says,
      holds: ({ snapshot }: Reached) => rule.holds(snapshot),
    })),
    isEnd: ({ snapshot }) => snapshot.status === "done",
    ...(context === null ? {} : { outside: outsideOf(context) }),
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
 * `checkMachine`, also one XState catches into a snapshot whose status is `error`. A
 * declaration in `context` or in an event schema whose values are not plain JavaScript, such as
 * `m.set(...)`, is a `TypeError` before the search starts, naming the option and the field.
 */
export const checkMachine = <TMachine extends AnyStateMachine>(
  machine: TMachine,
  options: NoInfer<CheckMachineOptions<TMachine>>,
): MachineCheck<TMachine> => check(machine, options);
