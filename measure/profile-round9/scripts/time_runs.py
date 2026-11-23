# Times builds in turn, after turn.py of measure/tools: one uncounted run per case, then ROUNDS rounds
# of every build in turn. Prints one JSON line per run (wall, user, sys, peak RSS, exit status, and the
# status and state count from the --json output), then a summary per build and case.
# Usage: python3 time_runs.py name=/bin,name2=/bin ROUNDS model:jobs,model:jobs   (run in the model folder)
import json, os, statistics, subprocess, sys, tempfile, time

BUILDS = dict(pair.split("=", 1) for pair in sys.argv[1].split(","))
ROUNDS = int(sys.argv[2])
CASES = [(c.split(":")[0], int(c.split(":")[1])) for c in sys.argv[3].split(",")]


def run(binary, model, jobs):
    with tempfile.TemporaryFile() as out:
        start = time.perf_counter()
        pid = os.fork()
        if pid == 0:
            os.dup2(out.fileno(), 1)
            os.execv(binary, [binary, "verify", "--no-cache", "--json", "--jobs", str(jobs),
                              "--max-states", "200000000", model + ".model.ts"])
        _, status, usage = os.wait4(pid, 0)
        wall = time.perf_counter() - start
        out.seek(0)
        try:
            m = json.loads(out.read())["models"][0]
            st, n, ms = m["status"], m["states"], m["milliseconds"]
        except Exception as e:  # noqa
            st, n, ms = "unreadable", None, None
    return dict(wall=round(wall, 3), user=round(usage.ru_utime, 3), sys=round(usage.ru_stime, 3),
                cpu=round(usage.ru_utime + usage.ru_stime, 3), mib=round(usage.ru_maxrss / 1024, 1),
                exit=status, status=st, states=n, ms=ms, invol=usage.ru_nivcsw, vol=usage.ru_nvcsw,
                minflt=usage.ru_minflt)


def p(**kw):
    print(json.dumps(kw), flush=True)


p(event="start", loadavg=open("/proc/loadavg").read().strip(), utc=time.strftime("%FT%TZ", time.gmtime()))
for model, jobs in CASES:
    first = next(iter(BUILDS))
    p(event="warmup", build=first, model=model, jobs=jobs, **run(BUILDS[first], model, jobs))
    seen = {name: [] for name in BUILDS}
    for r in range(1, ROUNDS + 1):
        for name, binary in BUILDS.items():
            res = run(binary, model, jobs)
            seen[name].append(res)
            p(event="run", build=name, model=model, jobs=jobs, round=r, **res)
    for name, runs in seen.items():
        def mm(k):
            v = [x[k] for x in runs]
            return [round(statistics.median(v), 3), min(v), max(v)]
        p(event="summary", build=name, model=model, jobs=jobs, n=len(runs),
          wall=mm("wall"), cpu=mm("cpu"), user=mm("user"), sys=mm("sys"), mib=mm("mib"),
          statuses=sorted({(x["status"], x["states"], x["exit"]) for x in runs}))
p(event="end", loadavg=open("/proc/loadavg").read().strip(), utc=time.strftime("%FT%TZ", time.gmtime()))
