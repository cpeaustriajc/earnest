import {
  type Reach,
  type SearchMove,
  type Status,
  type Verdict,
  type Visit,
  type Visitor,
  breadthFirst,
  stateLimit,
  trailTo,
} from "./breadth.ts";
import { type Declaration, rowsOf } from "./declarations.ts";
import type { ActionRunner, AnyModel, RuleRunner, Runners } from "./model.ts";
import {
  type Json,
  NullFault,
  DivisionFault,
  RemainderFault,
  type Row,
  isJsonList,
  keyOf,
  toJson,
} from "./values.ts";

/** A field or an input with its value in JSON form. */
export interface Named {
  readonly name: string;
  readonly value: Json;
}

/** A field an action changed, with its value before and after. */
export interface Change {
  readonly field: string;
  readonly before: Json;
  readonly after: Json;
}

/**
 * One action on a path: its id, its sentence as `label`, the input it was tried with, the
 * fields it changed and the whole state after it.
 */
export interface Step {
  readonly action: string;
  readonly label: string;
  readonly input: readonly Named[];
  readonly changes: readonly Change[];
  readonly state: readonly Named[];
}

/** A start state and the actions that lead from it, as `earnest verify --json` prints a path. */
export interface Path {
  readonly start: readonly Named[];
  readonly steps: readonly Step[];
}

/** One rule's result. `path` is the shortest path to a state that breaks it, or `null`. */
export interface RuleResult {
  readonly id: string;
  readonly says: string;
  readonly verdict: Verdict;
  readonly path: Path | null;
}

/**
 * `stuck`: the condition of every action is false and so is every end. `out-of-range`: an action
 * gave a field a value outside its declaration. `fault`: a rule, the condition or the change of an
 * action, or an end can't be evaluated in the state. A state where a condition or an end can't be
 * evaluated is not stuck: nothing says that it should stop there or go on.
 */
export type FindingKind = "stuck" | "out-of-range" | "fault";

/** A problem that is not a broken rule, with the shortest path to the first state showing it. */
export interface Finding {
  readonly kind: FindingKind;
  readonly message: string;
  readonly path: Path;
}

/**
 * What `explore` found. `states` counts distinct states, out-of-range ones included. `findings`
 * holds at most one of each kind, in the order stuck, out-of-range, fault. `reason` says why an
 * unfinished search stopped, and is `null` otherwise.
 */
export interface Exploration {
  readonly status: Status;
  readonly states: number;
  readonly rules: readonly RuleResult[];
  readonly findings: readonly Finding[];
  readonly reason: string | null;
}

/** The exports of a model file, by name: `import * as cart from "./cart.model.ts"`. */
export type Exports = Readonly<Record<string, unknown>>;

/**
 * `maxStates` stops the search before it holds more distinct states than this; the default is
 * 5,000,000. `exports` is the model file's exports, which give each action, rule and end its
 * id: the name it is exported under. One that is not among them has its sentence as its id.
 */
export interface ExploreOptions {
  readonly maxStates?: number;
  readonly exports?: Exports;
}

/**
 * Gives the function that names a part of a model: the name it is exported under in `exports`,
 * or `fallback`, its sentence, when it is not exported there.
 */
export const namer = (exports: Exports = {}): ((part: object, fallback: string) => string) => {
  const names = new Map<unknown, string>();
  for (const [name, exported] of Object.entries(exports)) {
    if (!names.has(exported)) {
      names.set(exported, name);
    }
  }
  return (part, fallback) => names.get(part) ?? fallback;
};

const NULL_READ = "reads a value that is null";
const OVERFLOW = "computes a number too large to hold";
const REMAINDER = "takes a remainder with a negative number or a divisor below 1";
const DIVISION = "takes a division with a negative number or a divisor below 1";
const WRONG_KIND = "uses a value of the wrong kind";

/** What `attempt` gives: the result, or the sentence part saying why there is none. */
export type Attempt<T> =
  | { readonly fault: null; readonly value: T }
  | { readonly fault: string; readonly value: null };

/**
 * Runs part of a model and turns a fault into a value: reading a field of `null`, or using a
 * value as something it is not. Any other error is a mistake in the model file and is thrown.
 */
export const attempt = <T>(run: () => T): Attempt<T> => {
  try {
    return { fault: null, value: run() };
  } catch (thrown) {
    if (thrown instanceof RemainderFault) {
      return { fault: REMAINDER, value: null };
    }
    if (thrown instanceof DivisionFault) {
      return { fault: DIVISION, value: null };
    }
    if (thrown instanceof NullFault) {
      return { fault: NULL_READ, value: null };
    }
    if (thrown instanceof TypeError) {
      return { fault: /\bnull\b/.test(thrown.message) ? NULL_READ : WRONG_KIND, value: null };
    }
    throw thrown;
  }
};

