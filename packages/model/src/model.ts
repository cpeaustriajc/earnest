import { type Declaration, type Fields, type StateOf, record, startingAt } from "./declarations.ts";
import { type Row, frozen, isRecord } from "./values.ts";

/** The inputs of an action that takes none. */
export type NoInput = Readonly<Record<never, never>>;

/**
 * The fields an action's `changes` gives back, with every name that is not a field of the model
 * typed as `never`, so that a misspelt field is a compile error.
 */
export type OnlyFields<F extends Fields, Changes> = Changes & {
  readonly [Stray in Exclude<keyof Changes, keyof F>]: never;
};

/**
 * What `m.action` is given, with its keys in any order. `input` declares what the action is
 * given when it happens, and types the second parameter of `when` and `changes`; leave it out
 * for an action that takes none. `when` says in which states, and for which inputs, the action
 * can happen; leave it out for an action that always can. `changes` gives back only the fields
 * that change, and must not change the state it is given. `fair: true` says the action is not
 * put off forever: a run in which it stays possible and never changes the state breaks no
 * `m.eventually` rule.
 */
export interface ActionSpec<
  F extends Fields,
  I extends Fields,
  Changes extends Partial<StateOf<F>> = Partial<StateOf<F>>,
> {
  readonly input?: I;
  readonly when?: (state: StateOf<F>, input: StateOf<I>) => boolean;
  readonly changes: (state: StateOf<F>, input: StateOf<I>) => OnlyFields<F, Changes>;
  readonly fair?: boolean;
}

/**
 * One thing that can happen in a model. Export it from the model file: the name of the export is
 * its id. `when` is always present here, and always true for an action declared without one.
 */
export interface Action<F extends Fields, I extends Fields = NoInput> {
  readonly kind: "action";
  readonly sentence: string;
  readonly input: I | undefined;
  readonly when: (state: StateOf<F>, input: StateOf<I>) => boolean;
  readonly changes: (state: StateOf<F>, input: StateOf<I>) => Partial<StateOf<F>>;
}

/** A rule every reachable state must meet. The name of its export is its id. */
export interface Rule<F extends Fields> {
  readonly kind: "rule";
  readonly says: string;
  readonly holds: (state: StateOf<F>) => boolean;
}

/**
 * States where stopping is correct. A state with no possible action that no end covers is
 * stuck. The name of its export is its id.
 */
export interface End<F extends Fields> {
  readonly kind: "end";
  readonly says: string;
  readonly when: (state: StateOf<F>) => boolean;
}

/**
 * Other values for some fields to start at: `earnest verify` checks the model once more from
 * them, and `explore` does when it is given the setup. The name of its export is its id.
 */
export interface Setup<F extends Fields> {
  readonly kind: "setup";
  readonly says: string;
  readonly starts: Partial<StateOf<F>>;
}

/**
 * What `m.eventually` is given. `reaches` is the condition every run must come to. `after` says
 * where the rule starts: in every reachable state where it is true. Without `after` the rule
 * starts in every start state.
 */
export interface EventuallySpec<F extends Fields> {
  readonly after?: (state: StateOf<F>) => boolean;
  readonly reaches: (state: StateOf<F>) => boolean;
}

/** A rule about whole runs: a state every run reaches. The name of its export is its id. */
export interface Eventually<F extends Fields> {
  readonly kind: "eventually";
  readonly says: string;
  readonly after: ((state: StateOf<F>) => boolean) | undefined;
  readonly reaches: (state: StateOf<F>) => boolean;
}

/**
 * One step of a witness: an action with the value of each of its inputs. Make one with
 * `m.step(action, { name: value })`; an action without inputs is a step as it is.
 */
export interface Step<F extends Fields> {
  readonly kind: "step";
  readonly action: ActionOf<F>;
  readonly input: Row;
}

/** An action of a model with the fields `F`, whatever its inputs are. */
export interface ActionOf<F extends Fields> {
  readonly kind: "action";
  readonly sentence: string;
  readonly when: (state: StateOf<F>, input: never) => boolean;
  readonly changes: (state: StateOf<F>, input: never) => Partial<StateOf<F>>;
}

