import type { Declaration } from "@earnestjs/model";
import type {
  MachineContextSchema,
  MachineEventSchema,
  MachinePayloadSchema,
} from "@earnestjs/xstate";
import type { visitorCount } from "./machines/visitor-count.ts";

type Accepts<Target, Value> = [Value] extends [Target] ? true : false;

type Rejects<Target, Value> = [Value] extends [Target] ? false : true;

type Holds<Claim extends true> = Claim;

type Context = MachineContextSchema<typeof visitorCount>;

type Events = MachineEventSchema<typeof visitorCount>;

export type ContextChecks = [
  Holds<Accepts<Context, { visitors: Declaration<number> }>>,
  Holds<Accepts<Context, { visitors: Declaration<1 | 2> }>>,
  Holds<Accepts<Context, {}>>,
  Holds<Rejects<Context, { visitorz: Declaration<number> }>>,
  Holds<Rejects<Context, { visitors: Declaration<boolean> }>>,
  Holds<Rejects<Context, { visitors: Declaration<number | null> }>>,
];

type Shapes = MachinePayloadSchema<{ type: "A"; x: number } | { type: "A"; y: string }>;

export type UnionChecks = [
  Holds<Accepts<Shapes, { x: Declaration<number> }>>,
  Holds<Accepts<Shapes, { y: Declaration<string> }>>,
  Holds<Rejects<Shapes, { x: Declaration<string> }>>,
];

export type EventChecks = [
  Holds<Accepts<Events, { ENTER: { group: Declaration<number> } }>>,
  Holds<Accepts<Events, {}>>,
  Holds<Rejects<Events, { ENTER: {} }>>,
  Holds<Rejects<Events, { ENTR: { group: Declaration<number> } }>>,
  Holds<Rejects<Events, { ENTER: { group: Declaration<string> } }>>,
];