const numberFault = (json: Json): string | null => {
  if (typeof json === "number") {
    if (Number.isNaN(json)) {
      return REMAINDER;
    }
    return Math.abs(json) > Number.MAX_SAFE_INTEGER ? OVERFLOW : null;
  }
  if (json === null || typeof json !== "object") {
    return null;
  }
  const inside = isJsonList(json) ? json : Object.values(json);
  return inside.map(numberFault).find((fault) => fault !== null) ?? null;
};

/**
 * Applies an action and gives the next state, or the fault that stops it. Besides the faults of
 * `attempt`, a next state that holds `NaN`, which `%` by zero gives, or a number too large for
 * JavaScript to hold exactly is a fault.
 */
export const nextState = (action: ActionRunner, state: Row, input: Row): Attempt<Row> => {
  const tried = attempt(() => action.changes(state, input));
  if (tried.fault !== null) {
    return tried;
  }
  const fault = numberFault(toJson(tried.value));
  return fault === null ? tried : { fault, value: null };
};

/** Every input an action is tried with, in the checker's order: the first input varies fastest. */
export const inputsOf = (action: ActionRunner): readonly Row[] => rowsOf(action.inputs);

interface Taken {
  readonly action: ActionRunner;
  readonly input: Row;
}

type Visited = Visit<Row, Taken>;

interface Expanded {
  readonly outside: string | null;
  readonly broken: RuleRunner[];
  readonly unjudged: RuleRunner[];
  readonly successors: SearchMove<Row, Taken>[];
  trouble: string | null;
  isCutShort: boolean;
  isEnabled: boolean;
  isStuck: boolean;
}

type FieldList = readonly (readonly [string, Declaration<unknown, never>])[];

class Search implements Visitor<Row, Taken> {
  readonly #fields: FieldList;
  readonly #parts: Runners;
  readonly #inputs: ReadonlyMap<ActionRunner, readonly Row[]>;
  readonly #name: (part: object, fallback: string) => string;
  readonly #firstBroken = new Map<RuleRunner, Visited>();
  readonly #unjudged = new Set<RuleRunner>();
  #stuck: Visited | null = null;
  #outside: { readonly at: Visited; readonly field: string } | null = null;
  #trouble: { readonly at: Visited; readonly message: string } | null = null;
  #isCutShort = false;

  constructor(fields: FieldList, parts: Runners, options: ExploreOptions) {
    this.#fields = fields;
    this.#parts = parts;
    this.#inputs = new Map(parts.actions.map((action) => [action, inputsOf(action)]));
    this.#name = namer(options.exports);
  }

  expand(at: Visited): readonly SearchMove<Row, Taken>[] {
    const expanded = this.#expand(at.state);
    this.#record(at, expanded);
    return expanded.successors;
  }

  judge(at: Visited): void {
    this.#record(at, this.#judge(at.state));
  }

