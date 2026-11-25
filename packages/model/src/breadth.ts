/**
 * `unknown` means the rule was not seen broken, but some state was not judged or not reached:
 * the state limit stopped the search, or a state out of range cut it short. In `explore` a
 * fault in an action or an end also cuts the search short, and a rule is also `unknown` when it
 * could not be evaluated in a state it reached.
 */
export type Verdict = "holds" | "broken" | "unknown";

/**
 * `unfinished` means the state limit stopped the search before anything was found wrong. Every
 * counted state that is not out of range has its rules judged, also the ones the limit kept the
 * search from going on from.
 */
export type Status = "pass" | "fail" | "unfinished";

/** The state limit `explore` and `search` use when given none, the same as `earnest verify`. */
export const DEFAULT_MAX_STATES = 5_000_000;

/** Checks a caller's `maxStates`, or gives `DEFAULT_MAX_STATES` when it is `undefined` or `null`. */
export const stateLimit = (given: number | undefined): number => {
  const maxStates = given ?? DEFAULT_MAX_STATES;
  if (!Number.isInteger(maxStates) || maxStates < 0) {
    throw new Error(`maxStates must be a whole number that is not negative, not ${maxStates}`);
  }
  return maxStates;
};

/** A move that `moves` lists from a state: the move, in the caller's terms, and where it leads. */
export interface SearchMove<State, Move> {
  readonly move: Move;
  readonly next: State;
}

/** A rule `search` judges in every state it counts that is not out of range. */
export interface SearchRule<State> {
  readonly id: string;
  readonly says: string;
  holds(state: State): boolean;
}

/**
 * What `search` is given. `key` must give two states the same text exactly when they are the
 * same state. `moves` lists the moves from a state in the order they are tried. `isEnd` is asked
 * only of a state with no move, and a state it does not mark is stuck; without it every such
 * state is stuck. `outside` gives a message for a state that is out of range, or `null`.
 * `maxStates` stops the search before it holds more distinct states than this; the default is
 * `DEFAULT_MAX_STATES`. An error thrown by any of these functions is thrown by `search`.
 */
export interface SearchInput<State, Move> {
  readonly starts: Iterable<State>;
  key(state: State): string;
  moves(state: State): readonly SearchMove<State, Move>[];
  readonly rules?: readonly SearchRule<State>[];
  isEnd?(state: State): boolean;
  outside?(state: State): string | null;
  readonly maxStates?: number;
}

/** One move on a trail and the state it leads to. */
export interface TrailStep<State, Move> {
  readonly move: Move;
  readonly state: State;
}

/** A start state and the moves that lead from it, each with the state after it. */
export interface Trail<State, Move> {
  readonly start: State;
  readonly steps: readonly TrailStep<State, Move>[];
}

/** One rule's result. `path` is the shortest trail to a state that breaks it, or `null`. */
export interface SearchRuleResult<State, Move> {
  readonly id: string;
  readonly says: string;
  readonly verdict: Verdict;
  readonly path: Trail<State, Move> | null;
}

/** `stuck`: no move and `isEnd` false. `out-of-range`: `outside` gave a message. */
export type SearchFindingKind = "stuck" | "out-of-range";

/** A problem that is not a broken rule, with the shortest trail to the first state showing it. */
export interface SearchFinding<State, Move> {
  readonly kind: SearchFindingKind;
  readonly message: string;
  readonly path: Trail<State, Move>;
}

/**
 * What `search` found, in the words of `explore`. `states` counts distinct states, out-of-range
 * ones included. `findings` holds at most one of each kind, stuck first. `reason` says why an
 * unfinished search stopped, and is `null` otherwise. `reached` gives the shortest trail to
 * every counted state, in the order the search reached them; each trail is built as it is
 * asked for.
 */
export interface SearchResult<State, Move> {
  readonly status: Status;
  readonly states: number;
  readonly rules: readonly SearchRuleResult<State, Move>[];
  readonly findings: readonly SearchFinding<State, Move>[];
  readonly reason: string | null;
  reached(): Iterable<Trail<State, Move>>;
}

/** A counted state and the move it was first reached by from the state before it. */
export interface Visit<State, Move> {
  readonly state: State;
  readonly parent: { readonly from: Visit<State, Move>; readonly move: Move } | null;
}

/**
 * What a breadth-first search does in each counted state, in the order they are counted.
 * `expand` judges a state the search goes on from and gives its moves; `judge` judges one the
 * state limit kept the search from going on from.
 */
export interface Visitor<State, Move> {
  expand(at: Visit<State, Move>): Iterable<SearchMove<State, Move>>;
  judge(at: Visit<State, Move>): void;
}

/** Every counted state in the order it was counted, and why the search stopped early, or `null`. */
export interface Reach<State, Move> {
  readonly visits: readonly Visit<State, Move>[];
  readonly reason: string | null;
}

/**
 * The one breadth-first loop that `search` and `explore` share. States with the same `key` are
 * one state, counted once. When counting a state would pass `maxStates`, the search stops going
 * on, and every counted state not yet expanded is judged.
 */
