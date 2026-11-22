import os, statistics, subprocess, sys, time, json

BUILDS = dict(pair.split("=", 1) for pair in sys.argv[1].split(","))
ROUNDS = int(sys.argv[2])
CASES = [(c.split(":")[0], int(c.split(":")[1])) for c in sys.argv[3].split(",")]
EXTRA = sys.argv[4:]


def run(binary, model, jobs):
    start = time.perf_counter()
    pid = os.spawnv(os.P_NOWAIT, binary, [binary, "verify", "--no-cache", "--json", "--jobs", str(jobs), "--max-states", "200000000", *EXTRA, model + ".model.ts"])
    _, status, usage = os.wait4(pid, 0)
    return time.perf_counter() - start, usage.ru_utime + usage.ru_stime, usage.ru_maxrss / 1024, status


print(open("/proc/loadavg").read().strip(), flush=True)
for model, jobs in CASES:
    sys.stdout.flush()
    devnull = os.open(os.devnull, os.O_WRONLY)
    saved = os.dup(1)
    os.dup2(devnull, 1)
    try:
        run(next(iter(BUILDS.values())), model, jobs)
        seen = {name: [] for name in BUILDS}
        for _ in range(ROUNDS):
            for name, binary in BUILDS.items():
                seen[name].append(run(binary, model, jobs))
    finally:
        os.dup2(saved, 1)
        os.close(saved)
        os.close(devnull)
    out = {"model": model, "jobs": jobs}
    for name, runs in seen.items():
        real = sorted(r[0] for r in runs)
        out[name] = {"real": [round(r, 3) for r in real], "median": round(statistics.median(real), 3),
                     "cpu": round(statistics.median(r[1] for r in runs), 2), "mib": round(statistics.median(r[2] for r in runs), 1),
                     "status": sorted({r[3] for r in runs})}
    print(json.dumps(out), flush=True)
print(open("/proc/loadavg").read().strip(), flush=True)
