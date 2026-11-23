# Times the checker and, where one applies, a floor in turn: one uncounted run of each per case, then
# ROUNDS rounds, and in each round the checker and then the floor. After time_runs.py of
# measure/profile-lone, with the floor added as a second "build" whose command comes from a JSON file.
# Prints one JSON line per run (wall, user, sys, cpu, peak MiB, minor faults, exit, and the state count
# and status; for a floor its own time_s, run_cpu_s, run_sys_s and set checksum), then a summary per case.
# Usage: python3 time_turn.py ROUNDS CHECKER MODEL:JOBS,... FLOORS.json      (run in the model folder)
#   FLOORS.json: {"grid_10m:1": {"env": {"HUGE": "1"}, "argv": ["/SCRATCH/bin/floor_par", "7", "serial"]}, ...}
import json, os, statistics, sys, tempfile, time

ROUNDS = int(sys.argv[1])
CHECKER = sys.argv[2]
CASES = [(c.split(":")[0], int(c.split(":")[1])) for c in sys.argv[3].split(",")]
FLOORS = json.load(open(sys.argv[4])) if len(sys.argv) > 4 else {}


def spawn(argv, env):
    with tempfile.TemporaryFile() as out:
        start = time.perf_counter()
        pid = os.fork()
        if pid == 0:
            os.dup2(out.fileno(), 1)
            os.execve(argv[0], argv, {**os.environ, **env})
        _, status, usage = os.wait4(pid, 0)
        wall = time.perf_counter() - start
        out.seek(0)
        text = out.read().decode(errors="replace")
    return text, dict(wall=round(wall, 3), user=round(usage.ru_utime, 3), sys=round(usage.ru_stime, 3),
                      cpu=round(usage.ru_utime + usage.ru_stime, 3), mib=round(usage.ru_maxrss / 1024, 1),
                      exit=status, minflt=usage.ru_minflt, invol=usage.ru_nivcsw, vol=usage.ru_nvcsw)


def checker(model, jobs):
    text, r = spawn([CHECKER, "verify", "--no-cache", "--json", "--jobs", str(jobs), "--max-states", "200000000",
                     model + ".model.ts"], {})
    try:
        m = json.loads(text)["models"][0]
        r.update(status=m["status"], states=m["states"], ms=m["milliseconds"])
    except Exception:  # noqa
        r.update(status="unreadable", states=None, ms=None)
    return r


def floor(spec):
    text, r = spawn(spec["argv"], spec.get("env", {}))
    kv = dict(l.split("=", 1) for l in text.splitlines() if "=" in l and " " not in l.split("=", 1)[0])
    r.update(status="floor", states=int(kv.get("states", -1)), levels=int(kv.get("levels", -1)),
             time_s=float(kv.get("time_s", "nan")), run_cpu_s=float(kv.get("run_cpu_s", "nan")),
             run_sys_s=float(kv.get("run_sys_s", "nan")), set_checksum=kv.get("set_checksum"),
             bad_parents=kv.get("bad_parents"), gen_cpu_s=float(kv["gen_cpu_s"]) if "gen_cpu_s" in kv else None)
    return r


def p(**kw):
    print(json.dumps(kw), flush=True)


p(event="start", loadavg=open("/proc/loadavg").read().strip(), utc=time.strftime("%FT%TZ", time.gmtime()),
  checker=CHECKER, floors=FLOORS)
for model, jobs in CASES:
    key = f"{model}:{jobs}"
    spec = FLOORS.get(key)
    p(event="warmup", build="checker", model=model, jobs=jobs, **checker(model, jobs))
    if spec:
        p(event="warmup", build="floor", model=model, jobs=jobs, **floor(spec))
    seen = {"checker": []} | ({"floor": []} if spec else {})
    for r in range(1, ROUNDS + 1):
        res = checker(model, jobs)
        seen["checker"].append(res)
        p(event="run", build="checker", model=model, jobs=jobs, round=r, **res)
        if spec:
            res = floor(spec)
            seen["floor"].append(res)
            p(event="run", build="floor", model=model, jobs=jobs, round=r, **res)
    for name, runs in seen.items():
        def mm(k):
            v = [x[k] for x in runs if x.get(k) is not None]
            return [round(statistics.median(v), 3), min(v), max(v)] if v else None
        p(event="summary", build=name, model=model, jobs=jobs, n=len(runs),
          wall=mm("wall"), cpu=mm("cpu"), user=mm("user"), sys=mm("sys"), mib=mm("mib"), minflt=mm("minflt"),
          time_s=mm("time_s"), run_cpu_s=mm("run_cpu_s"), run_sys_s=mm("run_sys_s"),
          statuses=sorted({(x["status"], x["states"], x["exit"], x.get("set_checksum")) for x in runs}))
p(event="end", loadavg=open("/proc/loadavg").read().strip(), utc=time.strftime("%FT%TZ", time.gmtime()))