/**
 * What `m.witness` is given: the actions to take in order, and the condition that must be true
 * after the last one. With `setup`, the path is played from that setup's start states and the
 * witness is reported with that setup; without it, from the model's own start states.
 */
export interface WitnessSpec<F extends Fields> {
  readonly setup?: Setup<F>;
  readonly steps: readonly (Action<F> | Step<F>)[];
  readonly expect: (state: StateOf<F>) => boolean;
}

/**
 * A path the model must be able to take from every start state, with a condition on the state
 * it ends in. The name of its export is its id.
 */
export interface Witness<F extends Fields> {
  readonly kind: "witness";
  readonly says: string;
  readonly setup: Setup<F> | undefined;
  readonly steps: readonly Step<F>[];
  readonly expect: (state: StateOf<F>) => boolean;
}

/**
 * What `m.view` is given, all of it optional: the chart's title, the label of a state, the
 * group a state is drawn in, arrow labels by action id (the default is the action's sentence)
 * and colors.
 */
export interface ViewSpec<F extends Fields> {
  readonly title?: string;
  readonly state?: (state: StateOf<F>) => string;
  readonly group?: (state: StateOf<F>) => string;
  readonly step?: Readonly<Record<string, string>>;
  readonly colors?: { readonly broken?: string; readonly start?: string; readonly end?: string };
}

/** How `earnest chart` draws a model. It changes nothing about what is checked. */
export interface View<F extends Fields> extends ViewSpec<F> {
  readonly kind: "view";
}

/**
 * An action as a search runs it: on states and inputs whose types are not known statically.
 * `part` is the `Action` the model file exports. `when` and `changes` throw when the state or
 * the input is outside what was declared; `changes` gives back the whole next state.
 */
export interface ActionRunner {
  readonly part: object;
  readonly sentence: string;
  readonly fair?: boolean;
  readonly inputs: readonly (readonly [string, Declaration<unknown, never>])[];
  when(state: Row, input: Row): boolean;
  changes(state: Row, input: Row): Row;
}

/** A rule as a search runs it. `holds` throws when the state is outside what was declared. */
export interface RuleRunner {
  readonly part: object;
  readonly says: string;
  holds(state: Row): boolean;
}

/** An end as a search runs it. `when` throws when the state is outside what was declared. */
export interface EndRunner {
  readonly part: object;
  readonly says: string;
  when(state: Row): boolean;
}

/**
 * A rule about runs as a search judges it. `after` is `null` for a rule that starts in every
 * start state. Both throw when the state is outside what was declared.
 */
export interface EventuallyRunner {
  readonly part: object;
  readonly says: string;
  readonly after: ((state: Row) => boolean) | null;
  reaches(state: Row): boolean;
}

/**
 * A witness as a search plays it: each step is an action of the same `Runners` with its input.
 * `setup` is the `part` of the setup it is played in, when it names one. `expect` throws when
 * the state is outside what was declared.
 */
export interface WitnessRunner {
  readonly part: object;
  readonly says: string;
  readonly setup?: object;
  readonly steps: readonly { readonly action: ActionRunner; readonly input: Row }[];
  expect(state: Row): boolean;
}

/** A setup as a search starts in it: `starts` holds the fields that start elsewhere. */
export interface SetupRunner {
  readonly part: object;
  readonly says: string;
  readonly starts: Row;
}

/** Everything declared on a model so far, each list in declaration order. */
export interface Runners {
  readonly actions: readonly ActionRunner[];
  readonly rules: readonly RuleRunner[];
  readonly ends: readonly EndRunner[];
  readonly eventually?: readonly EventuallyRunner[];
  readonly witnesses?: readonly WitnessRunner[];
  readonly setups?: readonly SetupRunner[];
}

/**
 * What `explore` and `replay` need of a model, whatever its fields are. Every `Model` is one.
 */
export interface AnyModel {
  readonly name: string;
  readonly fields: Fields;
  starts(): readonly Row[];
  runners(): Runners;
}

