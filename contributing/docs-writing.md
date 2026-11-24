# Writing Earnest Documentation

This guide explains where documentation lives in this repository, how pages are written, and how docs stay useful to both people and coding agents.

## Where each kind of doc lives

Keep each document focused on one audience instead of putting everything in one file:

| Location | Audience and purpose |
|---|---|
| `README.md` | First-time visitor on GitHub. What Earnest is, why it exists, one benchmark chart, install, one short example, and links into the docs site. Keep it short and scannable |
| `apps/docs/src/content/docs/start/` | Getting started: agent quickstart (`/start/quickstart/`) and installation details (`/start/install/`) |
| `apps/docs/src/content/docs/tutorials/` | Step-by-step walkthroughs that teach one workflow from start to finish (TypeScript model, Rust model, pull request review) |
| `apps/docs/src/content/docs/reference/` | Exact contracts: CLI flags and file discovery, output/JSON/reports, TypeScript model rules, Rust model rules, primitives and ordering, TLA+ export |
| `apps/docs/src/content/docs/compare/` | Measured comparisons and trade-offs against TLA+ and Quint |
| `apps/docs/src/content/docs/changelog.md` | User-visible changes in nightly builds, newest first |
| `CONTRIBUTING.md` and `contributing/` | Contributors working on Earnest: workspace layout, checks, benchmarks, and docs guidelines. Keep contributor rules out of public user pages |
| `crates/*/README.md`, `packages/*/README.md` | Internal crate overview or npm package contract, linking to the docs site for shared reference material |

## Get to the point

Open each page with what the reader came to do:

- **Tutorials** state what the reader will build and what they need, then step through the code in order.
- **Reference pages** state what the page specifies in one or two sentences, then show the command, schema, or syntax immediately.
- **Frontmatter `description`** starts with a verb and says what the reader can do (for example, `"Export models to TLA+ modules and confirm earnest's state counts and verdicts against TLC or tla-rs."`). Do not repeat the description verbatim as the opening sentence.

## Plain prose

Follow these writing rules on every page:

- **Short and direct.** Keep sentences under ~25 words and paragraphs under 6 sentences. Prefer active voice and familiar words.
- **Facts over fluff.** Skip marketing claims, rhetorical questions, vague adjectives, and invented scenarios. If a sentence could sit unchanged in another tool's docs, replace it with a concrete fact or measurement (`earnest checks 18 models in 5.9 to 11 ms`).
- **Real names.** Use exact identifiers from the code and CLI (`earnest verify`, `m.model`, `#[derive(Model)]`, `--max-states`) instead of vague phrases ("the verification command").
- **One word for one thing.** Call a model rule a "rule", an unreachable stop a "stuck state", and an evaluation error a "fault" everywhere.
- **Punctuation.** Prefer periods, commas, colons, or parentheses over em-dashes.
- **Action before code, result after.** Say what the command or snippet does before showing it, then explain the output, defaults, or limits after it.

## Examples

- Start with the smallest complete working example before showing optional features.
- Do not put explanatory comments inside code blocks. Explain the code in the surrounding prose instead.
- Include file titles (` ```ts title="src/turnstile.model.ts" `) when the reader creates or edits a named file.
- Use tables when comparing flags, types, exit codes, or compatibility rules.

## Links instead of duplication

Link to another page where it helps rather than repeating setup steps or reference tables:

- Tutorials introduce a concept with one concrete example and link to `/reference/...` for edge cases.
- `README.md` summarizes install and one example and links to `https://earnest.cpeaustriajc.dev` for everything else.
- Package READMEs explain package-specific behavior and link to the docs site for the shared model contract.

## Docs for AI agents

`apps/docs` uses `starlight-llms-txt` to generate agent-readable Markdown on every build:

- `/llms.txt`: index of documentation sets.
- `/_llms-txt/typescript-models.txt`: focused bundle for writing and checking TypeScript models.
- `/_llms-txt/rust-models.txt`: focused bundle for writing and checking Rust models.
- `/llms-full.txt` and `/llms-small.txt`: full and abridged site bundles.
- `/prompt.md`: self-contained quickstart prompt that points an agent to the language bundle matching the user's project.

When you add, rename, or remove a page under `apps/docs/src/content/docs/`:

1. Update the `sidebar` in `apps/docs/astro.config.mjs`.
2. Update `customSets` and `promote` in `apps/docs/astro.config.mjs` so agents still get the right pages without loading unrelated content.
3. Update `apps/docs/src/prompt.md` if any URL or file name it cites changed.
4. Do not use raw LaTeX (`$...$`) in Starlight pages; write plain text or inline code (for example, `` `2^53 - 1` ``).

## Check before finishing

1. Verify every flag, default, error message, and behavior you document against the actual source code and tests (`apps/earnest/src/`, `crates/`, `packages/`).
2. Run `just docs` and confirm the build succeeds with zero broken imports or warnings on changed pages.
3. Inspect `apps/docs/dist/llms.txt` and `apps/docs/dist/_llms-txt/` to confirm agent bundles include the expected pages and links.
4. Reread the changed page from top to bottom: can a reader or agent accomplish the task without unexplained terms, broken links, or missing defaults?
