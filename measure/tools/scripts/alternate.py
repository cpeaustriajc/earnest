import os, re, statistics, subprocess, sys

N = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
S = "/SCRATCH_OLDER"
OLD = os.environ.get("OLD", os.path.join(N, "rows", "earnest-fix5"))
NEW = sys.argv[1]
ROUNDS = int(sys.argv[2]) if len(sys.argv) > 2 else 7
CASES = [
    ("grid_10m", os.path.join(S, "models-new", "grid_10m.model.ts"), 4),
    ("grid_10m", os.path.join(S, "models-new", "grid_10m.model.ts"), 10),
    ("grid_1m", os.path.join(S, "models-new", "grid_1m.model.ts"), 4),
    ("wide", os.path.join(S, "models-new", "wide.model.ts"), 4),
    ("ledger", os.path.join(S, "models-new", "ledger.model.ts"), 4),
    ("corridor", os.path.join(N, "loop", "probe", "corridor.model.ts"), 4),
    ("corridor", os.path.join(N, "loop", "probe", "corridor.model.ts"), 10),
]


def run(binary, path, jobs):
    p = subprocess.run(["/usr/bin/time", "-l", binary, "verify", "--no-cache", "--json", "--jobs", str(jobs), "--max-states", "20000000", path],
                       capture_output=True, text=True, cwd=os.path.dirname(path))
    m = re.search(r"([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys", p.stderr)
    return float(m.group(1)), float(m.group(2)) + float(m.group(3))


print(os.popen("uptime").read().strip())
for name, path, jobs in CASES:
    run(OLD, path, jobs)
    times = {"old": [], "new": []}
    for _ in range(ROUNDS):
        times["old"].append(run(OLD, path, jobs))
        times["new"].append(run(NEW, path, jobs))
    line = f"{name} --jobs {jobs}, {ROUNDS} rounds in turn:"
    for which in ("old", "new"):
        real = [r for r, _ in times[which]]
        cpu = [c for _, c in times[which]]
        line += f" {which} real {min(real):.2f}/{statistics.median(real):.2f}/{max(real):.2f} s, CPU median {statistics.median(cpu):.2f} s;"
    print(line, flush=True)
print(os.popen("uptime").read().strip())
