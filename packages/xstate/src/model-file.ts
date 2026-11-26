import { type Row, combinationsOf } from "@earnestjs/model";
import type { AnyMachineSnapshot, AnyStateMachine, EventObject, StateValue } from "xstate";
import {
  type CheckMachineOptions,
  type Declared,
  EACH_FIELD,
  type Fields,
  described,
  fieldRecord,
  historyOf,
  isPlain,
  keyOf,
  payloadsOf,
  plainFields,
} from "./check.ts";
import { quietScope } from "./machine.ts";

/**
 * The most rows `modelFileOf` tabulates: for every state of the machine, one row for each
 * combination of declared context values with each event and payload, and one with each rule.
 */
export const MOST_TABLE_ROWS = 100_000;

/** What `modelFileOf` is given: what `checkMachine` is given, and the name of the model. */
export interface ModelFileOptions<
  TMachine extends AnyStateMachine,
> extends CheckMachineOptions<TMachine> {
  readonly name: string;
}

/**
 * A point of the table where the machine's own code threw, or XState caught an error into a
 * snapshot whose status is `error`. `part` says whether it was an event's guard or transition
 * (`id` is the event type) or a rule (`id` is the rule's id). `state` is the label of the
 * machine's state in the model file, `context` and `payload` the values it was tried with.
 */
export interface MachineThrow {
  readonly part: "transition" | "rule";
  readonly id: string;
  readonly state: string;
  readonly context: Row;
  readonly payload: Row;
  readonly error: unknown;
}

/**
 * A machine written out as a model file. `text` is the file. `threw` lists every point of the
 * table where the machine's code threw; the file makes reaching one visible: an event there
 * sets the field `threw` out of its range, and a rule there is broken. `actions` gives the event
 * type of each action the file exports, by its id, and `rules` the id in `rules` of each rule it
 * exports, by its id, for an id that had to change to be a name in the file.
 */
export interface MachineModelFile {
  readonly text: string;
  readonly threw: readonly MachineThrow[];
  readonly actions: Readonly<Record<string, string>>;
  readonly rules: Readonly<Record<string, string>>;
}

const CALLER = "modelFileOf";

const WIDTH = 100;

const RESERVED = new Set([
  "m",
  "s",
  "input",
  "State",
  "arguments",
  "as",
  "async",
  "await",
  "break",
  "case",
  "catch",
  "class",
  "const",
  "continue",
  "debugger",
  "default",
  "delete",
  "do",
  "else",
  "enum",
  "eval",
  "export",
  "extends",
  "false",
  "finally",
  "for",
  "from",
  "function",
  "if",
  "implements",
  "import",
  "in",
  "Infinity",
  "instanceof",
  "interface",
  "let",
  "NaN",
  "new",
  "null",
  "of",
  "package",
  "private",
  "protected",
  "public",
  "return",
  "static",
  "super",
  "switch",
  "this",
  "throw",
  "true",
  "try",
  "type",
  "typeof",
  "undefined",
  "var",
  "void",
  "while",
  "with",
  "yield",
]);

const IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]*$/;

const isName = (text: string): boolean => IDENTIFIER.test(text) && !RESERVED.has(text);

const isFieldName = (text: string): boolean => IDENTIFIER.test(text) && text !== "__proto__";

const nameLike = (text: string): string => {
  const replaced = text.replace(/[^A-Za-z0-9_]/g, "_");
  const started = /^[A-Za-z_]/.test(replaced) ? replaced : `_${replaced}`;
  return RESERVED.has(started) ? `${started}_` : started;
};

class Names {
  readonly #taken: Set<string>;

  constructor(taken: Iterable<string>) {
    this.#taken = new Set(taken);
  }

  claim(wanted: string): string {
    const base = nameLike(wanted);
    let name = base;
    for (let count = 2; this.#taken.has(name); count += 1) {
      name = `${base}_${count}`;
    }
    this.#taken.add(name);
    return name;
  }
}

type Primitive = string | number | boolean | null;

type Shape =
  | { readonly kind: "plain"; readonly type: string; readonly nullable: boolean }
  | {
      readonly kind: "record";
      readonly fields: readonly (readonly [string, Shape])[];
      readonly nullable: boolean;
    };

const fieldOf = (value: unknown, name: string): unknown =>
  typeof value === "object" && value !== null && Object.hasOwn(value, name)
    ? Reflect.get(value, name)
    : undefined;

const shapeOf = (option: string, values: readonly unknown[]): Shape => {
  const nullable = values.includes(null);
  const held = values.filter((value) => value !== null);
  const first = held[0];
  if (typeof first !== "object" || first === null) {
    return { kind: "plain", type: first === undefined ? "null" : typeof first, nullable };
  }
  return {
    kind: "record",
    nullable,
    fields: Object.keys(first).map((name) => {
      if (!isFieldName(name)) {
        throw new TypeError(
          `${CALLER} can't take ${option}.${name}: a model file reads a record's field by its name, so the name must be an identifier, such as ${nameLike(name)}`,
        );
      }
      return [
        name,
        shapeOf(
          `${option}.${name}`,
          held.map((value) => fieldOf(value, name)),
        ),
      ] as const;
    }),
  };
};

const NOT_WRITTEN = Symbol("not written");

