# Sums user samples by the first earnest source line of the inlined chain (all innermost library lines under it),
# and lists the perf symbols of the samples that have no earnest frame at all.
import collections, re, sys
exec(open(sys.path[0] + "/attr.py").read().split("EARNEST = re.compile")[0])
E = re.compile(r"/crates/(earnest_[a-z_]+)/src/(.*?:\d+)")
by = collections.Counter(); none = collections.Counter(); tot = 0
for s in samples:
    if not s.get("chain"): continue
    tot += 1
    if not s["dso"].endswith("earnest-sym"): continue
    ch = frames.get(s["off"]) or []
    e = next((m.group(1) + "/" + m.group(2) for f, l in ch for m in [E.search(l)] if m), None)
    if e: by[e] += 1
    else: none[(s["sym"][:110], ch[0][1].split("/src/")[-1] if ch else "??")] += 1
print(f"# total samples {tot}")
for e, n in by.most_common(int(sys.argv[3])): print(f"{100*n/tot:6.2f}% {n:6d}  {e}")
print("# samples with no earnest frame, by perf symbol and innermost line")
for (sym, inner), n in none.most_common(8): print(f"{100*n/tot:6.2f}% {n:6d}  {sym}  [{inner}]")
