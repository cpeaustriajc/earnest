# Summarises floor logs (floor_block.sh or floor_rounds.sh): per label (or binary) and wheels, the median and
# smallest of time_s, wall, CPU (user+sys of GNU time), sys, peak MiB, and the search's own CPU (run_cpu_s);
# the states, levels and checksums seen.  Usage: python3 floor_summary.py LOG...
import collections, re, statistics, sys
R = collections.defaultdict(list)
for path in sys.argv[1:]:
    cur = None
    for line in open(path):
        m = re.match(r"### run (?:label|binary)=(\S+) (?:wheels=(\d+) round=(\d+)|args='(\d+) \S+' repeat=(\d+))", line)
        if m:
            label, w, r = m.group(1), m.group(2) or m.group(4), int(m.group(3) or m.group(5))
            cur = {"label": label, "w": w, "r": r}; continue
        if cur is None: continue
        if line.startswith("gnutime"):
            cur.update({k: float(v) for k, v in (x.split("=") for x in line.split()[1:])})
        elif line.startswith("### end"):
            cur["exit"] = line.strip().split("=")[1]
            if cur["r"] > 0: R[(cur["label"], cur["w"])].append(cur)
            cur = None
        elif "=" in line and " " not in line.split("=")[0]:
            k, v = line.strip().split("=", 1); cur[k] = v
def md(v): return f"{statistics.median(v):.3f} ({min(v):.3f})"
print("| Floor | Wheels | n | time_s median (smallest) | all time_s | wall | CPU | System | Search CPU | Peak MiB | states/levels | checksums | exit |")
print("|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---|---|---|")
for (label, w), runs in sorted(R.items(), key=lambda x: (x[0][1], x[0][0])):
    ts = [float(x["time_s"]) for x in runs]
    cpu = [x["user"] + x["sys"] for x in runs]
    rc = [float(x["run_cpu_s"]) for x in runs if "run_cpu_s" in x]
    extra = md(rc) if rc else "-"
    sums = {x.get("set_checksum", x.get("checksum", "?")) for x in runs}
    print(f"| {label} | {w} | {len(runs)} | {md(ts)} | {', '.join(f'{t:.3f}' for t in ts)} | {md([x['wall'] for x in runs])} | {md(cpu)} | "
          f"{min(x['sys'] for x in runs):.2f}–{max(x['sys'] for x in runs):.2f} | {extra} | {statistics.median(x['maxrss_kb'] for x in runs)/1024:.0f} | "
          f"{'/'.join(sorted({x['states']+'/'+x['levels'] for x in runs}))} | {' '.join(sorted(sums))} | {''.join(sorted({x['exit'] for x in runs}))} |")
    if "gen_cpu_s" in runs[0]:
        print(f"|   (generating alone: gen_cpu_s) | {w} | | {md([float(x['gen_cpu_s']) for x in runs])} | | | | | | | successors {runs[0].get('successors')} | hash {runs[0].get('hash_sum')} | |")