const written = (shape: Shape, value: unknown): unknown => {
  if (value === undefined || value === null) {
    return shape.nullable || (shape.kind === "plain" && shape.type === "null") ? null : NOT_WRITTEN;
  }
  if (shape.kind === "plain") {
    if (typeof value !== shape.type) {
      return NOT_WRITTEN;
    }
    return typeof value === "number" && !Number.isSafeInteger(value) ? NOT_WRITTEN : value;
  }
  if (typeof value !== "object" || Array.isArray(value) || !isPlain(value)) {
    return NOT_WRITTEN;
  }
  const held = Object.keys(value).filter((name) => fieldOf(value, name) !== undefined);
  if (held.some((name) => !shape.fields.some(([field]) => field === name))) {
    return NOT_WRITTEN;
  }
  const fields = shape.fields.map(([name, inner]) => [name, written(inner, fieldOf(value, name))]);
  return fields.some(([, inner]) => inner === NOT_WRITTEN)
    ? NOT_WRITTEN
    : Object.fromEntries(fields);
};

const literal = (value: unknown): string => {
  if (typeof value === "object" && value !== null) {
    const fields = Object.entries(value).map(([name, inner]) => `${name}: ${literal(inner)}`);
    return fields.length === 0 ? "{}" : `{ ${fields.join(", ")} }`;
  }
  return typeof value === "number" && Object.is(value, -0) ? "0" : JSON.stringify(value);
};

interface Field {
  readonly name: string;
  readonly declared: Declared;
  readonly shape: Shape;
}

const fieldsOf = (option: string, fields: Fields): readonly Field[] =>
  fields.map(([name, declared]) => {
    if (!isFieldName(name)) {
      throw new TypeError(
        `${CALLER} can't take ${option}.${name}: a model file reads a field by its name, so the name must be an identifier, such as ${nameLike(name)}`,
      );
    }
    if (typeof declared.source !== "string") {
      throw new TypeError(
        `${CALLER} can't take ${option}.${name}: it was made with an @earnestjs/model older than the one @earnestjs/xstate uses; install the version it depends on`,
      );
    }
    return { name, declared, shape: shapeOf(`${option}.${name}`, declared.values()) };
  });

interface EventKind {
  readonly type: string;
  readonly fields: Fields;
}

const PAYLOAD_ADVICE =
  "write the events as a record of event types, each with a declaration for each payload field, such as { ENTER: { group: m.int(1, 2) } }";

const eventKindsOf = (events: unknown): readonly EventKind[] => {
  if (typeof events === "function") {
    throw new TypeError(
      `${CALLER} can't take events as a function of the snapshot: a model file gives each event once, for every state; ${PAYLOAD_ADVICE}, or list the events`,
    );
  }
  if (Array.isArray(events)) {
    const types: string[] = [];
    events.forEach((event: unknown, index) => {
      const type = fieldOf(event, "type");
      if (typeof type !== "string") {
        throw new TypeError(
          `${CALLER} can't take events[${index}]: it is ${described(event)}, not an event with a type`,
        );
      }
      const payload = Object.keys(event as object).find(
        (name) => name !== "type" && fieldOf(event, name) !== undefined,
      );
      if (payload !== undefined) {
        throw new TypeError(
          `${CALLER} can't take events[${index}], ${type} with the payload field ${payload}: a model file declares a payload's values; ${PAYLOAD_ADVICE}`,
        );
      }
      if (!types.includes(type)) {
        types.push(type);
      }
    });
    return types.map((type) => ({ type, fields: [] }));
  }
  const schema = fieldRecord(
    CALLER,
    "events",
    events,
    "a list of events or a record of event types, such as { ENTER: { group: m.int(1, 2) } }",
  );
  return payloadsOf(CALLER, schema).map(([type, fields]) => ({ type, fields }));
};

const countOf = (declarations: readonly Declared[]): number =>
  declarations.reduce((total, declared) => total * declared.count, 1);

const rowsOf = (fields: readonly Field[]): readonly Row[] =>
  combinationsOf(Object.fromEntries(fields.map(({ name, declared }) => [name, declared])));

interface Configuration {
  readonly value: StateValue;
  readonly status: AnyMachineSnapshot["status"];
  readonly historyValue: AnyMachineSnapshot["historyValue"];
  readonly history: Readonly<Record<string, readonly string[]>>;
}

type Outcome =
  | { readonly kind: "off" }
  | { readonly kind: "threw" }
  | { readonly kind: "on"; readonly state: number; readonly context: Row };

const OFF: Outcome = { kind: "off" };

const THREW: Outcome = { kind: "threw" };

const leavesOf = (value: StateValue, prefix: string): readonly string[] => {
  if (typeof value === "string") {
    return [`${prefix}${value}`];
  }
  const entries = Object.entries(value);
  return entries.length === 0
    ? [prefix.slice(0, -1)].filter((leaf) => leaf !== "")
    : entries.flatMap(([key, inner]) =>
        inner === undefined ? [`${prefix}${key}`] : leavesOf(inner, `${prefix}${key}.`),
      );
};