interface Parts<F extends Fields> {
  readonly actions: ActionRunner[];
  readonly rules: RuleRunner[];
  readonly ends: EndRunner[];
  readonly eventually: EventuallyRunner[];
  readonly witnesses: WitnessRunner[];
  readonly setups: SetupRunner[];
  readonly views: View<F>[];
}

let partsOf: <F extends Fields>(model: Model<F>) => Parts<F>;

/**
 * A model: named fields, and the actions, rules, ends and views that `m.action`, `m.rule`,
 * `m.end` and `m.view` have declared on it, in the order they were called, which is the order a
 * search tries actions in.
 */
export class Model<F extends Fields> {
  readonly name: string;
  readonly fields: F;
  readonly #state: Declaration<StateOf<F>>;
  readonly #parts: Parts<F> = {
    actions: [],
    rules: [],
    ends: [],
    eventually: [],
    witnesses: [],
    setups: [],
    views: [],
  };

  static {
    partsOf = (model) => model.#parts;
  }

  /** Model files call `m.model(name, fields)` instead. */
  constructor(name: string, fields: F) {
    this.name = name;
    this.fields = fields;
    this.#state = record(fields);
  }

  /** Whether `state` has exactly the model's fields, each at a value its declaration has. */
  has(state: unknown): state is StateOf<F> {
    return this.#state.has(state);
  }

  /**
   * Every start state, in the order the checker visits them: fields in declaration order, the
   * first field varying slowest.
   */
  starts(): readonly StateOf<F>[] {
    const isState = (row: unknown): row is StateOf<F> => this.has(row);
    return Object.entries(this.fields)
      .reduce<Row[]>(
        (partial, [name, declared]) =>
          partial.flatMap((prefix) =>
            declared.starts().map((value) => ({ ...prefix, [name]: value })),
          ),
        [{}],
      )
      .map(frozen)
      .filter(isState);
  }

  /** The views declared so far, in declaration order. */
  get views(): readonly View<F>[] {
    return [...this.#parts.views];
  }

  /**
   * The actions, rules, ends, rules about runs, witnesses and setups declared so far, in a form a search can run without knowing
   * the model's types. `explore` and `replay` are built on this.
   */
  runners(): Runners {
    const { actions, rules, ends, eventually, witnesses, setups } = this.#parts;
    return {
      actions: [...actions],
      rules: [...rules],
      ends: [...ends],
      eventually: [...eventually],
      witnesses: [...witnesses],
      setups: [...setups],
    };
  }
}

const known = <F extends Fields>(model: Model<F>, state: Row): StateOf<F> => {
  if (!model.has(state)) {
    throw new Error(`${model.name}: the state is outside what the model declares`);
  }
  return state;
};

const changed = (model: AnyModel, state: Row, changes: unknown, sentence: string): Row => {
  if (!isRecord(changes)) {
    throw new Error(
      `${model.name}: the change of "${sentence}" must give back an object holding the fields that change`,
    );
  }
  const stray = Object.keys(changes).find((name) => !Object.hasOwn(model.fields, name));
  if (stray !== undefined) {
    throw new Error(
      `${model.name}: the change of "${sentence}" gives \`${stray}\`, which is not a field of the model`,
    );
  }
  return frozen({ ...state, ...changes });
};

/**
 * Declares a model: a name and its fields, each built with an `m` builder. The state type is
 * inferred from the fields; `m.Infer<typeof Model>` names it.
 */
export const model = <F extends Fields>(name: string, fields: F): Model<F> =>
  new Model(name, fields);

/**
 * Declares something that can happen in `model`. `sentence` says it in the reviewer's words.
 * Export the result from the model file: the name of the export is the action's id. Actions are
 * tried in the order they are declared. `input`, `when` and `changes` may be written in any
 * order; `changes` is the only one that is required.
 */
export const action = <
  F extends Fields,
  I extends Fields = NoInput,
  const Changes extends Partial<StateOf<F>> = Partial<StateOf<F>>,
