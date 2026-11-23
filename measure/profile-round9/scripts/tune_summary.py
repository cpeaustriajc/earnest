# Summarises a tune_floor.sh log: for each setting, the median time_s and run_cpu_s of its runs; the best
# few settings of each design and thread count.  Usage: python3 tune_summary.py LOG [TOP]
import collections, re, statistics, sys
runs = collections.defaultdict(list)
for line in open(sys.argv[1]):
    if " :: " not in line: continue
    cfg, out = line.split(" :: ", 1)
    kv = dict(x.split("=", 1) for x in out.split())
    if kv.get("states") is None: continue
    cfg = re.sub(r"\S*/floor_par ", "", cfg)
    runs[cfg].append((float(kv["time_s"]), float(kv["run_cpu_s"]), float(kv["run_sys_s"]), kv["set_checksum"], kv["bad_parents"]))
top = int(sys.argv[2]) if len(sys.argv) > 2 else 6
groups = collections.defaultdict(list)
for cfg, r in runs.items():
    w = cfg.split()
    design = "shared" if "shared" in w else "parts"
    t = w[-1]; huge = next(x for x in w if x.startswith("HUGE="))
    groups[(design, t, huge)].append((statistics.median(x[0] for x in r), statistics.median(x[1] for x in r),
                                      statistics.median(x[2] for x in r), len(r), cfg, {x[3] for x in r}, {x[4] for x in r}))
for k in sorted(groups):
    print(f"## {k[0]} threads={k[1]} {k[2]}")
    for t, c, s, n, cfg, cs, bp in sorted(groups[k])[:top]:
        print(f"  time_s {t:.3f}  cpu {c:.3f}  sys {s:.3f}  n={n}  {cfg}  {'/'.join(cs)} bad={'/'.join(bp)}")
