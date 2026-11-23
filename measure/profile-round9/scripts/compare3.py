# The tables of RESULTS-0ee0be2.md (compare2.py for three builds), from $S/raw/0ee0be2: times of both builds, the floors, the ratios before and
# after from one block, the groups, and the pieces above 2% of grid_100m with the seconds of both builds.
# Usage: python3 compare2.py [S]
import collections, json, re, statistics, sys

S = sys.argv[1] if len(sys.argv) > 1 else "/SCRATCH"
R = S + "/raw/0ee0be2"
sys.path.insert(0, S + "/scripts")


def summaries(path):
    out = {}
    for line in open(path):
        if line.startswith("{"):
            d = json.loads(line)
            if d.get("event") == "summary":
                out[(d["build"], d["model"], d["jobs"])] = d
    return out


C = {}
for f in ("t1_7rounds.log", "t2_100m_3rounds.log"):
    C.update(summaries(f"{R}/{f}"))

# floors of the short block
runs = collections.defaultdict(list)
cur = None
for line in open(f"{R}/f_floors.log"):
    m = re.match(r"### run label=(\S+) wheels=(\d+) round=(\d+)", line)
    if m:
        cur = {"label": m.group(1), "w": m.group(2), "r": int(m.group(3))}
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
F = {}
for k, rs in runs.items():
    med = lambda f: statistics.median(f(x) for x in rs)
    F[k] = dict(time=med(lambda x: float(x["time_s"])), time_min=min(float(x["time_s"]) for x in rs),
                cpu=med(lambda x: float(x["run_cpu_s"])), sys=med(lambda x: float(x["run_sys_s"])),
                mib=med(lambda x: x["maxrss_kb"]) / 1024,
                gen=med(lambda x: float(x["gen_cpu_s"])) if "gen_cpu_s" in rs[0] else None,
                all=[float(x["time_s"]) for x in rs], sums={x.get("set_checksum") for x in rs},
                states={x["states"] + "/" + x["levels"] for x in rs})

print("## Floors on this machine (5 rounds; time_s median (smallest); CPU and sys of the search)\n")
print("| Floor | Wheels | time_s | all | CPU | sys | peak MiB | generating CPU | states/levels | set checksum |")
print("|---|---:|---:|---|---:|---:|---:|---:|---|---|")
for (label, w), v in sorted(F.items(), key=lambda x: (x[0][1], x[1]["time"])):
    g = f"{v['gen']:.3f}" if v["gen"] is not None else ""
    print(f"| {label} | {w} | {v['time']:.3f} ({v['time_min']:.3f}) | {', '.join(f'{t:.3f}' for t in v['all'])} | {v['cpu']:.3f} | "
          f"{v['sys']:.2f} | {v['mib']:.0f} | {g} | {'/'.join(v['states'])} | {' '.join(sorted(map(str, v['sums'])))} |")

print("\n## The checker, both builds, in turn (median (smallest))\n")
print("| Model | Workers | Build | Wall | CPU | System | Minor faults | Peak MiB | n | statuses |")
print("|---|---:|---|---:|---:|---:|---:|---:|---:|---|")
for model in ("grid_10m", "grid_100m", "wide", "lights", "ledger"):
    for j in (1, 2, 4):
        for b in ("a3058", "b89de", "c0ee0", "floor"):
            d = C.get((b, model, j))
            if not d:
                continue
            name = {"a3058": "3058cf4", "b89de": "89de199", "c0ee0": "0ee0be2", "floor": "floor (in turn)"}[b]
            extra = f" time_s {d['time_s'][0]:.3f} ({d['time_s'][1]:.3f})" if d.get("time_s") else ""
            print(f"| {model} | {j} | {name} | {d['wall'][0]:.3f} ({d['wall'][1]:.3f}) | {d['cpu'][0]:.3f} ({d['cpu'][1]:.3f}) | "
                  f"{d['sys'][0]:.3f} | {d['minflt'][0]:,.0f} | {d['mib'][0]:.1f} | {d['n']} | "
                  f"{[s[:3] for s in d['statuses']]}{extra} |")

# per-round ratios against the floor in the same round
def rounds(path):
    out = collections.defaultdict(dict)
    for line in open(path):
        if line.startswith("{") and '"run"' in line:
            d = json.loads(line)
            out[(d["model"], d["jobs"], d["round"])][d["build"]] = d
    return out


RR = {}
for f in ("t1_7rounds.log", "t2_100m_3rounds.log"):
    RR.update(rounds(f"{R}/{f}"))
