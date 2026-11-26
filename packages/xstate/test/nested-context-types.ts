import type { Declaration } from "@earnestjs/model";
import type { MachineContextSchema } from "@earnestjs/xstate";
import type { spotlight } from "./machines/spotlight.ts";

type Accepts<Target, Value> = [Value] extends [Target] ? true : false;

type Rejects<Target, Value> = [Value] extends [Target] ? false : true;

type Holds<Claim extends true> = Claim;

type Spotlight = MachineContextSchema<typeof spotlight>;

type Aisle<Side> = { readonly side: Side };

export type NestedOptionalChecks = [
  Holds<Accepts<Spotlight, { spot: Declaration<{ lit: boolean; dim: boolean | null }> }>>,
  Holds<Accepts<Spotlight, { spot: Declaration<{ lit: boolean; dim: boolean }> }>>,
  Holds<
    Accepts<
      Spotlight,
      { seat: Declaration<{ row: number | null; aisle: Aisle<"left" | "right" | null> } | null> }
    >
  >,
  Holds<Accepts<Spotlight, { seat: Declaration<{ row: 1 | 2; aisle: Aisle<"left"> }> }>>,
  Holds<Rejects<Spotlight, { spot: Declaration<{ lit: boolean | null; dim: boolean | null }> }>>,
  Holds<Rejects<Spotlight, { spot: Declaration<{ lit: boolean; dim: number | null }> }>>,
  Holds<Rejects<Spotlight, { spot: Declaration<{ lit: boolean; dimmed: boolean | null }> }>>,
  Holds<
    Rejects<Spotlight, { seat: Declaration<{ row: number | null; aisle: Aisle<"up" | null> }> }>
  >,
];
