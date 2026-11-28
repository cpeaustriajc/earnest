/** One way a state changes into another: the positions of both states and of the action. */
export interface Edge {
  readonly from: number;
  readonly to: number;
  readonly action: number;
}

/**
 * The states a search visited and the actions between them, for judging a rule about runs.
 * `edges` leaves out an action that keeps the state as it is, lists each edge once and is sorted
 * by `from`, then `to`, then `action`. `fair` holds the positions of the actions that are not put
 * off forever. `parents` gives the state each state was first reached from, or `null` for a start
 * state, and `depth` the number of steps of that shortest way.
 */
export interface Graph {
  readonly states: number;
  readonly edges: readonly Edge[];
  readonly fair: readonly number[];
  readonly parents: readonly (number | null)[];
  readonly depth: readonly number[];
}

/**
 * A run that never reaches the state a rule wants: `states` from a start state on, with the
 * action taken into each state in `actions` (`null` for the first one that leads there, and for
 * the start state). The steps from `repeatsFrom` on can repeat forever; `null` means the run
 * stays in the last state.
 */
export interface Lasso {
  readonly states: number[];
  readonly actions: (number | null)[];
  repeatsFrom: number | null;
}

const NOWHERE = -1;

class Outgoing {
  readonly #first: number[];
  readonly #edges: readonly Edge[];

  constructor(states: number, edges: readonly Edge[]) {
    this.#edges = edges;
    this.#first = Array.from({ length: states + 1 }, () => 0);
    for (const edge of edges) {
      this.#first[edge.from + 1] = (this.#first[edge.from + 1] ?? 0) + 1;
    }
    let total = 0;
    for (const [position, count] of this.#first.entries()) {
      total += count;
      this.#first[position] = total;
    }
  }

  of(from: number): readonly Edge[] {
    return this.#edges.slice(this.#first[from] ?? 0, this.#first[from + 1] ?? 0);
  }

  at(from: number, position: number): Edge | undefined {
    const index = (this.#first[from] ?? 0) + position;
    return index < (this.#first[from + 1] ?? 0) ? this.#edges[index] : undefined;
  }

  firstAction(from: number, to: number): number | null {
    return this.of(from).find((edge) => edge.to === to)?.action ?? null;
  }
}

interface Groups {
  readonly of: number[];
  readonly sizes: number[];
}

const groupsOf = (out: Outgoing, inside: readonly boolean[]): Groups => {
  const states = inside.length;
  const of = Array.from({ length: states }, () => NOWHERE);
  const sizes: number[] = [];
  const order = Array.from({ length: states }, () => NOWHERE);
  const low = Array.from({ length: states }, () => NOWHERE);
  const held: number[] = [];
  const onHold = Array.from({ length: states }, () => false);
  const walk: [number, number][] = [];
  let nextOrder = 0;
  for (let root = 0; root < states; root += 1) {
    if (inside[root] !== true || order[root] !== NOWHERE) {
      continue;
    }
    walk.push([root, 0]);
    for (let top = walk.pop(); top !== undefined; top = walk.pop()) {
      const [state, tried] = top;
      if (tried === 0) {
        order[state] = nextOrder;
        low[state] = nextOrder;
        nextOrder += 1;
        held.push(state);
        onHold[state] = true;
      }
      const edge = out.at(state, tried);
      if (edge !== undefined) {
        walk.push([state, tried + 1]);
        if (inside[edge.to] !== true) {
          continue;
        }
        if (order[edge.to] === NOWHERE) {
          walk.push([edge.to, 0]);
        } else if (onHold[edge.to] === true) {
          low[state] = Math.min(low[state] ?? 0, order[edge.to] ?? 0);
        }
        continue;
      }
      if (low[state] === order[state]) {
        const group = sizes.length;
        let size = 0;
        for (let member = held.pop(); member !== undefined; member = held.pop()) {
          onHold[member] = false;
          of[member] = group;
          size += 1;
          if (member === state) {
            break;
          }
        }
        sizes.push(size);
      }
      const parent = walk.at(-1);
      if (parent !== undefined) {
        low[parent[0]] = Math.min(low[parent[0]] ?? 0, low[state] ?? 0);
      }
    }
  }
  return { of, sizes };
};

class Hunt {
  readonly #graph: Graph;
  readonly #out: Outgoing;
  readonly #possible: readonly boolean[][];

  constructor(graph: Graph) {
    this.#graph = graph;
    this.#out = new Outgoing(graph.states, graph.edges);
    this.#possible = graph.fair.map((action) => {
      const flags = Array.from({ length: graph.states }, () => false);
      for (const edge of graph.edges) {
        if (edge.action === action) {
          flags[edge.from] = true;
        }
      }
      return flags;
    });
  }