const labelsOf = (machine: AnyStateMachine, configurations: readonly Configuration[]): string[] => {
  const root = `${machine.root.id}.`;
  const local = (id: string): string => (id.startsWith(root) ? id.slice(root.length) : id);
  const named = (labels: readonly string[], more: (index: number) => string): string[] =>
    labels.map((label, index) =>
      labels.filter((other) => other === label).length > 1 ? more(index) : label,
    );
  const bases = configurations.map(({ value }) => leavesOf(value, "").join(", ") || machine.id);
  const remembered = named(bases, (index) => {
    const configuration = configurations[index];
    const history =
      configuration === undefined
        ? []
        : Object.entries(configuration.history)
            .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
            .map(([node, held]) => `${local(node)}: ${held.map(local).join(" + ")}`);
    return history.length === 0 ? `${bases[index]}` : `${bases[index]} (${history.join("; ")})`;
  });
  const statused = named(remembered, (index) => {
    const status = configurations[index]?.status ?? "active";
    return status === "active" ? `${remembered[index]}` : `${remembered[index]} (${status})`;
  });
  return named(statused, (index) => {
    const label = statused[index] ?? "";
    return `${label} #${statused.slice(0, index + 1).filter((other) => other === label).length}`;
  });
};

interface Point {
  readonly state: number;
  readonly context: Row;
  readonly payload: Row;
}

const ABSENT = Symbol("absent");

interface Atom {
  readonly scope: "state" | "input";
  readonly name: string;
  readonly test: "value" | "null";
  readonly nullable: boolean;
  read(point: Point): Primitive | typeof ABSENT;
  expression(root: string): string;
  guards(root: string): readonly string[];
}

const atomsOf = (
  scope: "state" | "input",
  name: string,
  shape: Shape,
  read: (point: Point) => unknown,
  expression: (root: string) => string,
  guards: (root: string) => readonly string[],
): readonly Atom[] => {
  if (shape.kind === "plain") {
    return [
      {
        scope,
        name,
        test: "value",
        nullable: shape.nullable,
        read: (point) => {
          const value = read(point);
          return value === ABSENT ? ABSENT : (value as Primitive);
        },
        expression,
        guards,
      },
    ];
  }
  const nullAtoms: readonly Atom[] = shape.nullable
    ? [
        {
          scope,
          name,
          test: "null",
          nullable: false,
          read: (point) => {
            const value = read(point);
            return value === ABSENT ? ABSENT : value === null;
          },
          expression,
          guards,
        },
      ]
    : [];
  const inner = shape.nullable ? (root: string) => `m.some(${expression(root)})` : expression;
  const innerGuards = shape.nullable
    ? (root: string) => [...guards(root), `${expression(root)} !== null`]
    : guards;
  const innerRead = (point: Point): unknown => {
    const value = read(point);
    return value === null ? ABSENT : value;
  };
  return [
    ...nullAtoms,
    ...shape.fields.flatMap(([field, fieldShape]) =>
      atomsOf(
        scope,
        name,
        fieldShape,
        (point) => {
          const value = innerRead(point);
          return value === ABSENT ? ABSENT : fieldOf(value, field);
        },
        (root) => `${inner(root)}.${field}`,
        innerGuards,
      ),
    ),
  ];
};

const fieldAtoms = (
  scope: "state" | "input",
  field: Field,
  read: (point: Point) => unknown,
): readonly Atom[] =>
  atomsOf(
    scope,
    field.name,
    field.shape,
    read,
    (root) => root,
    () => [],
  );

type Value =
  | { readonly kind: "literal"; readonly text: string }
  | { readonly kind: "state" | "input"; readonly name: string };

const valueKey = (value: Value): string =>
  value.kind === "literal" ? `=${value.text}` : `${value.kind}:${value.name}`;

interface Output {
  readonly literal: string;
  readonly matches: ReadonlySet<string>;
}

type Test = readonly [Atom, Primitive | typeof ABSENT];

interface Leaf {
  readonly path: readonly Test[];
  readonly value: Value;
}

const atomKey = (value: Primitive | typeof ABSENT): string =>
  value === ABSENT ? "absent" : keyOf(value);

const grow = (
  atoms: readonly Atom[],
  points: readonly Point[],
  outputs: readonly (Output | null)[],
  candidates: readonly Value[],
): readonly Leaf[] => {
  const leaves: Leaf[] = [];
  const readAt = (atom: Atom, index: number): Primitive | typeof ABSENT => {
    const point = points[index];
    return point === undefined ? ABSENT : atom.read(point);
  };
  const literalAt = (index: number): string => outputs[index]?.literal ?? "";
  const decides = (others: readonly Atom[], cared: readonly number[]): boolean => {
    const seen = new Map<string, string>();
    for (const index of cared) {
      const key = others.map((atom) => atomKey(readAt(atom, index))).join("|");
      const found = seen.get(key);
      if (found === undefined) {
        seen.set(key, literalAt(index));
      } else if (found !== literalAt(index)) {
        return false;
      }
    }
    return true;
  };
  const visit = (cared: readonly number[], available: readonly Atom[], path: readonly Test[]) => {
    const [first] = cared;
    if (first === undefined) {
      return;
    }
    if (cared.every((index) => literalAt(index) === literalAt(first))) {
      leaves.push({ path, value: { kind: "literal", text: literalAt(first) } });
      return;
    }
    const shared = candidates.find((candidate) =>
      cared.every((index) => outputs[index]?.matches.has(valueKey(candidate)) === true),
    );
    if (shared !== undefined) {
      leaves.push({ path, value: shared });
      return;
    }
    const varying = available.filter((atom) =>
      cared.some((index) => atomKey(readAt(atom, index)) !== atomKey(readAt(atom, first))),
    );
    const chosen =
      varying.find(
        (atom) =>
          !decides(
            varying.filter((other) => other !== atom),
            cared,
          ),
      ) ?? varying[0];
    if (chosen === undefined) {
      throw new Error(`${CALLER} found two outcomes for the same state and event`);
    }
    const groups = new Map<string, { value: Primitive | typeof ABSENT; cared: number[] }>();
    for (const index of cared) {
      const value = readAt(chosen, index);
      const key = atomKey(value);
      const group = groups.get(key);
      if (group === undefined) {
        groups.set(key, { value, cared: [index] });
      } else {
        group.cared.push(index);
      }
    }
    const rest = varying.filter((atom) => atom !== chosen);
    for (const group of groups.values()) {
      visit(group.cared, rest, [...path, [chosen, group.value]]);
    }
  };
  visit(
    outputs.flatMap((output, index) => (output === null ? [] : [index])),
    atoms,
    [],
  );
  return leaves;
};

