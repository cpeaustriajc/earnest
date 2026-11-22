import pathlib, re, sys

readme = pathlib.Path(sys.argv[1])
log = pathlib.Path(sys.argv[2])

lines = []
for raw in log.read_text().splitlines():
    parts = raw.split("\t", 2)
    text = parts[2] if len(parts) == 3 else raw
    lines.append(re.sub(r"^\S+Z ", "", text))
start = next(i for i, line in enumerate(lines) if line.startswith("Cores: "))
end = next(i for i, line in enumerate(lines) if i > start and line.startswith("- peak memory "))
printed = [("#" + line if line.startswith("## ") else line) for line in lines[start : end + 1]]

intro = """## Results

Last run: the release build of commit d6eb95e, `--runs 5`, in the `check` job of pull request 15.

Machine: a GitHub-hosted `ubuntu-latest` runner (image `ubuntu-26.04`), which GitHub documents as
4 vCPUs and 16 GB on x86-64 for a public repository. The benchmark counted 4 cores. The runner
shares its host, so other work there adds noise to every number below. Every later run of the
`check` job prints its own numbers in the step `just benchmark`.

Eight runs of the `check` job. The first four measured the same search code: their commits differ
only in tests, in this file and in the first line of the benchmark output, which names the
processor from 433b987 on. The run of 906e76c is the first with the search code that writes the
updates of an action into a copy of the packed row. The run of 2963116 is the first with the
search code that tries an action of one input combination without the loop over the inputs. The
run of 0ffa4d5 is the first with the search code that, near the state limit, hands out no more
parts of a level once the rows its threads kept outgrow the room that is left. The run of d6eb95e
is the first with the search code that merges a level on one thread for every 4,096 rows that the
parts of the level found, and that reads the clock once in 64 states under a time limit.

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

`grid_10m` with 4 workers took between 1.2 s and 2.4 s, against the target of under 5 s. The same
code took up to 1.9 times as long in one run as in another, so two builds can be compared only on
one machine. The runners have five kinds of processor, which explains a part of that spread. The
speed of a runner also changes within a run: the run of 433b987 shows a speed-up of 4.79 on 4
cores. The sections below are as the run of d6eb95e printed them.
"""

text = readme.read_text()
head = text[: text.index("## Results\n")]
readme.write_text(head + intro + "\n" + "\n".join(printed) + "\n")
