import importlib.util, json, os, sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
SCRATCH = os.path.dirname(HERE)
spec = importlib.util.spec_from_file_location("sw5b", os.path.join(SCRATCH, "review5b", "sw5b.py"))
sw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sw)
TIP = os.environ["TIP"]
REF = sw.REF
JOBS = (1, 2, 4, 10)


def one(job):
    path, limit, mode = job
    args = sw.MODES[mode] + ["--max-states", str(limit)]
    ref = sw.run(REF, args + ["--jobs", "1"], path)
    bad = []
    for jobs in JOBS:
        tip = sw.run(TIP, args + ["--jobs", str(jobs)], path)
        if tip[0] != ref[0] or tip[1] != ref[1] or "panick" in tip[2]:
            bad.append((limit, mode, jobs))
    return bad


if __name__ == "__main__":
    path = os.path.abspath(sys.argv[1])
    limits, edge, levels = sw.limits_for(path, 6)
    work = [(path, limit, mode) for limit in limits for mode in ("verify", "report")]
    with ThreadPoolExecutor(4) as pool:
        bad = [b for found in pool.map(one, work) for b in found]
    widest = max(end - start for start, end in levels)
    print(json.dumps({"model": os.path.basename(path), "levels": len(levels), "widest level": widest,
                      "limits": len(limits), "runs": len(work) * len(JOBS), "differences": len(bad), "first": bad[:10]}))
