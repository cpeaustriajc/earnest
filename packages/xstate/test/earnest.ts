import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import type { MachineCheck, MachineModelFile, MachinePath } from "@earnestjs/xstate";
import type { AnyStateMachine, EventObject } from "xstate";

export const SLOW = 900_000;

const REPOSITORY = resolve(import.meta.dirname, "../../..");

const targetFolder = (): string => process.env.CARGO_TARGET_DIR ?? join(REPOSITORY, "target");

export const hasCargo = spawnSync("cargo", ["--version"], { stdio: "ignore" }).status === 0;

export const NO_CARGO = "skipped when cargo is not on the PATH to build the earnest binary";

let built = "";

export const buildEarnest = (): string => {
  if (built === "") {
    execFileSync("cargo", ["build", "--release", "--locked", "-p", "earnest-cli"], {
      cwd: REPOSITORY,
      stdio: "pipe",
    });
    built = join(
      targetFolder(),
      "release",
      process.platform === "win32" ? "earnest.exe" : "earnest",
    );
    if (!existsSync(built)) {
      throw new Error(`cargo built earnest-cli but ${built} is not there`);
    }
  }
  return built;
};

interface Named {
  readonly name: string;
  readonly value: unknown;
}

interface Step {
  readonly action: string;
  readonly input: readonly Named[];
}

interface Path {
  readonly steps: readonly Step[];
}

interface Verified {
  readonly name: string;
  readonly status: string;
  readonly states: number;
  readonly rules: readonly {
    readonly id: string;
    readonly says: string;
    readonly verdict: string;
    readonly path: Path | null;
  }[];
  readonly findings: readonly { readonly kind: string; readonly path: Path }[];
}

export const verify = (text: string): Verified => {
  const folder = mkdtempSync(join(tmpdir(), "earnest-xstate-"));
  try {
    const file = join(folder, "machine.model.ts");
    writeFileSync(file, text);
    const run = spawnSync(buildEarnest(), ["verify", "--json", "--no-cache", file], {
      encoding: "utf8",
    });
    const report = JSON.parse(run.stdout) as {
      readonly models: readonly Verified[];
      readonly unreadable: readonly unknown[];
    };
    const [model, ...others] = report.models;
    if (model === undefined || others.length > 0 || report.unreadable.length > 0) {
      throw new Error(`earnest verify did not read one model: ${run.stdout}${run.stderr}`);
    }
    return model;
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
};

export const eventsOf = (written: MachineModelFile, path: Path): readonly EventObject[] =>
  path.steps.map(({ action, input }) => ({
    ...Object.fromEntries(input.map(({ name, value }) => [name, value])),
    type: written.actions[action] ?? `no action ${action}`,
  }));

export interface Summary {
  readonly status: string;
  readonly states: number;
  readonly rules: readonly {
    readonly id: string;
    readonly says: string;
    readonly verdict: string;
    readonly length: number | null;
  }[];
  readonly findings: readonly { readonly kind: string; readonly length: number }[];
}

export const summaryOfCheck = (found: MachineCheck<AnyStateMachine>): Summary => ({
  status: found.status,
  states: found.states,
  rules: found.rules.map(({ id, says, verdict, path }) => ({
    id,
    says,
    verdict,
    length: path === null ? null : path.events.length,
  })),
  findings: found.findings.map(({ kind, path }) => ({ kind, length: path.events.length })),
});

export const summaryOfVerify = (written: MachineModelFile, verified: Verified): Summary => ({
  status: verified.status,
  states: verified.states,
  rules: verified.rules.map(({ id, says, verdict, path }) => ({
    id: written.rules[id] ?? `no rule ${id}`,
    says,
    verdict,
    length: path === null ? null : path.steps.length,
  })),
  findings: verified.findings.map(({ kind, path }) => ({ kind, length: path.steps.length })),
});

export const pathEvents = (path: MachinePath<AnyStateMachine> | null) =>
  path === null ? null : path.events;
