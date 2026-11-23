"""Times builds of `earnest` in turn, run for run on one machine, so that they share its load.

Usage: python3 -I tasks/benchmark/turn.py old=<path>,new=<path> <rounds> <model>:<workers>[,...]

Each model is a path to a `.model.ts` file without its extension. For each model and number of
workers it runs every build once unmeasured, then the builds one after the other for each round,
and prints one line of JSON: for each build the sorted wall times, their median, the median CPU
time and the median peak memory.
"""
import json
import os
import statistics
import sys
import time

BUILDS = dict(pair.split("=", 1) for pair in sys.argv[1].split(","))
ROUNDS = int(sys.argv[2])
CASES = [(case.rsplit(":", 1)[0], int(case.rsplit(":", 1)[1])) for case in sys.argv[3].split(",")]
STATE_LIMIT = "200000000"


def run(binary, model, workers):
    start = time.perf_counter()
    with open(os.devnull, "w") as quiet:
        pid = os.posix_spawn(binary, [binary, "verify", "--no-cache", "--json", "--jobs", str(workers),
                                      "--max-states", STATE_LIMIT, model + ".model.ts"], os.environ,
                             file_actions=[(os.POSIX_SPAWN_DUP2, quiet.fileno(), 1)])
        _, status, usage = os.wait4(pid, 0)
    return time.perf_counter() - start, usage.ru_utime + usage.ru_stime, usage.ru_maxrss / 1024, status


for model, workers in CASES:
    for binary in BUILDS.values():
        run(binary, model, workers)
    seen = {name: [] for name in BUILDS}
    for _ in range(ROUNDS):
        for name, binary in BUILDS.items():
            seen[name].append(run(binary, model, workers))
    line = {"model": model, "workers": workers}
    for name, runs in seen.items():
        walls = sorted(each[0] for each in runs)
        line[name] = {"wall": [round(wall, 3) for wall in walls], "median": round(statistics.median(walls), 3),
                      "cpu": round(statistics.median(each[1] for each in runs), 2),
                      "mib": round(statistics.median(each[2] for each in runs), 1),
                      "status": sorted({each[3] for each in runs})}
    print(json.dumps(line), flush=True)