interface Rendering {
  readonly root: (atom: Atom) => string;
  readonly input: (name: string) => string;
}

const rendered = (value: Value, rendering: Rendering): string => {
  if (value.kind === "literal") {
    return value.text;
  }
  return value.kind === "state" ? `s.${value.name}` : rendering.input(value.name);
};

const termsOf = ([atom, value]: Test, { root }: Rendering): readonly string[] => {
  const base = root(atom);
  const guards = atom.guards(base);
  if (value === ABSENT) {
    const last = guards.at(-1);
    return guards.length === 1 && last !== undefined
      ? [last.replace(/ !== null$/, " === null")]
      : [`!(${guards.join(" && ")})`];
  }
  const expression = atom.expression(base);
  if (atom.test === "null") {
    return [...guards, `${expression} ${value === true ? "===" : "!=="} null`];
  }
  if (typeof value === "boolean" && !atom.nullable) {
    return [...guards, value ? expression : `!${expression}`];
  }
  return [...guards, `${expression} === ${literal(value)}`];
};

const conditionOf = (leaf: Leaf, rendering: Rendering): readonly string[] => [
  ...new Set(leaf.path.flatMap((step) => termsOf(step, rendering))),
];

const negated = (terms: readonly string[]): string => {
  const [only] = terms;
  if (terms.length !== 1 || only === undefined) {
    return `!(${terms.join(" && ")})`;
  }
  if (/^[\w.()]+ === /.test(only)) {
    return only.replace(" === ", " !== ");
  }
  if (/^[\w.()]+ !== /.test(only)) {
    return only.replace(" !== ", " === ");
  }
  if (/^![\w.()]+$/.test(only)) {
    return only.slice(1);
  }
  return /^[\w.()]+$/.test(only) ? `!${only}` : `!(${only})`;
};

interface Table {
  readonly leaves: readonly Leaf[];
  readonly fallback: Value;
  readonly usesState: boolean;
  readonly inputs: ReadonlySet<string>;
}

const FALSE: Value = { kind: "literal", text: "false" };

const tableOf = (leaves: readonly Leaf[]): Table => {
  const counts = new Map<string, { value: Value; count: number }>();
  for (const { value } of leaves) {
    const key = valueKey(value);
    counts.set(key, { value, count: (counts.get(key)?.count ?? 0) + 1 });
  }
  let fallback = counts.get(valueKey(FALSE)) ?? { value: leaves[0]?.value ?? FALSE, count: 0 };
  for (const each of counts.values()) {
    if (each.count > fallback.count) {
      fallback = each;
    }
  }
  const kept = leaves.filter(({ value }) => valueKey(value) !== valueKey(fallback.value));
  const values = [fallback.value, ...kept.map(({ value }) => value)];
  const atoms = kept.flatMap(({ path }) => path.map(([atom]) => atom));
  return {
    leaves: kept,
    fallback: fallback.value,
    usesState:
      values.some(({ kind }) => kind === "state") || atoms.some(({ scope }) => scope === "state"),
    inputs: new Set([
      ...values.flatMap((value) => (value.kind === "input" ? [value.name] : [])),
      ...atoms.filter(({ scope }) => scope === "input").map(({ name }) => name),
    ]),
  };
};

interface Parameter {
  readonly name: string;
  readonly type: string;
  readonly input: string | null;
}

interface Written {
  readonly expression: string;
  readonly usesState: boolean;
  readonly usesInput: boolean;
}

const ifLine = (terms: readonly string[], expression: string): string => {
  const head = `  if (${terms.join(" && ")})`;
  const line = `${head} return ${expression};`;
  if (line.length <= WIDTH) {
    return line;
  }
  return head.length <= WIDTH
    ? `${head}\n    return ${expression};`
    : `  if (\n    ${terms.join(" &&\n    ")}\n  )\n    return ${expression};`;
};

