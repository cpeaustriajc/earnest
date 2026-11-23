# Benchmark

A repeatable speed measurement of the release `earnest verify`. The spec sets two targets: the
first app's 12 models in under 5 s in all, and the slowest of them under 10 s alone. Those 12
models are not in this repository, so this task holds fixed models that stand in for them, and
the numbers below say how the checker does on those, not on the app's. The owner set a third
target: the model with 10,000,000 states (`grid_10m`) in under 5 s on 4 vCPUs and 16 GB, a Linux
x86-64 machine. That one is measured on the model itself, with no stand-in, and only a run on
such a machine says whether it is met.

```sh
cargo build --release --locked -p earnest-cli
cargo run --release --locked -p earnest_benchmark -- --runs 5
```

`grid_10m` is run with `--max-states 20000000`, because the checker stops a model at 5,000,000
states by default; the other cases use the default.

`--runs <n>` is the number of measured runs of each case (default 5, after one run that is not
counted) and `--binary <path>` the `earnest` to measure (default `target/release/earnest`). With
`--counts` it counts instructions in place of measuring time (see "Instructions" below), and with
`--counts --record` it writes those counts down. The binary runs `earnest verify --no-cache --json`
in an empty temporary folder, so nothing is served from a cache and nothing is written to the
repository, and prints Markdown:

- the scaled models in TypeScript and in Rust, with their median wall time, states per second
  and peak memory;
- the 12 mixed models, each alone;
- the two spec lines, marked as a stand-in, with the 12 models checked together in one
  `earnest verify`;
- the 10-million-state target: `grid_10m` with 4 workers (`--jobs 4`), its median wall time and
  peak memory, against "under 5 s on 4 cores and 16 GB", and whether the machine it ran on has
  at least 4 cores or is a stand-in with fewer;
- the largest model (`grid_10m`) on one worker (`--jobs 1`) and on all cores, with the speed-up;
- the R12 comparison: the 12 mixed models and the large model `grid_1m` in one `earnest verify`,
  against the longest of the 13 alone (`grid_1m` or `wide`, whichever takes longer). R12 asks
  for about 1x. It stays on `grid_1m` because it is about many models in one run, not about the
  biggest one.
- the budget: the 10-million-state target and the four limits below, each with its measurement.

Wall time is the time from starting the process to its exit, so it includes reading the files.
Search time is the `"milliseconds"` the checker reports for a model in its JSON: the time of the
search of that model alone, from building its table of states and its actions to the outcome,
without starting the process, reading and lowering the file or printing. The checker gives it in
whole milliseconds, cut down, so a search time can read up to 1 ms short: about 1% of a search of
100 ms, and a ratio of two such times can be off by the error of both.
Peak memory is the largest resident set (`VmHWM` in `/proc`) any run reached; it is `n/a` where
`/proc` has no such file. The binary fails, with the model and both numbers, when the checker's
state count for a model differs from the one derived below, when a model is not `pass`, or when
a file is unreadable. It also fails, after it prints the result, when `grid_10m` with 4 workers
takes 5 s or more on a machine with at least 4 cores, or when a line of the budget below is
missed, with one line for each miss; with fewer cores it judges only the line on memory. It is
not part of `just check`; that runs only its unit tests, which also check that every derived
count below is in this file. The `check` workflow runs it after `just check`, on a runner with
4 vCPUs and 16 GB, so a change that misses the target or the budget fails there.

## Models and their state counts

The files are in `models/`. Each scaled model is also in Rust (`.rs` beside the `.model.ts`),
with the same names, fields and actions, and the same count. Every count is derived by hand from
the model and the contract in the root `README.md`, never from the checker.

