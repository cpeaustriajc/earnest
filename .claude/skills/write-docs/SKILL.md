---
name: write-docs
description: >
  Write or review Earnest documentation in README.md, CONTRIBUTING.md, and apps/docs/src/content/docs.
  Use when adding a page, updating CLI or language reference docs, trimming README bloat, or
  keeping llms.txt and prompt.md in sync with code changes.
---

# Write Earnest Docs

Keep Earnest's documentation accurate, concise, and useful to both people and coding agents.

Size the work to the request: for a small fix, read the target section and verify the claim against code; for a new page or restructure, follow every step below.

## 1. Read the style guide and neighboring pages

Read `contributing/docs-writing.md` first. Then read the page you are changing and its neighbors in `apps/docs/src/content/docs/` so terminology, tone, and links fit smoothly.

Decide what belongs where:

- `README.md` stays short and scannable (front door only).
- Detailed flags, syntax rules, edge cases, and schemas go in `apps/docs/src/content/docs/reference/`.
- Repo layout and contributor workflows go in `CONTRIBUTING.md`.

## 2. Verify against the code

Never guess a flag name, default value, error message, or language rule from memory or old prose. Check the implementation and tests before writing:

- CLI flags and defaults: `apps/earnest/src/arguments.rs`
- Model discovery and output formatting: `apps/earnest/src/discovery.rs`, `print.rs`, `changes.rs`
- TypeScript and Rust readers: `crates/earnest_ts/`, `crates/earnest_rs/`, `crates/earnest_semantic/`
- Libraries: `packages/model/`, `packages/cli/`, `crates/earnest/`

Keep planned capabilities clearly separated from shipped behavior.

## 3. Write and wire links

Apply `contributing/docs-writing.md`:

- Frontmatter `description` starts with a verb and says what the reader can do.
- Plain, active prose: short sentences, real code names, concrete measures, no fluff, no comments inside code examples, no raw LaTeX `$...$`.
- Link instead of repeating setup or reference tables across pages.
- If you add, rename, or move a page, update:
  - `sidebar`, `promote`, and `customSets` in `apps/docs/astro.config.mjs`
  - any incoming links in `README.md`, `apps/docs/src/prompt.md`, tutorials, and package READMEs

## 4. Build and check

Run `just docs` and verify:

- The Starlight build completes cleanly.
- `apps/docs/dist/llms.txt` and `apps/docs/dist/_llms-txt/*.txt` list and include the updated pages with reasonable sizes.
- Every internal link and anchor resolves to a real page and heading.

Report what changed, what was verified against the code, and the result of `just docs`.