export const breadthFirst = <State, Move>(
  starts: Iterable<State>,
  key: (state: State) => string,
  maxStates: number,
  visitor: Visitor<State, Move>,
): Reach<State, Move> => {
  const seen = new Set<string>();
  const visits: Visit<State, Move>[] = [];
  const hasRoom = (state: State, parent: Visit<State, Move>["parent"]): boolean => {
    const found = key(state);
    if (seen.has(found)) {
      return true;
    }
    if (seen.size >= maxStates) {
      return false;
    }
    seen.add(found);
    visits.push({ state, parent });
    return true;
  };
  let isWithin = true;
  for (const start of starts) {
    isWithin = hasRoom(start, null);
    if (!isWithin) {
      break;
    }
  }
  let expanded = 0;
  for (const from of visits) {
    if (!isWithin) {
      break;
    }
    expanded += 1;
    for (const { move, next } of visitor.expand(from)) {
      isWithin = hasRoom(next, { from, move });
      if (!isWithin) {
        break;
      }
    }
  }
  for (const counted of visits.slice(expanded)) {
    visitor.judge(counted);
  }
  return { visits, reason: isWithin ? null : `state limit of ${maxStates}` };
};

/** The shortest trail to a counted state, from the start state it was reached from. */
export const trailTo = <State, Move>(target: Visit<State, Move>): Trail<State, Move> => {
  const steps: TrailStep<State, Move>[] = [];
  let at = target;
  while (at.parent !== null) {
    steps.push({ move: at.parent.move, state: at.state });
    at = at.parent.from;
  }
  return { start: at.state, steps: steps.reverse() };
};

const STUCK = "no move can happen here, and the caller does not mark it as an end";

class Judge<State, Move> implements Visitor<State, Move> {
  readonly #input: SearchInput<State, Move>;
  readonly #rules: readonly SearchRule<State>[];
  readonly #firstBroken = new Map<number, Visit<State, Move>>();
  #stuck: Visit<State, Move> | null = null;
  #outside: { readonly at: Visit<State, Move>; readonly message: string } | null = null;

  constructor(input: SearchInput<State, Move>) {
    this.#input = input;
    this.#rules = input.rules ?? [];
  }

  expand(at: Visit<State, Move>): readonly SearchMove<State, Move>[] {
    if (!this.#isInRange(at)) {
      return [];
    }
    const moves = this.#input.moves(at.state);
    if (moves.length === 0 && !(this.#input.isEnd?.(at.state) ?? false)) {
      this.#stuck ??= at;
    }
    return moves;
  }

  judge(at: Visit<State, Move>): void {
    this.#isInRange(at);
  }

  #isInRange(at: Visit<State, Move>): boolean {
    const message = this.#input.outside?.(at.state) ?? null;
    if (message !== null) {
      this.#outside ??= { at, message };
      return false;
    }
    for (const [index, rule] of this.#rules.entries()) {
      if (!rule.holds(at.state) && !this.#firstBroken.has(index)) {
        this.#firstBroken.set(index, at);
      }
    }
    return true;
  }

  outcome({ visits, reason }: Reach<State, Move>): SearchResult<State, Move> {
    const findings: SearchFinding<State, Move>[] = [];
    if (this.#stuck !== null) {
      findings.push({ kind: "stuck", message: STUCK, path: trailTo(this.#stuck) });
    }
    if (this.#outside !== null) {
      const { at, message } = this.#outside;
      findings.push({ kind: "out-of-range", message, path: trailTo(at) });
    }
    const unbroken = reason === null && this.#outside === null ? "holds" : "unknown";
    const rules = this.#rules.map((rule, index): SearchRuleResult<State, Move> => {
      const first = this.#firstBroken.get(index);
      return {
        id: rule.id,
        says: rule.says,
        verdict: first === undefined ? unbroken : "broken",
        path: first === undefined ? null : trailTo(first),
      };
    });
    const hasFailed = findings.length > 0 || rules.some((rule) => rule.verdict === "broken");
    const unfailed = reason === null ? "pass" : "unfinished";
    return {
      status: hasFailed ? "fail" : unfailed,
      states: visits.length,
      rules,
      findings,
      reason,
      *reached() {
        for (const visit of visits) {
          yield trailTo(visit);
        }
      },
    };
  }
}

/**
 * Searches every state reachable from `starts` through `moves`, breadth first, for states that
 * are not models: a plain object, a snapshot of a state machine, anything `key` can name. It
 * gives what `explore` gives, with trails of the caller's own moves and states as paths: the
 * count of distinct states, each rule's verdict with the shortest trail to a state that breaks
 * it, the first stuck and out-of-range states, and the shortest trail to every state it
 * reached. States are judged in the order they are reached: start states in the order given,
 * then the moves of each state in the order `moves` lists them.
 */
export const search = <State, Move>(input: SearchInput<State, Move>): SearchResult<State, Move> => {
  const judge = new Judge(input);
  return judge.outcome(
    breadthFirst(input.starts, (state) => input.key(state), stateLimit(input.maxStates), judge),
  );
};
