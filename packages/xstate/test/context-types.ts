import type { Declaration } from "@earnestjs/model";
import type { MachineContextSchema } from "@earnestjs/xstate";
import type { dial, toggle } from "./machines/modes.ts";

type Accepts<Target, Value> = [Value] extends [Target] ? true : false;

type Rejects<Target, Value> = [Value] extends [Target] ? false : true;

type Holds<Claim extends true> = Claim;

type Toggle = MachineContextSchema<typeof toggle>;

type Dial = MachineContextSchema<typeof dial>;

export type UnionContextChecks = [
  Holds<Accepts<Toggle, { level: Declaration<number | null> }>>,
  Holds<Accepts<Toggle, { level: Declaration<number> }>>,
  Holds<Accepts<Toggle, { mode: Declaration<"on" | "off"> }>>,
  Holds<Accepts<Toggle, { mode: Declaration<"on">; level: Declaration<1 | 2 | null> }>>,
  Holds<Rejects<Toggle, { levl: Declaration<number | null> }>>,
  Holds<Rejects<Toggle, { level: Declaration<string | null> }>>,
  Holds<Rejects<Toggle, { mode: Declaration<"on" | "off" | null> }>>,
  Holds<Rejects<Toggle, { mode: Declaration<string> }>>,
];

export type OptionalContextChecks = [
  Holds<Accepts<Dial, { level: Declaration<number | null> }>>,
  Holds<Accepts<Dial, { level: Declaration<number> }>>,
  Holds<Accepts<Dial, { note: Declaration<string | null> }>>,
  Holds<Rejects<Dial, { level: Declaration<boolean | null> }>>,
  Holds<Rejects<Dial, { lvl: Declaration<number | null> }>>,
];
