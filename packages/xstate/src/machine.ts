import type { SearchInput } from "@earnestjs/model";
import type { TraversalOptions } from "@xstate/graph";
import {
  type ActorLogic,
  type AnyActorScope,
  type EventObject,
  type Snapshot,
  createEmptyActor,
} from "xstate";

/**
 * A snapshot the search reached, with the event that led to it and the snapshot before that
 * event. A start has neither. `serializeState` is given all three, as XState gives them.
 */
export interface Arrival<TSnapshot, TEvent> {
  readonly snapshot: TSnapshot;
  readonly event: TEvent | undefined;
  readonly before: TSnapshot | undefined;
}

/**
 * The actor scope `@xstate/graph` steps logic with: built on `createEmptyActor()`, it runs no
 * side effect, so a transition only computes the next snapshot.
 */
export const quietScope = (): AnyActorScope => {
  const emptyActor = createEmptyActor();
  return {
    self: emptyActor,
    logger: console.log,
    id: "",
    sessionId: Math.random().toString(32).slice(2),
    defer: () => {},
    system: emptyActor.system,
    stopChild: () => {},
    emit: () => {},
    actionExecutor: () => {},
  };
};

/**
 * What `search` needs to step XState logic as `@xstate/graph` does with the options
 * `TestModel.getPaths` passes on: the start is `fromState`, or the initial snapshot made with
 * `input`; states are keyed by `serializeState`, which defaults to `JSON.stringify` of the
 * snapshot; the moves are the events `events` lists (none by default), each sent through
 * `logic.transition`; and no event is sent from a snapshot for which `stopWhen` is true, or,
 * without `stopWhen`, for which `toState` is.
 */
export const stepping = <TSnapshot extends Snapshot<unknown>, TEvent extends EventObject, TInput>(
  logic: ActorLogic<TSnapshot, TEvent, TInput>,
  options: TraversalOptions<TSnapshot, TEvent, TInput>,
): Pick<SearchInput<Arrival<TSnapshot, TEvent>, TEvent>, "starts" | "key" | "moves"> => {
  const scope = quietScope();
  const snapshot = options.fromState ?? logic.getInitialSnapshot(scope, options.input as TInput);
  const serializeState = options.serializeState ?? ((state) => JSON.stringify(state));
  const events = options.events ?? [];
  const stopWhen = options.stopWhen ?? options.toState;
  return {
    starts: [{ snapshot, event: undefined, before: undefined }],
    key: ({ snapshot, event, before }) => serializeState(snapshot, event, before),
    moves: ({ snapshot }) => {
      if (stopWhen?.(snapshot) ?? false) {
        return [];
      }
      const sent = typeof events === "function" ? events(snapshot) : events;
      return sent.map((event) => ({
        move: event,
        next: { snapshot: logic.transition(snapshot, event, scope), event, before: snapshot },
      }));
    },
  };
};