| Model | File | States | Why that many states |
|---|---|---|---|
| `Grid1k` | `grid_1k` | 1,000 | Three wheels `a`, `b`, `c`, each 0 to 9, each turned up one at a time by its own action and never down. Every combination is reachable from (0, 0, 0): 10 × 10 × 10 = 1,000. Only (9, 9, 9) has no action and it is a marked end |
| `Grid100k` | `grid_100k` | 100,000 | The same with five wheels: 10^5 |
| `Grid1m` | `grid_1m` | 1,000,000 | The same with six wheels: 10^6. This is the large model of the R12 comparison |
| `Grid10m` | `grid_10m` | 10,000,000 | The same with seven wheels `a` to `g`: 10^7. This is the largest model |
| `Ledger` | `ledger` | 36,400 | A queue of capacity 5 over 3 letters and a record `{a, b}` of two numbers 0 to 9. A queue holds 0 to 5 letters, 3^k contents of k letters: 1 + 3 + 9 + 27 + 81 + 243 = 364. The record takes 10 × 10 = 100 values. Deliveries and reads change only the queue and the two bumps only the record, so every pair is reachable: 364 × 100 = 36,400. An empty queue can take a letter and any other can be read, so nothing is stuck |
| `Wide` | `wide` | 10,000 | `x` and `y` each 0 to 99. One action takes three inputs `a`, `b`, `c` of 0 to 9, 1,000 combinations in every state, and moves `x` to `(x + a) mod 100` and `y` to `(y + b + c) mod 100`. Choosing `a = 1` and `b = 1, c = 0` steps each by one, so each reaches every value, and `a` and `b + c` are chosen independently: 100 × 100 = 10,000. The action is always allowed, so nothing is stuck. The checker evaluates it 10,000 × 1,000 = 10^7 times |
| `NarrowLevels` | `narrow_levels` | 1,020,300 | Time `t` 0 to 1,999 and a walker `y` 0 to 599. Each step raises `t` by one and may also raise `y` by one, so after `t` steps `y` is any of 0 to the smaller of `t` and 599: the first 600 levels hold 1 + 2 + ... + 600 = 180,300 states and the other 1,400 hold 600 each, 840,000. 180,300 + 840,000 = 1,020,300, in 2,000 levels of at most 600 states. A state with `t` at 1,999 has no step and is a marked end |

The 12 mixed models that stand in for the app's 12 are the next ten (in `models/mixed/`) and
`Ledger` and `Wide` above. All are TypeScript.

| Model | File | States | Why that many states |
|---|---|---|---|
| `Counter` | `mixed/counter100` | 100 | `n` goes 0 to 99 one at a time, and 99 is a marked end. 100 |
| `Traffic` | `mixed/traffic` | 30 | `light` cycles through 3 values with an always allowed action, and `cars` goes 0 to 9 on its own. 3 × 10 = 30 |
| `Plane` | `mixed/plane` | 400 | `x` and `y` each go 0 to 19 on their own. 20 × 20 = 400. (19, 19) is a marked end |
| `Basket` | `mixed/basket` | 16 | Subsets of 4 items, each added when absent and removed when present: 2^4 = 16 |
| `Mailbox` | `mixed/mailbox4` | 31 | A queue of capacity 4 over 2 letters: 1 + 2 + 4 + 8 + 16 = 31 |
| `Trail` | `mixed/trail3` | 40 | A sequence of at most 3 over 3 stops, pushed to below 3 items and popped above 0: 1 + 3 + 9 + 27 = 40 |
| `Shelf` | `mixed/shelf` | 64 | A map from 3 items to a number 0 to 2. Each key is absent or maps to one of 3 numbers, so 4 values, and the action that sets and the one that deletes reach every combination: 4^3 = 64 |
| `Seats` | `mixed/seats` | 18 | `seat` has no initial value, so it starts at every record `{row 1 to 3, aisle}`: 3 × 2 = 6. `owner` has none either and starts at `null`, `Kim` or `Lee`: 3. Every combination is a start state and no action leaves them. 6 × 3 = 18 |
| `Votes` | `mixed/votes` | 21 | `Idle` (0, 0): 1. `Voting` with every `(yes, no)` of sum at most 3: 1 + 2 + 3 + 4 = 10. `Done`, closed from each of them: 10. 1 + 10 + 10 = 21 |
| `Pairs` | `mixed/pairs` | 120 | `(a, b)` of 0 to 14 with `a <= b`, reached by raising `b` and then `a` up to it: 15 + 14 + ... + 1 = 120 |

## Budget

The release this budget was set on must stay this fast; `just benchmark` fails on any line it
misses. Times are medians; "all cores" is the default number of workers.