>(
  model: Model<F>,
  sentence: string,
  spec: ActionSpec<F, I, Changes>,
): Action<F, I> => {
  const declared = spec.input === undefined ? null : record(spec.input);
  const isInput = (input: unknown): input is StateOf<I> =>
    declared === null ? isRecord(input) && Object.keys(input).length === 0 : declared.has(input);
  const knownInput = (input: Row): StateOf<I> => {
    if (!isInput(input)) {
      throw new Error(`${model.name}: "${sentence}" was given an input it does not declare`);
    }
    return input;
  };
  const declaredAction: Action<F, I> = Object.freeze({
    kind: "action",
    sentence,
    input: spec.input,
    when: spec.when ?? (() => true),
    changes: spec.changes,
  });
  partsOf(model).actions.push({
    part: declaredAction,
    sentence,
    fair: spec.fair ?? false,
    inputs: spec.input === undefined ? [] : Object.entries(spec.input),
    when: (state, input) => declaredAction.when(known(model, state), knownInput(input)),
    changes: (state, input) =>
      changed(
        model,
        state,
        declaredAction.changes(known(model, state), knownInput(input)),
        sentence,
      ),
  });
  return declaredAction;
};

/**
 * Declares a rule of `model`: `holds` must be true in every state the model can reach.
 * `sentence` says it in the reviewer's words. Export the result from the model file: the name
 * of the export is the rule's id.
 */
export const rule = <F extends Fields>(
  model: Model<F>,
  sentence: string,
  holds: (state: StateOf<F>) => boolean,
): Rule<F> => {
  const declaredRule: Rule<F> = Object.freeze({ kind: "rule", says: sentence, holds });
  partsOf(model).rules.push({
    part: declaredRule,
    says: sentence,
    holds: (state) => declaredRule.holds(known(model, state)),
  });
  return declaredRule;
};

/**
 * Declares where stopping is correct in `model`, so that those states are not reported as
 * stuck. `sentence` says it in the reviewer's words. Export the result from the model file: the
 * name of the export is the end's id.
 */
export const end = <F extends Fields>(
  model: Model<F>,
  sentence: string,
  when: (state: StateOf<F>) => boolean,
): End<F> => {
  const declaredEnd: End<F> = Object.freeze({ kind: "end", says: sentence, when });
  partsOf(model).ends.push({
    part: declaredEnd,
    says: sentence,
    when: (state) => declaredEnd.when(known(model, state)),
  });
  return declaredEnd;
};

/**
 * Declares a rule about whole runs of `model`: every run comes to a state where `reaches` is
 * true. With `after`, the rule starts in every reachable state where `after` is true, and the
 * state `reaches` wants may be that state or any later one; without it, the rule starts in every
 * start state. A run never stops while an action can still change the state, and never puts off
 * forever an action declared with `fair: true` that stays possible. `sentence` says the rule in
 * the reviewer's words. Export the result from the model file: the name of the export is the
 * rule's id.
 */
export const eventually = <F extends Fields>(
  model: Model<F>,
  sentence: string,
  spec: EventuallySpec<F>,
): Eventually<F> => {
  const declaredRule: Eventually<F> = Object.freeze({
    kind: "eventually",
    says: sentence,
    after: spec.after,
    reaches: spec.reaches,
  });
  const { after } = declaredRule;
  partsOf(model).eventually.push({
    part: declaredRule,
    says: sentence,
    after: after === undefined ? null : (state) => after(known(model, state)),
    reaches: (state) => declaredRule.reaches(known(model, state)),
  });
  return declaredRule;
};

/**
 * Declares a setup of `model`: the model checked once more, with the fields in `starts` starting
 * at the values given and every other field starting where the model declares. A field no action
 * changes works as a parameter of the model this way. `sentence` says what the setup stands for.
 * Export the result from the model file: the name of the export is the setup's id. Throws when
 * `starts` is empty, names something that is not a field, or gives a field a value it does not
 * declare.
 */
