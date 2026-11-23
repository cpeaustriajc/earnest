# Same answers

A change to the search must not change what the checker answers: the same outcome, states, ids,
paths and report, whatever the number of workers and wherever a state limit stops it.

```sh
just same-answers <commit>
```

builds the release `earnest` of the commit (in a worktree of its own, into `target/reference`) and
of the working tree, and compares their output with the times taken out:

- the sweep: the three models in `models/`, each checked by `verify` and by `report` at state
  limits within 2 of the end of every level and at six places inside every level that follows one
  of 512 states or more, by the tree at 1, 2, 4 and 10 workers against the commit at 1 worker.
  `models/levels.json` holds the first and last state id of every level of each model. The three
  are small on purpose: a run takes milliseconds, so every limit can be tried.
- every model file of `tasks/models`, `tasks/benchmark/models` and `tasks/conformance`, by `verify`
  and by `report --max-arrows 100000`, at 1 and at 4 workers, the tree against the commit with the
  same number of workers.

It prints one line for each part, with the runs of the tree and the differences:

| Part | Runs |
|---|---:|
| sweep of `big4.model.ts` | 1,200 |
| sweep of `c1.model.ts` | 792 |
| sweep of `l1.model.ts` | 656 |
| every model file | 328 |

then the first differences, and it fails on any. Compare with the commit before the change. It takes
about half an hour on 4 cores; it is not part of `just check`.