const helperOf = (
  name: string,
  table: Table,
  result: string,
  parameters: readonly Parameter[],
): { readonly text: string; readonly written: Written } => {
  const parameterOf = (input: string): string =>
    parameters.find((parameter) => parameter.input === input)?.name ?? input;
  const rendering: Rendering = {
    root: (atom) => (atom.scope === "state" ? `s.${atom.name}` : parameterOf(atom.name)),
    input: parameterOf,
  };
  const used = parameters.filter(({ input }) =>
    input === null ? table.usesState : table.inputs.has(input),
  );
  const typed = used.map(({ name: each, type }) => `${each}: ${type}`);
  const signature = `const ${name} = (${typed.join(", ")}): ${result} => {`;
  const head =
    signature.length <= WIDTH
      ? signature
      : `const ${name} = (\n${typed.map((each) => `  ${each},\n`).join("")}): ${result} => {`;
  const lines = table.leaves.map((leaf) =>
    ifLine(conditionOf(leaf, rendering), rendered(leaf.value, rendering)),
  );
  return {
    text: [head, ...lines, `  return ${rendered(table.fallback, rendering)};`, "};"].join("\n"),
    written: {
      expression: `${name}(${used.map(({ input }) => (input === null ? "s" : `input.${input}`)).join(", ")})`,
      usesState: used.some(({ input }) => input === null),
      usesInput: used.some(({ input }) => input !== null),
    },
  };
};

const INLINE: Rendering = {
  root: (atom) => (atom.scope === "state" ? `s.${atom.name}` : `input.${atom.name}`),
  input: (name) => `input.${name}`,
};

const MOST_INLINE = 60;

const writeTable = (
  leaves: readonly Leaf[],
  boolean: boolean,
  helper: (table: Table) => { readonly text: string; readonly written: Written },
  helpers: string[],
): Written => {
  const table = tableOf(leaves);
  const usesInput = table.inputs.size > 0;
  if (table.leaves.length === 0) {
    return { expression: rendered(table.fallback, INLINE), usesState: table.usesState, usesInput };
  }
  const [only] = table.leaves;
  if (boolean && table.leaves.length === 1 && only !== undefined) {
    const terms = conditionOf(only, INLINE);
    const expression =
      valueKey(table.fallback) === valueKey(FALSE) ? terms.join(" && ") : negated(terms);
    if (expression.length <= MOST_INLINE) {
      return { expression, usesState: table.usesState, usesInput };
    }
  }
  const { text, written } = helper(table);
  helpers.push(text);
  return written;
};

const arrow = (uses: readonly Written[], body: string): string => {
  const state = uses.some(({ usesState }) => usesState);
  const input = uses.some(({ usesInput }) => usesInput);
  const parameters = input ? (state ? "(s, input)" : "(_, input)") : state ? "(s)" : "()";
  return `${parameters} => ${body}`;
};

const callLines = (head: string, items: readonly string[], tail: string): string => {
  const line = `${head}${items.join(", ")}${tail}`;
  return line.length <= WIDTH ? line : `${head}\n  ${items.join(",\n  ")},\n${tail}`;
};