export const setup = <F extends Fields, const Starts extends Partial<StateOf<F>>>(
  model: Model<F>,
  sentence: string,
  starts: OnlyFields<F, Starts>,
): Setup<F> => {
  const names = Object.keys(starts);
  if (names.length === 0) {
    throw new Error(`${model.name}: the setup "${sentence}" starts no field at another value`);
  }
  const adopted: Record<string, unknown> = {};
  for (const [name, value] of Object.entries(starts)) {
    const declared: Declaration<unknown, never> | undefined = Object.hasOwn(model.fields, name)
      ? model.fields[name]
      : undefined;
    if (declared === undefined) {
      throw new Error(
        `${model.name}: the setup "${sentence}" starts \`${name}\`, which is not a field of the model`,
      );
    }
    try {
      adopted[name] = startingAt(declared, value);
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      throw new Error(
        `${model.name}: the setup "${sentence}" starts \`${name}\` at a value the field does not declare: ${why}`,
        { cause: error },
      );
    }
  }
  const declaredSetup: Setup<F> = Object.freeze({ kind: "setup", says: sentence, starts });
  partsOf(model).setups.push({ part: declaredSetup, says: sentence, starts: frozen(adopted) });
  return declaredSetup;
};

/**
 * Makes a step of a witness from an action that takes inputs: `m.step(press, { by: "mouse" })`.
 * `input` gives every input of the action a value it declares.
 */
export const step = <F extends Fields, I extends Fields>(
  taken: Action<F, I>,
  input: StateOf<I>,
): Step<F> => Object.freeze({ kind: "step", action: taken, input: frozen({ ...input }) });

/**
 * Declares a witness of `model`: a path it must be able to take. From every start state, each
 * step in `steps` must be able to happen in turn, and `expect` must be true in the state after
 * the last one. `sentence` says it in the reviewer's words. Export the result from the model
 * file: the name of the export is the witness's id. `earnest verify` and `explore` play it and
 * report it with the rules. Throws when a step is not an action declared on `model` before the
 * witness, or gives an input the action does not declare.
 */
export const witness = <F extends Fields>(
  model: Model<F>,
  sentence: string,
  spec: WitnessSpec<F>,
): Witness<F> => {
  const { actions, witnesses, setups } = partsOf(model);
  if (spec.setup !== undefined && !setups.some((declared) => declared.part === spec.setup)) {
    throw new Error(
      `${model.name}: the setup of "${sentence}" is not one declared on this model before the witness`,
    );
  }
  const played = spec.steps.map((written, position) => {
    const taken: Step<F> =
      written.kind === "step"
        ? written
        : Object.freeze({ kind: "step", action: written, input: frozen({}) });
    const runner = actions.find((declared) => declared.part === taken.action);
    const number = position + 1;
    if (runner === undefined) {
      throw new Error(
        `${model.name}: step ${number} of "${sentence}" is not an action declared on this model before the witness`,
      );
    }
    if (!record(Object.fromEntries(runner.inputs)).has(taken.input)) {
      throw new Error(
        `${model.name}: step ${number} of "${sentence}" gives "${runner.sentence}" an input it does not declare`,
      );
    }
    return { taken, runner };
  });
  const declaredWitness: Witness<F> = Object.freeze({
    kind: "witness",
    says: sentence,
    setup: spec.setup,
    steps: Object.freeze(played.map(({ taken }) => taken)),
    expect: spec.expect,
  });
  witnesses.push({
    part: declaredWitness,
    says: sentence,
    ...(spec.setup === undefined ? {} : { setup: spec.setup }),
    steps: played.map(({ taken, runner }) => ({ action: runner, input: taken.input })),
    expect: (state) => declaredWitness.expect(known(model, state)),
  });
  return declaredWitness;
};

/**
 * Declares how `earnest chart` draws `model`. A model needs no view, and a view has no id, so
 * its constant needs no export. In a model file, write the second argument, even as `{}`: the
 * checker refuses `m.view(model)`.
 */
export const view = <F extends Fields>(model: Model<F>, spec: ViewSpec<F> = {}): View<F> => {
  const declaredView: View<F> = Object.freeze({ ...spec, kind: "view" });
  partsOf(model).views.push(declaredView);
  return declaredView;
};