  #judge(state: Row): Expanded {
    const outside = this.#fields.find(([name, declared]) => !declared.has(state[name]));
    const found: Expanded = {
      outside: outside === undefined ? null : outside[0],
      broken: [],
      unjudged: [],
      successors: [],
      trouble: null,
      isCutShort: outside !== undefined,
      isEnabled: false,
      isStuck: false,
    };
    if (found.outside !== null) {
      return found;
    }
    for (const rule of this.#parts.rules) {
      const held = attempt(() => rule.holds(state));
      if (held.fault !== null) {
        found.trouble ??= `the rule \`${this.#name(rule.part, rule.says)}\` ${held.fault}`;
        found.unjudged.push(rule);
      } else if (!held.value) {
        found.broken.push(rule);
      }
    }
    return found;
  }

  #expand(state: Row): Expanded {
    const found = this.#judge(state);
    if (found.outside !== null) {
      return found;
    }
    let canTell = true;
    for (const action of this.#parts.actions) {
      canTell = this.#expandAction(action, state, found) && canTell;
    }
    if (found.isEnabled) {
      return found;
    }
    let hasEnded = false;
    for (const end of this.#parts.ends) {
      const ended = attempt(() => end.when(state));
      if (ended.fault === null) {
        hasEnded ||= ended.value;
      } else {
        found.trouble ??= `the end \`${this.#name(end.part, end.says)}\` ${ended.fault}`;
        found.isCutShort = true;
        canTell = false;
      }
    }
    found.isStuck = canTell && !hasEnded;
    return found;
  }

  #expandAction(action: ActionRunner, state: Row, found: Expanded): boolean {
    const id = this.#name(action.part, action.sentence);
    let canTell = true;
    for (const input of this.#inputs.get(action) ?? []) {
      const can = attempt(() => action.when(state, input));
      if (can.fault !== null) {
        found.trouble ??= `the condition of the action \`${id}\` ${can.fault}`;
        found.isCutShort = true;
        canTell = false;
        continue;
      }
      if (!can.value) {
        continue;
      }
      found.isEnabled = true;
      const next = nextState(action, state, input);
      if (next.fault === null) {
        found.successors.push({ move: { action, input }, next: next.value });
      } else {
        found.trouble ??= `the change of the action \`${id}\` ${next.fault}`;
        found.isCutShort = true;
      }
    }
    return canTell;
  }

  #record(at: Visited, expanded: Expanded): void {
    if (expanded.outside !== null) {
      this.#outside ??= { at, field: expanded.outside };
    }
    for (const rule of expanded.broken) {
      if (!this.#firstBroken.has(rule)) {
        this.#firstBroken.set(rule, at);
      }
    }
    for (const rule of expanded.unjudged) {
      this.#unjudged.add(rule);
    }
    if (expanded.trouble !== null) {
      this.#trouble ??= { at, message: `${expanded.trouble} in this state` };
    }
    this.#isCutShort ||= expanded.isCutShort;
    if (expanded.isStuck) {
      this.#stuck ??= at;
    }
  }

  #named(row: Row): Named[] {
    return this.#fields.map(([name]) => ({ name, value: toJson(row[name]) }));
  }

  #step({ action, input }: Taken, before: Row, after: Row): Step {
    return {
      action: this.#name(action.part, action.sentence),
      label: action.sentence,
      input: action.inputs.map(([name]) => ({ name, value: toJson(input[name]) })),
      changes: this.#fields
        .filter(([name]) => keyOf(before[name]) !== keyOf(after[name]))
        .map(([name]) => ({
          field: name,
          before: toJson(before[name]),
          after: toJson(after[name]),
        })),
      state: this.#named(after),
    };
  }

  #pathTo(target: Visited): Path {
    const { start, steps } = trailTo(target);
    return {
      start: this.#named(start),
      steps: steps.map(({ move, state }, index) =>
        this.#step(move, steps[index - 1]?.state ?? start, state),
      ),
    };
  }

  #findings(): Finding[] {
    const findings: Finding[] = [];
    if (this.#stuck !== null) {
      findings.push({
        kind: "stuck",
        message: "no action can happen here, and the model does not mark it as an end",
        path: this.#pathTo(this.#stuck),
      });
    }
    if (this.#outside !== null) {
      findings.push({
        kind: "out-of-range",
        message: `the field \`${this.#outside.field}\` leaves the values it declares`,
        path: this.#pathTo(this.#outside.at),
      });
    }
    if (this.#trouble !== null) {
      findings.push({
        kind: "fault",
        message: this.#trouble.message,
        path: this.#pathTo(this.#trouble.at),
      });
    }
    return findings;
  }

  outcome({ visits, reason }: Reach<Row, Taken>): Exploration {
    const rules = this.#parts.rules.map((rule): RuleResult => {
      const first = this.#firstBroken.get(rule);
      const isJudged = reason === null && !this.#isCutShort && !this.#unjudged.has(rule);
      const unbroken = isJudged ? "holds" : "unknown";
      return {
        id: this.#name(rule.part, rule.says),
        says: rule.says,
        verdict: first === undefined ? unbroken : "broken",
        path: first === undefined ? null : this.#pathTo(first),
      };
    });
    const findings = this.#findings();
    const hasFailed = findings.length > 0 || rules.some((rule) => rule.verdict === "broken");
    const unfailed = reason === null ? "pass" : "unfinished";
    return {
      status: hasFailed ? "fail" : unfailed,
      states: visits.length,
      rules,
      findings,
      reason,
    };
  }
}

/**
 * Searches every state a model can reach, breadth first, and gives the count of distinct
 * states, each rule's verdict with the shortest path to a state that breaks it, and the first
 * stuck, out-of-range and fault findings. An end is asked only in a state where no action can
 * happen. It visits states in the same order as
 * `earnest verify`, so the two results can be compared. Pass the model file's exports as
 * `exports` to get the same ids.
 */
export const explore = (model: AnyModel, options: ExploreOptions = {}): Exploration => {
  const maxStates = stateLimit(options.maxStates);
  const fields = Object.entries(model.fields);
  const found = new Search(fields, model.runners(), options);
  const key = (row: Row): string => fields.map(([name]) => keyOf(row[name])).join(";");
  return found.outcome(breadthFirst(model.starts(), key, maxStates, found));
};
