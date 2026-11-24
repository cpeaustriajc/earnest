# Rules for agents

- Never silence a lint. Change the code.
- Never write a comment, except the doc comment a lint requires on a public item. Never write a
  doc attribute in its place. Comments go stale: `// keep 3 weeks` above code that keeps 5
  misleads the next agent. Use names: `const WEEKS_KEPT: usize = 5;`
- Write each doc comment for the person who calls the item. Say what it is for and what the
  caller must know. Never restate the name.
- `just benchmark` holds a speed budget (`tasks/benchmark/README.md`). Never raise a limit to pass
  it: make the change faster or take it back.
- `just counts` is a budget of instructions: re-record it only on purpose and say why in the commit.
- Read `tasks/benchmark/TRIED.md` before speed changes; keep one only if `just same-answers` agrees.
- Follow `contributing/docs-writing.md` and `.claude/skills/write-docs/SKILL.md` when editing documentation.
- Run `just check` before you finish.