print("\n## Whole checker against the floor of the same round (wall / floor time_s), median of rounds, and against the floor block\n")
print("| Model | Workers | 3058cf4 in turn | 89de199 in turn | 0ee0be2 in turn | per round 0ee0be2 | 3058cf4 block | 89de199 block | 0ee0be2 block | 0ee0be2 / parts floor |")
print("|---|---:|---:|---:|---:|---|---:|---:|---:|---:|")
for model, w in (("grid_10m", "7"), ("grid_100m", "8")):
    for j in (1, 2, 4):
        ks = sorted(k for k in RR if k[0] == model and k[1] == j)
        if not ks:
            continue
        r = {b: [RR[k][b]["wall"] / RR[k]["floor"]["time_s"] for k in ks] for b in ("a3058", "b89de", "c0ee0")}
        blk = F.get(("serial_huge" if j == 1 else "shared4_huge", w)) if j != 2 else None
        parts = F.get(("parts4_huge", w)) if j == 4 else (F.get(("serial_huge", w)) if j == 1 else None)
        cell = lambda b: f"{C[(b, model, j)]['wall'][0] / blk['time']:.2f}" if blk else ""
        pc = f"{C[('c0ee0', model, j)]['wall'][0] / parts['time']:.2f}" if parts else ""
        print(f"| {model} | {j} | {statistics.median(r['a3058']):.2f} | {statistics.median(r['b89de']):.2f} | {statistics.median(r['c0ee0']):.2f} | "
              f"{', '.join(f'{x:.2f}' for x in r['c0ee0'])} | {cell('a3058')} | {cell('b89de')} | {cell('c0ee0')} | {pc} |")

# groups and pieces
cpu = {}
for b in ("b89de", "c0ee0"):
    for t, (m, j) in {"10m_j1": ("grid_10m", 1), "10m_j4": ("grid_10m", 4), "100m_j1": ("grid_100m", 1),
                      "100m_j4": ("grid_100m", 4), "wide_j1": ("wide", 1)}.items():
        cpu[(b, t)] = C[(b, m, j)]["cpu"][0]
G = {(b, t): json.load(open(f"{R}/prof_{b}_{t}_groups.json")) for (b, t) in cpu}
names = {"a": "(a) index of known states", "b": "(b) packed rows", "c": "(c) part tables, claim and merge",
         "d": "(d) actions and rules", "e": "(e) hashing", "f": "(f) allocation and page faults",
         "g": "(g) threads, waiting, other"}
T = ("10m_j1", "10m_j4", "100m_j1", "100m_j4", "wide_j1")
share = lambda b, t, gs: sum(G[(b, t)]["groups"].get(g, 0) for g in gs) / G[(b, t)]["total"]
print("\n## Groups, CPU seconds (share x median CPU of the same block), 89de199 -> 0ee0be2\n")
print("| Group | " + " | ".join(T) + " |")
print("|---|" + "---:|" * len(T))
for g in "abcdefg":
    print(f"| {names[g]} | " + " | ".join(f"{share('b89de', t, g)*cpu[('b89de', t)]:.2f} → {share('c0ee0', t, g)*cpu[('c0ee0', t)]:.2f}" for t in T) + " |")
print("| total | " + " | ".join(f"{cpu[('b89de', t)]:.2f} → {cpu[('c0ee0', t)]:.2f}" for t in T) + " |")
print("| samples | " + " | ".join(f"{G[('b89de', t)]['total']} / {G[('c0ee0', t)]['total']}" for t in T) + " |")

units = {"10m": {"states": 1e7, "succ": 63e6, "cand": 26.0e6}, "100m": {"states": 1e8, "succ": 720e6, "cand": 259.5e6}}
pieces = [
    ("a part's table (hashbrown at 3058cf4, RecentWords at 89de199)", "c",
     ["part table: hashbrown in explore/part.rs", "part table: hashbrown outlined ()", "part table: RecentWords in explore/part.rs",
      "parts/claim (by name)"], "succ"),
    ("index probe in `claim_level` (hashbrown in level.rs)", "a", ["index: hashbrown in store/level.rs"], "cand"),
    ("key load from the bucket, `Key::holds` (`store/index.rs`)", "a", ["index: store/index.rs"], "cand"),
    ("index rehash, outlined `reserve_rehash`", "a", ["index: hashbrown outlined ({closure#1}>)"], "states"),
    ("frontier read, `store/rows.rs`", "b", ["rows: store/rows.rs"], "states"),
    ("rows: codec, layout, `read_state`, `outside`", "b",
     ["rows: store/codec.rs", "rows: store/layout.rs", "rows: store.rs read_state", "rows: store.rs outside", "rows (by name)"], "states"),
    ("`explore/part.rs` lines of its own", "c", ["parts/claim: explore/part.rs"], "states"),
    ("`store/level.rs` lines of its own", "c", ["parts/claim: store/level.rs"], "states"),
    ("`expand.rs`", "d", ["eval: expand.rs", "eval (by name)"], "succ"),
    ("`earnest_ir`", "d", None, "succ"),
    ("hashing, `mix`/`hash_words`", "e", ["hash: store/layout.rs mix/hash_words"], "succ"),
    ("kernel page faults", "f", ["kernel: page faults"], "states"),
    ("kernel mmap/munmap/madvise", "f", ["kernel: mmap/munmap/madvise"], "states"),
    ("libc malloc/free/realloc", "f", ["alloc: libc malloc/realloc/free", "alloc: libc memmove inside realloc"], "states"),
    ("libc memmove/memset, caller not resolved", "g", ["libc memmove/memset (caller not resolved)"], "states"),
    ("`explore/workers.rs` and the rest of (g)", "g", "REST_G", "states"),
]