  #can(number: number, state: number): boolean {
    return this.#possible[number]?.[state] === true;
  }

  #doomed(inside: readonly boolean[], groups: Groups): boolean[] {
    const { fair } = this.#graph;
    const flagsPerGroup = (): boolean[][] =>
      fair.map(() => Array.from({ length: groups.sizes.length }, () => false));
    const putOff = flagsPerGroup();
    const taken = flagsPerGroup();
    for (const [state, group] of groups.of.entries()) {
      if (group === NOWHERE) {
        continue;
      }
      for (const [number, action] of fair.entries()) {
        if (!this.#can(number, state)) {
          (putOff[number] ?? [])[group] = true;
        }
        const insideTheGroup = this.#out
          .of(state)
          .some((edge) => edge.action === action && groups.of[edge.to] === group);
        if (insideTheGroup) {
          (taken[number] ?? [])[group] = true;
        }
      }
    }
    return inside.map((_, state) => {
      const group = groups.of[state] ?? NOWHERE;
      if (group === NOWHERE) {
        return false;
      }
      if ((groups.sizes[group] ?? 0) < 2) {
        return this.#out.at(state, 0) === undefined;
      }
      return fair.every(
        (_action, number) => putOff[number]?.[group] === true || taken[number]?.[group] === true,
      );
    });
  }

  #wayIn(
    inside: readonly boolean[],
    triggers: readonly boolean[],
    doomed: readonly boolean[],
  ): number[] | null {
    const { depth } = this.#graph;
    const depthOf = (state: number): number => depth[state] ?? 0;
    const sources = inside
      .map((_, state) => state)
      .filter((state) => triggers[state] === true && inside[state] === true)
      .sort((one, other) => depthOf(one) - depthOf(other) || one - other);
    let nextSource = 0;
    const seen = inside.map(() => false);
    const via = inside.map(() => NOWHERE);
    const injectUpTo = (level: number, into: number[]): void => {
      for (
        let source = sources[nextSource];
        source !== undefined && depthOf(source) <= level;
        source = sources[nextSource]
      ) {
        nextSource += 1;
        if (seen[source] !== true) {
          seen[source] = true;
          into.push(source);
        }
      }
    };
    const first = sources[0];
    if (first === undefined) {
      return null;
    }
    let level = depthOf(first);
    let frontier: number[] = [];
    for (;;) {
      injectUpTo(level, frontier);
      frontier.sort((one, other) => one - other);
      const found = frontier.find((state) => doomed[state] === true);
      if (found !== undefined) {
        const chain = [found];
        for (let state = via[found] ?? NOWHERE; state !== NOWHERE; state = via[state] ?? NOWHERE) {
          chain.push(state);
        }
        return chain.reverse();
      }
      const next: number[] = [];
      injectUpTo(level + 1, next);
      for (const state of frontier) {
        for (const edge of this.#out.of(state)) {
          if (inside[edge.to] === true && seen[edge.to] !== true) {
            seen[edge.to] = true;
            via[edge.to] = state;
            next.push(edge.to);
          }
        }
      }
      if (next.length === 0) {
        const waiting = sources[nextSource];
        if (waiting === undefined) {
          return null;
        }
        level = depthOf(waiting);
      } else {
        level += 1;
      }
      frontier = next;
    }
  }

  #route(
    from: number,
    group: number,
    groups: Groups,
    arrived: (state: number) => boolean,
  ): number[] | null {
    const via = new Map<number, number>();
    const queue = [from];
    for (let head = 0; head < queue.length; head += 1) {
      const state = queue[head] ?? from;
      for (const edge of this.#out.of(state)) {
        if (groups.of[edge.to] !== group) {
          continue;
        }
        if (arrived(edge.to)) {
          const chain = [edge.to, state];
          for (let before = state; before !== from;) {
            const earlier = via.get(before);
            if (earlier === undefined) {
              return null;
            }
            before = earlier;
            chain.push(before);
          }
          return chain.reverse();
        }
        if (edge.to !== from && !via.has(edge.to)) {
          via.set(edge.to, state);
          queue.push(edge.to);
        }
      }
    }
    return null;
  }

  #extend(lasso: Lasso, chain: readonly number[]): void {
    for (const [position, to] of chain.entries()) {
      const from = chain[position - 1];
      if (from !== undefined) {
        lasso.states.push(to);
        lasso.actions.push(this.#out.firstAction(from, to));
      }
    }
  }

  #turn(lasso: Lasso, groups: Groups): boolean {
    const home = lasso.states.at(-1);
    if (home === undefined) {
      return false;
    }
    const group = groups.of[home] ?? NOWHERE;
    const begin = lasso.states.length - 1;
    for (const [number, action] of this.#graph.fair.entries()) {
      const here = lasso.states.at(-1) ?? home;
      const waited = lasso.states.slice(begin).some((state) => !this.#can(number, state));
      const happened = lasso.actions.slice(begin + 1).includes(action);
      if (waited || happened) {
        continue;
      }
      const waitsSomewhere = groups.of.some(
        (found, state) => found === group && !this.#can(number, state),
      );
      if (waitsSomewhere) {
        const chain = this.#route(here, group, groups, (state) => !this.#can(number, state));
        if (chain === null) {
          return false;
        }
        this.#extend(lasso, chain);
        continue;
      }
      const takes = (state: number): Edge | undefined =>
        this.#out.of(state).find((edge) => edge.action === action && groups.of[edge.to] === group);
      if (takes(here) === undefined) {
        const chain = this.#route(here, group, groups, (state) => takes(state) !== undefined);
        if (chain === null) {
          return false;
        }
        this.#extend(lasso, chain);
      }
      const edge = takes(lasso.states.at(-1) ?? home);
      if (edge === undefined) {
        return false;
      }
      lasso.states.push(edge.to);
      lasso.actions.push(edge.action);
    }
    const here = lasso.states.at(-1) ?? home;
    if (here !== home || lasso.states.length - 1 === begin) {
      const chain = this.#route(here, group, groups, (state) => state === home);
      if (chain === null) {
        return false;
      }
      this.#extend(lasso, chain);
    }
    lasso.repeatsFrom = begin;
    return true;
  }

  lasso(startsHere: readonly boolean[], reached: readonly boolean[]): Lasso | "lost" | null {
    const inside = reached.map((reachedHere) => !reachedHere);
    const groups = groupsOf(this.#out, inside);
    const doomed = this.#doomed(inside, groups);
    const chain = this.#wayIn(inside, startsHere, doomed);
    const entered = chain?.[0];
    if (chain === null || entered === undefined) {
      return null;
    }
    const before = [entered];
    for (
      let parent = this.#graph.parents[entered] ?? null;
      parent !== null;
      parent = this.#graph.parents[parent] ?? null
    ) {
      before.push(parent);
    }
    before.reverse();
    const lasso: Lasso = {
      states: before,
      actions: before.map(() => null),
      repeatsFrom: null,
    };
    this.#extend(lasso, chain);
    const last = lasso.states.at(-1) ?? entered;
    if ((groups.sizes[groups.of[last] ?? NOWHERE] ?? 0) >= 2 && !this.#turn(lasso, groups)) {
      return "lost";
    }
    return lasso;
  }
}

/**
 * Gives the function that finds, for one rule about runs, a run of `graph` that never reaches
 * the state the rule wants, or `null` when every run reaches it. `startsHere` says for each
 * state whether the rule starts there, and `reached` whether the state is one the rule wants.
 * A run never stops while an edge leaves its state, and never puts off forever a fair action
 * that stays possible. The run found has the fewest steps before it stays or repeats. `"lost"`
 * means a run exists and its loop could not be walked, which a graph of a finished search never
 * gives.
 */
export const hunter = (
  graph: Graph,
): ((startsHere: readonly boolean[], reached: readonly boolean[]) => Lasso | "lost" | null) => {
  const hunt = new Hunt(graph);
  return (startsHere, reached) => hunt.lasso(startsHere, reached);
};
