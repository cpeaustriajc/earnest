# The comparison of step 5: the checker's groups (profile shares x median CPU of the unprofiled runs) against
# the floors. Reads the timing logs, the floor logs and the groups of the profiles from $S/raw.
# Usage: python3 compare.py [S]
import collections, json, re, statistics, sys

S = sys.argv[1] if len(sys.argv) > 1 else "/SCRATCH"
R = S + "/raw"
names = {"a": "(a) index of known states", "b": "(b) packed rows", "c": "(c) part tables, claim and merge",
         "d": "(d) actions and rules", "e": "(e) hashing", "f": "(f) allocation and page faults",
         "g": "(g) threads, waiting, other"}


def summaries(path):
    out = {}
    for line in open(path):
        if line.startswith("{"):
            d = json.loads(line)
            if d.get("event") == "summary":
                out[(d["build"], d["model"], d["jobs"])] = d
    return out


C = {}
for f in ("c1_10m_wide_lights.log", "c2_100m.log"):
    try:
        C.update(summaries(f"{R}/{f}"))
    except FileNotFoundError:
        pass


def floors(paths):
    runs = collections.defaultdict(list)
    for path in paths:
        cur = None
        for line in open(path):
            m = re.match(r"### run (?:label|binary)=(\S+) (?:wheels=(\d+) round=(\d+)|args='(\d+) \S+' repeat=(\d+))", line)
            if m:
                cur = {"label": m.group(1), "w": m.group(2) or m.group(4), "r": int(m.group(3) or m.group(5))}
                continue
            if cur is None:
                continue
            if line.startswith("gnutime"):
                cur.update({k: float(v) for k, v in (x.split("=") for x in line.split()[1:])})
            elif line.startswith("### end"):
                if cur["r"] > 0:
                    runs[(cur["label"], cur["w"])].append(cur)
                cur = None
            elif "=" in line and " " not in line.split("=")[0]:
                k, v = line.strip().split("=", 1)
                cur[k] = v
    out = {}
    for k, rs in runs.items():
        med = lambda f: statistics.median(f(x) for x in rs)
        low = lambda f: min(f(x) for x in rs)
        cpu = (lambda x: float(x["run_cpu_s"])) if "run_cpu_s" in rs[0] else (lambda x: x["user"] + x["sys"])
        out[k] = dict(time=med(lambda x: float(x["time_s"])), time_min=low(lambda x: float(x["time_s"])),
                      cpu=med(cpu), cpu_min=low(cpu), sys=med(lambda x: float(x.get("run_sys_s", x["sys"]))),
                      gen_cpu=med(lambda x: float(x["gen_cpu_s"])) if "gen_cpu_s" in rs[0] else None,
                      n=len(rs))
    return out


F = floors([f"{R}/f1_floor_serial.log", f"{R}/f2_floor_par_gen.log"])
G = {}
for t in ("10m_j1", "10m_j4", "100m_j1", "100m_j4", "wide_j1"):
    try:
        G[t] = json.load(open(f"{R}/prof_{t}_groups.json"))
    except FileNotFoundError:
        pass

model = {"10m": "grid_10m", "100m": "grid_100m", "wide": "wide"}
wheels = {"10m": "7", "100m": "8"}


def cpu_of(t):
    size, j = t.split("_j")
    return C[("checker", model[size], int(j))]["cpu"][0]


def share(t, gs):
    d = G[t]
    return sum(d["groups"].get(g, 0) for g in gs) / d["total"]


print("## Groups (share of samples, and share x median CPU of the unprofiled runs)\n")
print("| Group | " + " | ".join(f"{t} share | {t} CPU s" for t in G) + " |")
print("|---|" + "---:|---:|" * len(G))
for g in "abcdefg":
    print(f"| {names[g]} | " + " | ".join(f"{100*share(t, g):.1f}% | {share(t, g)*cpu_of(t):.2f}" for t in G) + " |")
print("| total (median CPU) | " + " | ".join(f"100% | {cpu_of(t):.2f}" for t in G) + " |")
print("| samples | " + " | ".join(f"{G[t]['total']} | " for t in G) + " |")

print("\n## Floors (medians of 5; CPU is the search's own, from getrusage, for floor_par; GNU time for floor_i/ii/iii)\n")
print("| Floor | Wheels | time_s | smallest | CPU | smallest | sys | generating alone, CPU |")
print("|---|---:|---:|---:|---:|---:|---:|---:|")
for (label, w), v in sorted(F.items(), key=lambda x: (x[0][1], x[1]["time"])):
    print(f"| {label} | {w} | {v['time']:.3f} | {v['time_min']:.3f} | {v['cpu']:.3f} | {v['cpu_min']:.3f} | {v['sys']:.2f} | "
          f"{v['gen_cpu']:.3f} |" if v["gen_cpu"] is not None else
          f"| {label} | {w} | {v['time']:.3f} | {v['time_min']:.3f} | {v['cpu']:.3f} | {v['cpu_min']:.3f} | {v['sys']:.2f} | |")


def best(w, threads, pages=None):
    pre = {1: ("floor_i", "floor_ii", "floor_iii", "serial_"), 2: ("parts2", "shared2"), 4: ("parts4", "shared4")}[threads]
    c = [(v["time"], k[0], v) for k, v in F.items() if k[1] == w and k[0].startswith(pre)
         and (pages is None or k[0].endswith(pages) or (pages == "4k" and k[0].startswith("floor_")))]
    return min(c, key=lambda x: x[0]) if c else None


print("\n## Step 5: the checker against the best floor\n")
print("| Size | Threads | Floor (best by time_s) | Floor time_s | Floor CPU | Checker wall | Checker CPU | wall / floor | CPU / floor CPU | "
      "Floor, 4 KiB pages | its time_s | wall / it |")
print("|---|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---:|")
for size in ("10m", "100m"):
    for j in (1, 2, 4):
        k = ("checker", model[size], j)
        if k not in C:
            continue
        b, b4 = best(wheels[size], j), best(wheels[size], j, "4k")
        if not b:
            continue
        cw, cc = C[k]["wall"][0], C[k]["cpu"][0]
        print(f"| {size} | {j} | {b[1]} | {b[0]:.3f} | {b[2]['cpu']:.3f} | {cw:.3f} | {cc:.3f} | {cw/b[0]:.2f} | {cc/b[2]['cpu']:.2f} | "
              f"{b4[1]} | {b4[0]:.3f} | {cw/b4[0]:.2f} |")

print("\n## Step 5: store work against the floor less its generating; evaluation against the generating\n")
print("| Run | store (a+b+c+e) CPU s | with (f) | floor | floor CPU | gen CPU | floor less gen | store / it | with (f) / it | "
      "(d) CPU s | (d) / gen | (g) CPU s |")
print("|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|")
for t in ("10m_j1", "10m_j4", "100m_j1", "100m_j4"):
    if t not in G:
        continue
    size, j = t.split("_j")
    j = int(j)
    b = best(wheels[size], j)
    gen = F[(f"gen{j}", wheels[size])]["gen_cpu"]
    cpu = cpu_of(t)
    st, stf, d, g = share(t, "abce") * cpu, share(t, "abcef") * cpu, share(t, "d") * cpu, share(t, "g") * cpu
    rest = b[2]["cpu"] - gen
    print(f"| {t} | {st:.2f} | {stf:.2f} | {b[1]} | {b[2]['cpu']:.2f} | {gen:.3f} | {rest:.2f} | {st/rest:.2f} | {stf/rest:.2f} | "
          f"{d:.2f} | {d/gen:.1f} | {g:.2f} |")