def sh(b, t, subs, g):
    d = G[(b, t)]
    n = 0
    named = {s for p in pieces if isinstance(p[2], list) for s in p[2]}
    for gg, s, c in d["subgroups"]:
        if subs is None:
            n += c if (gg == g and s.startswith("eval: earnest_ir")) else 0
        elif subs == "REST_G":
            n += c if (gg == "g" and s not in named) else 0
        elif s in subs:
            n += c
    return n / d["total"]


for size in ("100m", "10m"):
    print(f"\n## Pieces of grid_{size}: CPU seconds 89de199 -> 0ee0be2 at 1 and 4 workers, and ns per unit at 0ee0be2\n")
    print("| Piece | Group | 1: 89de199 | 0ee0be2 | change | 4: 89de199 | 0ee0be2 | change | 0ee0be2 share 1 / 4 | ns/unit 0ee0be2: 1 / 4 |")
    print("|---|---|---:|---:|---:|---:|---:|---:|---|---|")
    for name, g, subs, u in pieces:
        v = {(b, j): sh(b, f"{size}_j{j}", subs, g) * cpu[(b, f"{size}_j{j}")] for b in ("b89de", "c0ee0") for j in (1, 4)}
        ns = [v[("c0ee0", j)] / units[size][u] * 1e9 for j in (1, 4)]
        print(f"| {name} | {g} | {v[('b89de',1)]:.2f} | {v[('c0ee0',1)]:.2f} | {v[('c0ee0',1)]-v[('b89de',1)]:+.2f} | "
              f"{v[('b89de',4)]:.2f} | {v[('c0ee0',4)]:.2f} | {v[('c0ee0',4)]-v[('b89de',4)]:+.2f} | "
              f"{100*sh('c0ee0',f'{size}_j1',subs,g):.1f}% / {100*sh('c0ee0',f'{size}_j4',subs,g):.1f}% | {ns[0]:.1f} / {ns[1]:.1f} per {u} |")

# the claim (group a) per row reaching it, 1 against 4 workers
print("\n## The claim, group (a), ns per row reaching it (rows: estimated candidates x 1.16), and its parts\n")
print("| Build | Size | (a) 1 worker | (a) 4 workers | 4 / 1 | probe level.rs:131 1 / 4 | key load index.rs 1 / 4 |")
print("|---|---|---:|---:|---:|---|---|")
for b in ("b89de", "c0ee0"):
    for size in ("10m", "100m"):
        rows = units[size]["cand"] * 1.16
        a1, a4 = share(b, f"{size}_j1", "a") * cpu[(b, f"{size}_j1")] / rows * 1e9, share(b, f"{size}_j4", "a") * cpu[(b, f"{size}_j4")] / rows * 1e9
        p = [sh(b, f"{size}_j{j}", ["index: hashbrown in store/level.rs"], "a") * cpu[(b, f"{size}_j{j}")] / rows * 1e9 for j in (1, 4)]
        k = [sh(b, f"{size}_j{j}", ["index: store/index.rs"], "a") * cpu[(b, f"{size}_j{j}")] / rows * 1e9 for j in (1, 4)]
        print(f"| {b} | {size} | {a1:.1f} | {a4:.1f} | {a4/a1:.2f} | {p[0]:.1f} / {p[1]:.1f} | {k[0]:.1f} / {k[1]:.1f} |")

print("\n## Store and evaluation against the floor of this machine\n")
print("| Run | Build | store a+b+c+e | with (f) | floor | floor CPU | gen CPU | floor less gen | store / it | with (f) / it | (d) | (d) / gen |")
print("|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|")
for t in ("10m_j1", "10m_j4", "100m_j1", "100m_j4"):
    size, j = t.split("_j")
    w = {"10m": "7", "100m": "8"}[size]
    for fl in (("serial_huge",) if j == "1" else ("shared4_huge", "parts4_huge")):
        fc, gen = F[(fl, w)]["cpu"], F[(f"gen{j}", w)]["gen"]
        for b in ("b89de", "c0ee0"):
            c = cpu[(b, t)]
            st, stf, d = share(b, t, "abce") * c, share(b, t, "abcef") * c, share(b, t, "d") * c
            print(f"| {t} | {'89de199' if b == 'b89de' else '0ee0be2'} | {st:.2f} | {stf:.2f} | {fl} | {fc:.2f} | {gen:.3f} | {fc-gen:.2f} | "
                  f"{st/(fc-gen):.2f} | {stf/(fc-gen):.2f} | {d:.2f} | {d/gen:.1f} |")