| What | Measured when the budget was set | Limit |
|---|---|---|
| `grid_10m` with 4 workers, wall time: the product's target | 1.2 s to 2.4 s in ten runs of the `check` job | under 5 s |
| `grid_10m` with 1 worker, wall time | 3.7 s to 3.8 s in the `check` job, 3.9 s to 6.1 s on cloud machines | under 8 s |
| `grid_10m` peak memory, with 4 workers and with 1 worker | 196 MiB and 207 MiB | under 256 MiB |
| `narrow_levels`, search time with all cores over search time with 1 worker | about 1x, one measurement up to 1.37x, both in wall time | at most 1.5x |
| `wide`, search time with all cores over search time with 1 worker | about 0.33x in wall time | at most 0.75x |

Machines of the same size differed by more than two times on the same code in one day, so the two
times are loose and the memory and the two ratios, taken on one machine in one job, carry the
budget. The two ratios divide search times and the two times of `grid_10m` are wall times, which is
what a person waits for. A ratio of wall times would count starting the process and reading the
file in both runs, a part that no worker shortens, so a faster search would move the ratio towards
its limit; `wide` leaves room for four CPUs that are two cores of two threads. The benchmark wants
a machine that does nothing else; a missed line is measured once more and judged by the better of
the two. When a line fails, make the change faster or take it back. A limit changes only with the
owner's word. What was tried in the search and not kept, and how to judge a speed change, is in
`TRIED.md`.

### Instructions

A clock on a shared machine moves 5% to 10% between runs, and as much again when the compiler
merely places a loop elsewhere, so it cannot see a change of 1% to 5%; a count of instructions
can. `just counts` runs `earnest verify --no-cache --json --jobs <n>` on each case below under
callgrind (`valgrind --tool=callgrind`), counts the instructions executed inside the checker's own
binary, and compares each count with its line in `instructions.txt`. It prints a table and fails,
naming the case, when a count is more than its share above the record; a case above it is counted
once more and judged by the smaller count. The `check` workflow runs it after `just benchmark`.

| Case | Workers | Most above the record | Spread of ten runs when it was set |
|---|---:|---:|---:|
| `grid_1m` | 1 | 1% | 0.001% |
| `ledger` | 1 | 1% | 0.02% |
| `wide` | 1 | 1% | under 0.001% |
| `narrow_levels` | 1 | 1% | 0.002% |
| `mixed/shelf` | 1 | 2% | 0.42% |
| `wide` | 4 | 1% | 0.013% |
| `grid_1m` | 4 | 1% | 0.16% |

The spreads were taken with the model's absolute path; `just counts` copies the model into an
empty folder and names it by its file name, and with that every case with 1 worker repeats to the
instruction. With 4 workers the count moves with the order in which the threads take parts.
`mixed/shelf` is mostly the reading of a TypeScript file, a few million instructions, so it gets
2%; `mixed/pairs` moved 3.9% between runs and is not a case. Only the checker's binary is counted
because the C library picks its `memcpy` and kin by processor and differs between machines; it is
8% of `grid_1m` with 4 workers. What a count depends on besides the code: the compiler and the
lock file (both pinned) and the case itself. The number of processors the process may use and its
environment change nothing. Whether the binary built on a runner executes exactly the instructions
of one built elsewhere was not seen; the limits leave five times the spread or more for that.

`just record-counts` writes the counts of the current build to `instructions.txt`. Run it only when
the compiler changes or a change is meant to cost more or less, and say why in the commit: the new
numbers show in the diff. A share above changes only with the owner's word.

## Results

Last run: the release build of commit fc4d8fb, `--runs 5`, in the `check` job of pull request 15.

Machine: a GitHub-hosted `ubuntu-latest` runner (image `ubuntu-26.04`), which GitHub documents as
4 vCPUs and 16 GB on x86-64 for a public repository. The benchmark counted 4 cores. The runner
shares its host, so other work there adds noise to every number below. Every later run of the
`check` job prints its own numbers in the step `just benchmark`.

