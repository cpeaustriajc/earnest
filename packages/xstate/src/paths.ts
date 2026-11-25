import { type Trail, search } from "@earnestjs/model";
import type { StatePath, Step, TraversalOptions } from "@xstate/graph";
import type { ActorLogic, EventObject, Snapshot } from "xstate";
import { type Arrival, stepping } from "./machine.ts";

const toStatePath = <TSnapshot extends Snapshot<unknown>, TEvent extends EventObject>({
  start,
  steps,
}: Trail<Arrival<TSnapshot, TEvent>, TEvent>): StatePath<TSnapshot, TEvent> => {
  const initStep: Step<TSnapshot, TEvent> = {
    state: start.snapshot,
    event: { type: "xstate.init" } as TEvent,
  };
  return {
    state: (steps.at(-1)?.state ?? start).snapshot,
    steps: [initStep, ...steps.map(({ move, state }) => ({ state: state.snapshot, event: move }))],
    weight: steps.length,
  };
};

/**
 * A path generator for `TestModel.getPaths` from `@xstate/graph`: `model.getPaths(earnestPaths)`
 * gives one shortest path to every state reachable from the start, found by the breadth-first
 * `search` of `@earnestjs/model`, in place of `model.getShortestPaths()`. Each path has the shape
 * XState's own shortest paths have: a first step with the start snapshot and the event
 * `xstate.init`, then one step per event with the snapshot after it, and `weight`, the number of
 * events. States are told apart by `serializeState`, and `events`, `fromState`, `input`,
 * `stopWhen` and `toState` work as they do for `getShortestPaths`. It takes the events and the
 * `serializeState` that `getPaths` passes on: called on its own with neither, it sends no event
 * and keys a snapshot by `JSON.stringify`. A finite `limit` is the most distinct states the
 * search may hold, a whole number that is not negative; when the search would hold more, it
 * throws an error that names the limit and the count, and gives no paths. Paths come in the
 * order the search reached their states, which may differ from XState's order.
 */
export const earnestPaths = <
  TSnapshot extends Snapshot<unknown>,
  TEvent extends EventObject,
  TInput,
>(
  logic: ActorLogic<TSnapshot, TEvent, TInput>,
  options: TraversalOptions<TSnapshot, TEvent, TInput>,
): StatePath<TSnapshot, TEvent>[] => {
  const { limit } = options;
  const isBounded = limit !== undefined && limit !== Infinity;
  if (isBounded && !(Number.isInteger(limit) && limit >= 0)) {
    throw new Error(`limit must be a whole number that is not negative, or Infinity, not ${limit}`);
  }
  const found = search({
    ...stepping(logic, options),
    ...(isBounded ? { maxStates: limit } : {}),
  });
  if (found.reason !== null) {
    throw new Error(
      `earnestPaths stopped at the ${found.reason} after ${found.states} states, before it reached every state; raise limit to search them all`,
    );
  }
  const paths: StatePath<TSnapshot, TEvent>[] = [];
  for (const trail of found.reached()) {
    const path = toStatePath(trail);
    if (options.toState?.(path.state) ?? true) {
      paths.push(path);
    }
  }
  return paths;
};