const write = (
  machine: AnyStateMachine,
  options: ModelFileOptions<AnyStateMachine>,
): MachineModelFile => {
  const modelName: unknown = options.name;
  if (typeof modelName !== "string" || !isName(modelName)) {
    throw new TypeError(
      `${CALLER} can't take name: it is ${described(modelName)}, not an identifier that can name the model, such as Turnstile`,
    );
  }
  const kinds = eventKindsOf(options.events);
  const contextFields: Fields =
    options.context === undefined
      ? []
      : plainFields(
          CALLER,
          "context",
          fieldRecord(
            CALLER,
            "context",
            options.context,
            `a record of context fields; ${EACH_FIELD}, such as { visitors: m.int(0, 3) }`,
          ),
        );
  const ruleEntries = Object.entries(options.rules ?? {});
  const contextCount = countOf(contextFields.map(([, declared]) => declared));
  const eventCount = kinds.reduce(
    (total, { fields }) => total + countOf(fields.map(([, declared]) => declared)),
    0,
  );
  const rowsPerState = contextCount * Math.max(1, eventCount + ruleEntries.length);
  const tooMany = (states: number): RangeError =>
    new RangeError(
      `${CALLER} can't write a table of ${states * rowsPerState} rows or more, past its limit of ${MOST_TABLE_ROWS}: ${states} × ${contextCount} × ${Math.max(1, eventCount + ruleEntries.length)}, the states of the machine found, the combinations of context values, and the events with each payload and the rules; declare fewer values`,
    );
  if (rowsPerState > MOST_TABLE_ROWS) {
    throw tooMany(1);
  }
  const context = fieldsOf("context", contextFields);
  const events = kinds.map(({ type, fields }) => {
    const inputs = fieldsOf(`events.${type}`, fields);
    return { type, inputs, payloads: rowsOf(inputs) };
  });
  const contexts = rowsOf(context);
  const declaredNames = new Set(context.map(({ name }) => name));

  const contextRow = (snapshot: AnyMachineSnapshot, where: string): Row => {
    const held: unknown = snapshot.context;
    const extra = Object.keys(typeof held === "object" && held !== null ? held : {}).find(
      (name) => fieldOf(held, name) !== undefined && !declaredNames.has(name),
    );
    if (extra !== undefined) {
      throw new TypeError(
        `${CALLER} can't write the context field \`${extra}\` ${where}: it has no declaration in context; declare its values, such as context: { ${extra}: m.int(0, 3) }`,
      );
    }
    return Object.fromEntries(
      context.map(({ name, declared, shape }) => {
        const value = written(shape, fieldOf(held, name));
        if (value === NOT_WRITTEN) {
          throw new TypeError(
            `${CALLER} can't write the context field \`${name}\` ${where}: it holds ${described(fieldOf(held, name))}, which a field declared ${declared.source} can't hold`,
          );
        }
        return [name, value];
      }),
    );
  };

  const scope = quietScope();
  const initial: AnyMachineSnapshot = machine.getInitialSnapshot(scope, options.input);
  if (initial.status === "error") {
    throw initial.error;
  }
  const start = contextRow(initial, "of the initial snapshot");
  const missing = new Set(
    context.flatMap(({ name }) => (fieldOf(initial.context, name) === undefined ? [name] : [])),
  );
  const contextFor = (row: Row): Row =>
    structuredClone(
      Object.fromEntries(
        Object.entries(row).filter(([name, value]) => value !== null || !missing.has(name)),
      ),
    );
  for (const { name, declared } of context) {
    if (!declared.has(start[name])) {
      throw new RangeError(
        `${CALLER} can't write the initial context field \`${name}\`: it is ${literal(start[name])}, outside ${declared.source}, and a model starts inside its declarations`,
      );
    }
  }

  const configurations: Configuration[] = [];
  const known = new Map<string, number>();
  const configurationOf = (snapshot: AnyMachineSnapshot): number => {
    const history = historyOf(snapshot);
    const key = keyOf({ value: snapshot.value, status: snapshot.status, history });
    const found = known.get(key);
    if (found !== undefined) {
      return found;
    }
    if ((configurations.length + 1) * rowsPerState > MOST_TABLE_ROWS) {
      throw tooMany(configurations.length + 1);
    }
    known.set(key, configurations.length);
    configurations.push({
      value: snapshot.value,
      status: snapshot.status,
      historyValue: snapshot.historyValue,
      history,
    });
    return configurations.length - 1;
  };
  configurationOf(initial);

  const threw: { readonly state: number; readonly throw: Omit<MachineThrow, "state"> }[] = [];
  const outcomes = events.map((): Outcome[][] => []);
  const verdicts = ruleEntries.map((): boolean[][] => []);
  for (let state = 0; state < configurations.length; state += 1) {
    const configuration = configurations[state];
    if (configuration === undefined) {
      break;
    }
    const rows = events.map((): Outcome[] => []);
    const judged = ruleEntries.map((): boolean[] => []);
    for (const row of contexts) {
      const resolved = (): AnyMachineSnapshot =>
        machine.resolveState({
          value: configuration.value,
          context: contextFor(row),
          historyValue: configuration.historyValue,
          status: configuration.status,
        });
      events.forEach(({ type, payloads }, kind) => {
        for (const payload of payloads) {
          const event: EventObject = { ...structuredClone(payload), type };
          const snapshot = resolved();
          let next: AnyMachineSnapshot | null = null;
          let error: unknown = null;
          try {
            if (snapshot.status === "active" && snapshot.can(event)) {
              next = machine.transition(snapshot, event, scope);
              if (next.status === "error") {
                error = next.error;
                next = null;
              }
            } else {
              rows[kind]?.push(OFF);
              continue;
            }
          } catch (caught) {
            error = caught;
          }
          if (next === null) {
            threw.push({
              state,
              throw: { part: "transition", id: type, context: row, payload, error },
            });
            rows[kind]?.push(THREW);
            continue;
          }
          rows[kind]?.push({
            kind: "on",
            state: configurationOf(next),
            context: contextRow(next, `after the event ${JSON.stringify(event)}`),
          });
        }
      });
      ruleEntries.forEach(([id, rule], index) => {
        let holds = false;
        try {
          holds = Boolean(rule.holds(resolved()));
        } catch (error) {
          threw.push({ state, throw: { part: "rule", id, context: row, payload: {}, error } });
        }
        judged[index]?.push(holds);
      });
    }
    rows.forEach((row, kind) => outcomes[kind]?.push(row));
    judged.forEach((row, index) => verdicts[index]?.push(row));
  }

  const labels = labelsOf(machine, configurations);
  const fieldNames = new Names(context.map(({ name }) => name));
  const control = fieldNames.claim("state");
  const threwField = threw.some(({ throw: { part } }) => part === "transition")
    ? fieldNames.claim("threw")
    : null;
  const names = new Names([modelName]);
  const actionIds = events.map(({ type }) => names.claim(type));
  const ruleIds = ruleEntries.map(([id]) => names.claim(id));
  const doneStates = configurations.flatMap((configuration, index) =>
    configuration.status === "done" ? [index] : [],
  );
  const endId = doneStates.length > 0 ? names.claim("done") : null;
  const statesConst = names.claim("States");

  const controlShape: Shape = { kind: "plain", type: "string", nullable: false };
  const controlAtoms = atomsOf(
    "state",
    control,
    controlShape,
    (point) => labels[point.state],
    (root) => root,
    () => [],
  );
  const contextAtoms = context.flatMap((field) =>
    fieldAtoms("state", field, (point) => point.context[field.name]),
  );
  const statePoints = configurations.flatMap((_, state) =>
    contexts.map((row): Point => ({ state, context: row, payload: {} })),
  );
  const NOTHING: ReadonlySet<string> = new Set();
  const booleanOutput = (value: boolean): Output => ({
    literal: String(value),
    matches: NOTHING,
  });

  const helpers: string[] = [];
  const declarationConsts: string[] = [];
  const stateParameter: Parameter = { name: "s", type: "State", input: null };

  const actionTexts = events.flatMap(({ type, inputs, payloads }, kind) => {
    const id = actionIds[kind] ?? type;
    const table = outcomes[kind] ?? [];
    const points: Point[] = [];
    const results: Outcome[] = [];
    table.forEach((rows, state) => {
      contexts.forEach((row, at) => {
        payloads.forEach((payload, index) => {
          points.push({ state, context: row, payload });
          results.push(rows[at * payloads.length + index] ?? OFF);
        });
      });
    });
    if (results.every(({ kind: each }) => each === "off")) {
      return [];
    }
    const inputConsts = inputs.map(({ name, declared }) => {
      const constName = names.claim(`${id}_${name}`);
      declarationConsts.push(`const ${constName} = ${declared.source};`);
      return { name, constName };
    });
    const inputNames = new Names([]);
    const parameters: readonly Parameter[] = [
      stateParameter,
      ...inputConsts.map(({ name, constName }) => ({
        name: inputNames.claim(name),
        type: `m.Infer<typeof ${constName}>`,
        input: name,
      })),
    ];
    const atoms = [
      ...controlAtoms,
      ...contextAtoms,
      ...inputs.flatMap((field) =>
        fieldAtoms("input", field, (point) => point.payload[field.name]),
      ),
    ];
    const makeHelper = (suffix: string, result: string) => (table: Table) =>
      helperOf(names.claim(`${id}_${suffix}`), table, result, parameters);
    const when = writeTable(
      grow(
        atoms,
        points,
        results.map(({ kind: each }) => booleanOutput(each !== "off")),
        [],
      ),
      true,
      makeHelper("when", "boolean"),
      helpers,
    );
    const valueOutput = (
      value: unknown,
      field: string,
      current: unknown,
      payload: Row | null,
      fitting: ReadonlySet<string>,
    ): Output => {
      const key = keyOf(value);
      return {
        literal: literal(value),
        matches: new Set([
          ...(keyOf(current) === key ? [`state:${field}`] : []),
          ...(payload === null
            ? []
            : inputs.flatMap(({ name }) =>
                fitting.has(name) && keyOf(payload[name]) === key ? [`input:${name}`] : [],
              )),
        ]),
      };
    };
    const changed: [string, Written][] = [];
    const fieldChange = (
      name: string,
      result: string,
      output: (outcome: Outcome, point: Point) => Output | null,
      candidates: readonly Value[],
    ): void => {
      const outputs = results.map((outcome, index) => {
        const point = points[index];
        return point === undefined ? null : output(outcome, point);
      });
      if (outputs.every((each) => each === null || each.matches.has(`state:${name}`))) {
        return;
      }
      changed.push([
        name,
        writeTable(
          grow(atoms, points, outputs, candidates),
          false,
          makeHelper(`next_${name}`, result),
          helpers,
        ),
      ]);
    };
    fieldChange(
      control,
      `State["${control}"]`,
      (outcome, point) =>
        outcome.kind === "on"
          ? valueOutput(labels[outcome.state], control, labels[point.state], null, NOTHING)
          : null,
      [{ kind: "state", name: control }],
    );
    for (const { name, declared } of context) {
      const fitting = new Set(
        inputs.flatMap((input) =>
          input.declared.values().every((value) => declared.has(value)) ? [input.name] : [],
        ),
      );
      fieldChange(
        name,
        `State["${name}"]`,
        (outcome, point) =>
          outcome.kind === "on"
            ? valueOutput(outcome.context[name], name, point.context[name], point.payload, fitting)
            : null,
        [
          { kind: "state", name },
          ...inputs.map((input): Value => ({ kind: "input", name: input.name })),
        ],
      );
    }
    if (threwField !== null) {
      fieldChange(
        threwField,
        `State["${threwField}"]`,
        (outcome) =>
          outcome.kind === "off"
            ? null
            : valueOutput(outcome.kind === "threw" ? 1 : 0, threwField, 0, null, NOTHING),
        [],
      );
    }
    const changes = changed.map(([name, { expression }]) => `${name}: ${expression}`);
    const changesLine = `  changes: ${arrow(
      changed.map(([, each]) => each),
      `({ ${changes.join(", ")} }),`,
    )}`;
    const changesText =
      changes.length === 0
        ? "  changes: () => ({}),"
        : changesLine.length <= WIDTH
          ? changesLine
          : `  changes: ${arrow(
              changed.map(([, each]) => each),
              `({\n${changes.map((change) => `    ${change},\n`).join("")}  }),`,
            )}`;
    const inputText =
      inputConsts.length === 0
        ? []
        : [
            `  input: { ${inputConsts.map(({ name, constName }) => `${name}: ${constName}`).join(", ")} },`,
          ];
    const whenText =
      when.expression === "true" ? [] : [`  when: ${arrow([when], when.expression)},`];
    return [
      [
        `export const ${id} = m.action(${modelName}, ${JSON.stringify(type)}, {`,
        ...inputText,
        ...whenText,
        changesText,
        "});",
      ].join("\n"),
    ];
  });

  const holdsText = (
    id: string,
    sentence: string,
    builder: string,
    truth: (point: Point, index: number) => boolean,
  ): string => {
    const holds = writeTable(
      grow(
        [...controlAtoms, ...contextAtoms],
        statePoints,
        statePoints.map((point, index) => booleanOutput(truth(point, index))),
        [],
      ),
      true,
      (table) => helperOf(names.claim(`${id}_holds`), table, "boolean", [stateParameter]),
      helpers,
    );
    return `${callLines(
      `export const ${id} = m.${builder}(`,
      [modelName, JSON.stringify(sentence), arrow([holds], holds.expression)],
      ");",
    )}`;
  };

  const ruleTexts = ruleEntries.map(([, rule], index) =>
    holdsText(
      ruleIds[index] ?? "",
      rule.says,
      "rule",
      (point, at) => verdicts[index]?.[point.state]?.[at % contexts.length] ?? false,
    ),
  );
  const endTexts =
    endId === null
      ? []
      : [
          holdsText(endId, "The machine is in a top-level final state", "end", (point) =>
            doneStates.includes(point.state),
          ),
        ];

  const enumLine = `m.enum([${labels.map((label) => JSON.stringify(label)).join(", ")}])`;
  const initialLabel = JSON.stringify(labels[0]);
  const controlLine = `  ${control}: ${enumLine}.initial(${initialLabel}),`;
  const statesText =
    controlLine.length <= WIDTH
      ? []
      : [
          `const ${statesConst} = m.enum([\n${labels.map((label) => `  ${JSON.stringify(label)},\n`).join("")}]);`,
        ];
  const modelText = [
    `export const ${modelName} = m.model(${JSON.stringify(modelName)}, {`,
    statesText.length === 0
      ? controlLine
      : `  ${control}: ${statesConst}.initial(${initialLabel}),`,
    ...context.map(
      ({ name, declared }) => `  ${name}: ${declared.source}.initial(${literal(start[name])}),`,
    ),
    ...(threwField === null ? [] : [`  ${threwField}: m.int(0, 0).initial(0),`]),
    "});",
  ].join("\n");

  const text = `${[
    'import { m } from "@earnestjs/model";',
    ...statesText,
    ...(declarationConsts.length === 0 ? [] : [declarationConsts.join("\n")]),
    ...helpers,
    modelText,
    ...actionTexts,
    ...ruleTexts,
    ...endTexts,
    `export type State = m.Infer<typeof ${modelName}>;`,
  ].join("\n\n")}\n`;

  return {
    text,
    threw: threw.map(({ state, throw: each }) => ({ ...each, state: labels[state] ?? "" })),
    actions: Object.fromEntries(
      events.flatMap(({ type }, kind) => {
        const id = actionIds[kind];
        return id === undefined ||
          !actionTexts.some((each) => each.startsWith(`export const ${id} =`))
          ? []
          : [[id, type]];
      }),
    ),
    rules: Object.fromEntries(ruleEntries.map(([id], index) => [ruleIds[index] ?? id, id])),
  };
};

/**
 * Writes an XState v5 machine out as the text of a `.model.ts` file, so `earnest verify` and
 * TLC can redo the search `checkMachine` does without running JavaScript. It takes what
 * `checkMachine` takes, and `name`, the model's name. Every value must be declared: each
 * context field in `context`, and each payload field in an `events` record; `events` may also be
 * a list of events without payloads. The machine's own `transition` is run with every declared
 * event and payload on every state value it can take, with every combination of declared context
 * values, not only the ones `checkMachine` reaches, and the results are written as tables. No
 * guard or `assign` is read as source, so they may call any code. The model has a field `state`
 * (`state_2` if the context has a `state`), a readable label for the state value and what the
 * history states remember, and one field per declared context field, starting where the initial
 * snapshot made with `input` starts. Each event type is an action, each rule a rule with the
 * same sentence, and a top-level final state an end. An id that is not a name in a model file,
 * such as `user.click`, is changed to one, such as `user_click`. An optional field's `null` is
 * given to the machine as a missing field when the initial context lacks the field or holds
 * `undefined`, and as `null` otherwise; a missing field and `undefined` read back as `null`.
 * A rule is judged on a snapshot rebuilt with `resolveState`, which has no output and no child
 * actors, so a rule must not read them.
 * `maxStates` is not written; pass `--max-states` to `earnest verify`. The same machine and
 * options give the same text. Refused before any event is tried, with an error that names what
 * is wrong: a context field without a declaration, a field name that is not an identifier,
 * `events` given as a function or a list entry with a payload, a model name that is not an
 * identifier, an initial context outside its declarations, and context declarations and events
 * whose table has more than `MOST_TABLE_ROWS` rows for one state. A table that passes the limit
 * as states are found is refused then. A next context the declaration's type can't hold, such as
 * text in a field declared `m.int`, is refused with the event and the field; one of the right
 * type outside the declaration is written as it is, so `earnest verify` reports it out of range.
 */
export const modelFileOf = <TMachine extends AnyStateMachine>(
  machine: TMachine,
  options: NoInfer<ModelFileOptions<TMachine>>,
): MachineModelFile => write(machine, options);