Seventeen runs of the `check` job. The first four measured the same search code: their commits
differ only in tests, in this file and in the first line of the benchmark output, which names the
processor from 433b987 on. The run of 906e76c is the first with the search code that writes the
updates of an action into a copy of the packed row. The run of 2963116 is the first with the
search code that tries an action of one input combination without the loop over the inputs. The
run of 0ffa4d5 is the first with the search code that, near the state limit, hands out no more
parts of a level once the rows its threads kept outgrow the room that is left. The run of d6eb95e
is the first with the search code that merges a level on one thread for every 4,096 rows that the
parts of the level found, and that reads the clock once in 64 states under a time limit. The
run of 4c25766 is the first built with Rust 1.100.0. The run of ca9e11a is the first with the
search code in which one thread takes a wide level in parts. The run of 59405c7 is the first with
the search code that cuts a level into parts by its work. The run of 58aacc6 is the first with the
budget above, and the first that prints the model `NarrowLevels` and the section "Budget". The
run of 10015fc is the first with the search code in which the claim reads a part shard by shard,
and the evaluation walks the input combinations of an action like an odometer and keeps the
answers of guards and updates. The run of 32210b3 is the first with the search code that runs
number expressions as a chain of steps and tells the rows of one state's action apart by the codes
it writes. The run of fc84df4 is the first whose two ratio lines of the budget divide search
times. The run of 6c76546 is the first with the search code in which one thread takes a wide level
in rounds of 32 parts. The run of fc4d8fb adds the fixes of the last review, which change nothing
that these models reach.

| Commit | Processor | `grid_10m`, 4 workers | `grid_10m`, 1 worker | `grid_1m` | `wide` |
|---|---|---:|---:|---:|---:|
| 34f2f29 | not printed | 1.526 s | 5.339 s | 0.147 s | 0.246 s |
| 4739dd8 | not printed | 2.376 s | 8.945 s | 0.237 s | 0.369 s |
| 433b987 | AMD EPYC 9V74 80-Core Processor | 2.139 s | 10.102 s | 0.197 s | 0.347 s |
| cb8f468 | Intel(R) Xeon(R) 6975P-C | 1.803 s | 5.675 s | 0.185 s | 0.275 s |
| 906e76c | Intel(R) Xeon(R) 6975P-C | 1.737 s | 6.168 s | 0.176 s | 0.235 s |
| 2963116 | AMD EPYC 7763 64-Core Processor | 2.124 s | 7.580 s | 0.208 s | 0.341 s |
| 0ffa4d5 | INTEL(R) XEON(R) PLATINUM 8573C | 1.824 s | 6.696 s | 0.193 s | 0.272 s |
| d6eb95e | AMD EPYC 9V45 96-Core Processor | 1.219 s | 3.764 s | 0.111 s | 0.169 s |
| 4c25766 | AMD EPYC 9V74 80-Core Processor | 1.658 s | 5.841 s | 0.155 s | 0.296 s |
| ca9e11a | AMD EPYC 9V74 80-Core Processor | 1.638 s | 3.783 s | 0.160 s | 0.287 s |
| 59405c7 | INTEL(R) XEON(R) PLATINUM 8573C | 1.852 s | 3.735 s | 0.176 s | 0.305 s |
| 58aacc6 | AMD EPYC 9V45 96-Core Processor | 1.328 s | 2.939 s | 0.107 s | 0.187 s |
| 10015fc | Intel(R) Xeon(R) 6975P-C | 1.263 s | 2.524 s | 0.126 s | 0.149 s |
| 32210b3 | Intel(R) Xeon(R) 6975P-C | 1.230 s | 2.504 s | 0.123 s | 0.123 s |
| fc84df4 | Intel(R) Xeon(R) 6975P-C | 1.228 s | 2.476 s | 0.123 s | 0.123 s |
| 6c76546 | AMD EPYC 9V74 80-Core Processor | 1.346 s | 3.163 s | 0.138 s | 0.148 s |
| fc4d8fb | INTEL(R) XEON(R) PLATINUM 8573C | 1.536 s | 3.070 s | 0.155 s | 0.159 s |

`grid_10m` with 4 workers took between 1.2 s and 2.4 s, against the target of under 5 s. The same
code took up to 1.9 times as long in one run as in another, so two builds can be compared only on
one machine. The runners have five kinds of processor, which explains a part of that spread. The
speed of a runner also changes within a run: the run of 433b987 shows a speed-up of 4.79 on 4
cores. The runs of 4c25766, ca9e11a and 6c76546, all built with Rust 1.100.0, had the same
processor, an AMD EPYC 9V74, and are the one fair line across this work on a runner: `grid_10m`
took 1.658 s, 1.638 s and 1.346 s with 4 workers and 5.841 s, 3.783 s and 3.163 s with 1 worker.
The sections below are as the run of fc4d8fb printed them.

Cores: 4 (INTEL(R) XEON(R) PLATINUM 8573C). Runs per model: 5 after one unmeasured run. Wall time includes starting the process and reading the files; peak memory is the largest resident set of any run.

