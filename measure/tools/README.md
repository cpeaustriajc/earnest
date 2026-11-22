# Tools of the speed work on `perf/joined`

This branch is not for merging. It carries the scripts and probe models that checked pull request 15
on a Mac, so that a session on another machine has them. Open items: issue 14.

The scripts are as they ran on the Mac. Their paths are placeholders: `/SCRATCH` and
`/SCRATCH_OLDER` were scratch folders, `/WORKTREES/perf-joined` was the checkout of `perf/joined`,
`/WORKTREES/perf-search` any checkout that holds `tasks/`. Port a script before you run it.

| File | What it does |
| --- | --- |
| `scripts/verify-tip.sh` | The full check of a tip: lint, `tests_kept.py` against `tests_kept_allowed.txt`, `just check`, the two tests with a time limit of 1 s eight times, the comparison, the sweeps, the counts, the breaks. |
| `scripts/compare.sh`, `scripts/compare_one.sh` | Print 1,744 runs of a new build and of a reference build (the build of `53d8497`) and count the differences: every model file of `tasks/models`, `tasks/benchmark/models` and `tasks/conformance` and the models in `models/own` in five modes at 1, 2, 4 and 10 workers, and the models in `models/older` and the benchmark models in two modes. Then the two models for the room at the state limit: `models/older/budget.model.ts` must fail with 2 states under `--max-states 2`, `models/spread.model.ts` with 2,050 states under `--max-states 2050`. |
| `scripts/sweep.py`, `scripts/sw5b.py` | Sweep `--max-states` around every level end and inside every level of a model in `models/sweep`, for `verify` and `report` at 1, 2, 4 and 10 workers, against the reference build. Expected: 792 runs on `c1`, 1,200 on `big4`, 656 on `l1`, 0 differences. |
| `models/sweep/levels.json`, `scripts/probe_patch.py` | `sw5b.py` reads the first and last state id of every level from a build with extra prints (`earnest-probe`), which exists only on the Mac. `levels.json` holds those pairs for the three sweep models, so read them from there. `probe_patch.py` made that build from an older source and no longer applies to the tip. |
| `scripts/measure.sh` | One run with the count of instructions and the peak memory. It reads macOS `/usr/bin/time -l`; Linux needs another source. |
| `scripts/measure-join.sh`, `scripts/both.sh`, `scripts/alternate.py` | Two builds measured on the same models, in turn. |
| `breaks/patch.py`, `breaks/run.sh` | Three deliberate breaks, each applied to a copy of the tree; each must make a test fail. |
| `scripts/readme_results.py` | Writes the table of `check` job runs in `tasks/benchmark/README.md`. It holds eight runs; `check-job-4c25766.log` is the ninth. |
| `models/corridor.model.ts` | 11,820,300 states in 20,000 levels of at most 600 states; ends in `fail`. Run with `--max-states 20000000`. |
| `models/lights.model.ts` | 3,000,000 states; its guards compare a text field with constants. |
| `models/grid_100m.model.ts` | 100 million states, about 1.4 GiB. |
