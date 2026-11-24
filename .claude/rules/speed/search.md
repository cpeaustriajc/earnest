---
paths:
  - "crates/earnest_check/src/**"
  - "crates/earnest_ir/src/**"
---

- Much of this code runs for every state of a search, so a change can make the search slower
  with every test still passing. When you change what a search runs, run `just counts` and
  `just benchmark` before you keep it. Each holds a budget, and no limit is raised to pass it.
- Run `just same-answers <commit>` against the commit before your change. It must find 0
  differences.
- For a change made for speed, read `tasks/benchmark/TRIED.md` first. It lists what was
  measured and not kept, with the numbers, and how to judge a speed change.
- Say what result keeps a speed change before you measure it, and take the change back when it
  misses.