### Scaled models

| Model | Language | States | Median wall time (s) | States per second | Peak memory (MiB) |
|---|---|---:|---:|---:|---:|
| Grid1k | TypeScript | 1,000 | 0.002 | 424,989 | 5.6 |
| Grid1k | Rust | 1,000 | 0.003 | 295,072 | 6.3 |
| Ledger | TypeScript | 36,400 | 0.038 | 949,920 | 7.0 |
| Ledger | Rust | 36,400 | 0.039 | 926,137 | 7.6 |
| Wide | TypeScript | 10,000 | 0.159 | 62,881 | 7.4 |
| Wide | Rust | 10,000 | 0.157 | 63,694 | 7.8 |
| NarrowLevels | TypeScript | 1,020,300 | 0.163 | 6,265,005 | 23.6 |
| NarrowLevels | Rust | 1,020,300 | 0.168 | 6,083,824 | 24.3 |
| Grid100k | TypeScript | 100,000 | 0.021 | 4,770,764 | 7.8 |
| Grid100k | Rust | 100,000 | 0.022 | 4,612,758 | 8.5 |
| Grid1m | TypeScript | 1,000,000 | 0.155 | 6,456,361 | 27.8 |
| Grid1m | Rust | 1,000,000 | 0.159 | 6,303,302 | 28.5 |
| Grid10m | TypeScript | 10,000,000 | 1.530 | 6,537,126 | 189.0 |
| Grid10m | Rust | 10,000,000 | 1.547 | 6,462,240 | 192.1 |

### The 12 mixed models, each alone

| Model | Language | States | Median wall time (s) | States per second | Peak memory (MiB) |
|---|---|---:|---:|---:|---:|
| counter100 | TypeScript | 100 | 0.002 | 42,607 | 5.7 |
| traffic | TypeScript | 30 | 0.002 | 12,992 | 5.8 |
| plane | TypeScript | 400 | 0.002 | 171,821 | 5.8 |
| basket | TypeScript | 16 | 0.002 | 6,875 | 5.7 |
| mailbox4 | TypeScript | 31 | 0.002 | 13,460 | 5.7 |
| trail3 | TypeScript | 40 | 0.002 | 17,278 | 5.7 |
| shelf | TypeScript | 64 | 0.002 | 27,187 | 5.6 |
| seats | TypeScript | 18 | 0.002 | 7,689 | 5.7 |
| votes | TypeScript | 21 | 0.002 | 9,067 | 5.8 |
| pairs | TypeScript | 120 | 0.002 | 50,933 | 5.8 |
| ledger | TypeScript | 36,400 | 0.038 | 949,920 | 7.0 |
| wide | TypeScript | 10,000 | 0.159 | 62,881 | 7.4 |

### Spec targets

- 12 models together: 0.190 s, target under 5 s (a stand-in: these benchmark models, not the first app's 12)
- slowest model alone: 0.159 s, target under 10 s (a stand-in: these benchmark models, not the first app's)
- the 12 models in that one run: 47,240 states, peak memory 8.1 MiB
- grid_10m with 4 workers: 1.536 s, peak memory 192.8 MiB, target under 5 s on 4 cores and 16 GB (this machine has 4 cores, at least the 4 of the target)

### One large model on one worker and on all cores

- grid_10m: 1 worker 3.070 s, 4 workers (all cores) 1.530 s, speed-up 2.01x

### R12: many models and one large model in one run

- the 13 models in one run: 0.309 s; the large model (grid_1m) alone: 0.155 s; the slowest of the 12 alone (wide): 0.159 s
- R12 asks for about as long as the largest model alone: the one run took 1.94x the longest of them alone (wide)
- peak memory 31.4 MiB

### Budget

- grid_10m with 4 workers, wall time: 1.536 s, target under 5.000 s: met
- grid_10m with 1 worker, wall time: 3.070 s, budget under 8.000 s: met
- grid_10m, peak memory, the larger of 4 workers and 1 worker (4 workers): 192.8 MiB, budget under 256.0 MiB: met
- narrow_levels, search time with all cores (0.159 s) over search time with 1 worker (0.159 s): 1.00x, budget at most 1.50x: met
- wide, search time with all cores (0.156 s) over search time with 1 worker (0.323 s): 0.48x, budget at most 0.75x: met
