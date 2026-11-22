---
title: Earnest and TLA+ on real apps
description: Four open-source apps, 18 models of their hidden state machines, checked by earnest and by TLC.
---

We gave an agent one sentence for each of four open-source apps: "find all implicit state machines
to the <app> using `earnest`". Nothing else: no models to start from, no hints about where to look.
Each agent cloned the app, read its code, wrote the machines it found as earnest models in the
app's own language and ran `earnest verify`. We then ran TLC on earnest's TLA+ export of every
model.

An implicit state machine is a status that many parts of the code change directly, with no one
place that says which changes are allowed. For example, zellij stores a tab switch that you press
while a new tab loads, and a different function applies it later.

## What the agents found

| App | Language | Commit | Models | Rules broken | Time for the agent |
|---|---|---|---|---|---|
| [zellij](https://github.com/zellij-org/zellij) | Rust | `62cfa6c` | 4 | 4 models | 11 min |
| [helix](https://github.com/helix-editor/helix) | Rust | `1a3fdfd` | 5 | 4 models | 12 min |
| [excalidraw](https://github.com/excalidraw/excalidraw) | TypeScript | `9c2e462` | 7 | 6 models | 15 min |
| [t3code](https://github.com/pingdotgg/t3code) | TypeScript | `9de4ec1` | 2 | 2 models | 17 min |

The agents also listed 66 more machines that they didn't model.

Each broken rule comes with the shortest path to it, step by step. We checked these in the code:

- **zellij, a tab switch goes one tab too far.** While a tab loads, `GoToTab` stores a 1-based
  index (`screen.rs:13032`). The code that applies it later adds 1 again (`screen.rs:12962`). Path:
  a new tab starts loading → you press `GoToTab 1` → the tab finishes, and the second tab is active.
- **helix, the register stays in the macro.** Typing `"a` and then `Q` to stop a recording leaves
  `"a` at the end of the macro, because `record_macro` drops only the last key (`commands.rs:6970`).
- **excalidraw, the crop editor stays open on a deleted image.** Delete never clears
  `croppingElementId` (`actionDeleteSelected.tsx:300`).
- **t3code, a run that ended starts again.** A `failed` run can go back to `preparing`
  (`Orchestrator.ts:12762`), while `isTerminalRunStatus` treats `failed` as final
  (`ThreadManagementService.ts:585`). We haven't shown what goes wrong because of it.

The other findings are traced in the code but not reproduced in a running app. None of them has
been reported to its project yet.

## Earnest and TLC on the same models

Earnest exports every model as TLA+, and TLC 1.8.0 checked each export. Times are wall-clock times
on a 10-core Apple silicon Mac, with TLC on one worker.

| App | Models | States per model | Same verdicts and states | earnest, per model | TLC, per model |
|---|---|---|---|---|---|
| zellij | 4 | 7–60 | 4 of 4 | 0 ms | 255–282 ms |
| helix | 5 | 43–148 | 5 of 5 | 0 ms | 272–326 ms |
| excalidraw | 7 | 16–76 | 7 of 7 | 0 ms | 260–279 ms |
| t3code | 2 | 33 | 2 of 2 | 0 ms | about 1 s each, with `earnest confirm` |

TLC agrees with earnest on every model. Real app machines are small: the largest has 148 states.
At that size the check takes no time, and most of TLC's time is Java starting up. The difference
that matters is what you write and who can write it.

## Quint

We ported two zellij models to Quint 0.33.1 by hand. `quint run`, a random simulator, found both
broken rules in about 0.2 s. `quint verify --backend tlc` checked all states in about 2.9 s. It
downloaded Apalache 0.62.4 (190 MB) on its first run, because it translates Quint to TLA+ through
Apalache.

## What differs

| | earnest | TLA+ with TLC | Quint |
|---|---|---|---|
| Language of the model | TypeScript or Rust, the app's own | TLA+ | Quint |
| Checked with the app's own type checker and linter | Yes | No | No |
| Setup | One binary, no Java, no network | Java and `tla2tools.jar` | Node, plus Java and Apalache for `verify` |
| Checks every state | Yes | Yes | Yes with `verify`. `run` samples paths |
| Checks "eventually" properties | No | Yes | Yes |
| Can export to TLA+ | Yes, and TLC confirms the result | Not applicable | Yes, through Apalache |

Earnest checks rules that must hold in every state, and states where nothing can happen. It
can't check that something happens eventually. For example, the t3code agent found that a run can
stay `waiting` after 5 failed checkpoint captures, and it couldn't model that one. For that kind
of property, export the model and use TLC.

## Limits of this comparison

- An agent wrote every model by reading the code, so a model can be wrong about the app. A broken
  rule is a lead to check in the code, not proof of a bug.
- "All" means what each agent's search found. Each agent modelled 2 to 7 machines and only listed
  the rest.
- One agent stopped on its first try because "the helix" named no repository. With
  `helix-editor/helix`, it went ahead.
