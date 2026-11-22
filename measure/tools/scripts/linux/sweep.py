import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

TIP, REF = os.environ["TIP"], os.environ["REF"]
SWEEP = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "models", "sweep")
MODES = {"verify": ["verify", "--no-cache", "--json"], "report": ["report", "--max-arrows", "100000"]}
JOBS = (1, 2, 4, 10)


def norm(s):
    s = re.sub(r'"milliseconds": \d+', '"milliseconds": 0', s)
    return re.sub(r'\d+(\.\d+)? ?(ms|s)\b', 'T', s)


def run(binary, args, path):
    p = subprocess.run([binary, *args, path], capture_output=True, text=True, timeout=900, cwd=os.path.dirname(path))
    return p.returncode, norm(p.stdout), p.stderr


def limits_for(lv, inside):
    out = set()
    for e in sorted({e for _, e in lv}):
        out.update(e + d for d in range(-2, 3) if e + d >= 1)
    for (s0, e0), (s, e) in zip(lv, lv[1:]):
        if e0 - s0 < 512:
            continue
        for k in range(1, inside + 1):
            x = s + ((e - s) * k) // (inside + 1)
            out.update([x, x + 1])
    return sorted(out)


def one(job):
    path, limit, mode = job
    args = MODES[mode] + ["--max-states", str(limit)]
    ref = run(REF, args + ["--jobs", "1"], path)
    bad = []
    for jobs in JOBS:
        tip = run(TIP, args + ["--jobs", str(jobs)], path)
        if tip[0] != ref[0] or tip[1] != ref[1] or "panick" in tip[2]:
            bad.append((limit, mode, jobs))
    return bad, len(ref[1])


name = sys.argv[1]
path = os.path.join(SWEEP, name)
levels = [tuple(l) for l in json.load(open(os.path.join(SWEEP, "levels.json")))[name]]
limits = limits_for(levels, 6)
work = [(path, limit, mode) for limit in limits for mode in ("verify", "report")]
with ThreadPoolExecutor(4) as pool:
    done = list(pool.map(one, work))
bad = [b for found, _ in done for b in found]
print(json.dumps({"model": name, "levels": len(levels), "limits": len(limits), "runs": len(work) * len(JOBS),
                  "differences": len(bad), "first": bad[:10], "shortest reference output": min(n for _, n in done)}))
