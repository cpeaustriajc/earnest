# The pieces of step 5: each subgroup's share, seconds and cost per unit (state, successor or candidate) at
# 10M and 100M, 1 and 4 workers. CPU from the timing logs of checker.sh; successors from the floor; candidates
# (distinct successors per part, 64 parts a level) from `floor_par 8 cands 64` (raw/cands.txt).
# Usage: python3 pieces.py [S]
import json, sys
S = sys.argv[1] if len(sys.argv) > 1 else "/SCRATCH"
cpu = {}
for f in ("c1_10m_wide_lights.log", "c2_100m.log"):
    for l in open(f"{S}/raw/{f}"):
        if '"summary"' in l:
            d = json.loads(l)
            if d["build"] == "checker": cpu[(d["model"], d["jobs"])] = d["cpu"][0]
cpu = {"10m_j1": cpu[("grid_10m", 1)], "10m_j4": cpu[("grid_10m", 4)], "100m_j1": cpu[("grid_100m", 1)], "100m_j4": cpu[("grid_100m", 4)]}
D = {t: json.load(open(f"{S}/raw/prof_{t}_groups.json")) for t in cpu}
units = {"10m": {"states": 1e7, "succ": 63e6, "cand": 26.0e6}, "100m": {"states": 1e8, "succ": 720e6, "cand": 259.5e6}}
pieces = [
    ("a part's table, `offer_unseen_word` (hashbrown in part.rs)", "c", ["part table: hashbrown in explore/part.rs", "part table: hashbrown outlined ()"], "succ"),
    ("index probe in `claim_level` (hashbrown in level.rs)", "a", ["index: hashbrown in store/level.rs"], "cand"),
    ("index key compare and shard, `store/index.rs`", "a", ["index: store/index.rs"], "cand"),
    ("index rehash, outlined `reserve_rehash`", "a", ["index: hashbrown outlined ({closure#1}>)"], "states"),
    ("frontier read, `store/rows.rs` (`word_of` in `unpack`)", "b", ["rows: store/rows.rs"], "states"),
    ("rows: codec, layout, `read_state`, `outside`", "b", ["rows: store/codec.rs", "rows: store/layout.rs", "rows: store.rs read_state", "rows: store.rs outside", "rows (by name)"], "states"),
    ("`explore/part.rs` lines of its own", "c", ["parts/claim: explore/part.rs", "parts/claim (by name)"], "states"),
    ("`store/level.rs` lines of its own", "c", ["parts/claim: store/level.rs"], "states"),
    ("`expand.rs`", "d", ["eval: expand.rs", "eval (by name)"], "succ"),
    ("`earnest_ir` (num, eval, cond, mod)", "d", None, "succ"),
    ("hashing, `mix`/`hash_words`", "e", ["hash: store/layout.rs mix/hash_words"], "succ"),
    ("kernel page faults", "f", ["kernel: page faults"], "states"),
    ("kernel mmap/munmap/madvise", "f", ["kernel: mmap/munmap/madvise"], "states"),
    ("libc malloc/free/realloc", "f", ["alloc: libc malloc/realloc/free", "alloc: libc memmove inside realloc"], "states"),
    ("libc memmove/memset, caller not resolved", "g", ["libc memmove/memset (caller not resolved)"], "states"),
]
def sh(t, subs, g):
    d = D[t]; n = 0
    for gg, s, c in d["subgroups"]:
        if (subs is None and gg == g and s.startswith("eval: earnest_ir")) or (subs and s in subs): n += c
    return n / d["total"]
print("| Piece | Group | Unit | 10M 1 share | 100M 1 share | 100M 1 s | 100M 4 share | 100M 4 s | ns/unit 10M 1 | 100M 1 | 10M 4 | 100M 4 |")
print("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
for name, g, subs, u in pieces:
    v = {t: sh(t, subs, g) for t in cpu}
    ns = {t: v[t] * cpu[t] / units[t.split('_')[0]][u] * 1e9 for t in cpu}
    print(f"| {name} | {g} | {u} | {100*v['10m_j1']:.2f}% | {100*v['100m_j1']:.2f}% | {v['100m_j1']*cpu['100m_j1']:.2f} | {100*v['100m_j4']:.2f}% | "
          f"{v['100m_j4']*cpu['100m_j4']:.2f} | {ns['10m_j1']:.1f} | {ns['100m_j1']:.1f} | {ns['10m_j4']:.1f} | {ns['100m_j4']:.1f} |")
