# For user samples whose innermost earnest line matches PATTERN, counts the chain of the next earnest
# frames above it in the inlined chain (addr2line -i), and the perf symbol: who reached that line.
# Usage: python3 callers_inline.py X.script BINARY PATTERN [N]
import collections, re, sys
pat = sys.argv[3]
sys.argv = sys.argv[:3]
exec(open(sys.path[0] + "/attr.py").read().split("EARNEST = re.compile")[0])
E = re.compile(r"/crates/(earnest_[a-z_]+)/src/(.*?:\d+)")
c = collections.Counter(); tot = 0; hit = 0
for s in samples:
    if not s.get("chain"): continue
    tot += 1
    if not s["dso"].endswith("earnest-sym"): continue
    ch = frames.get(s["off"]) or []
    es = [(f, m.group(1) + "/" + m.group(2)) for f, l in ch for m in [E.search(l)] if m]
    if not es or not re.search(pat, es[0][1]): continue
    hit += 1
    up = " < ".join(loc for f, loc in es[1:4])
    c[(up, s["sym"][:60])] += 1
print(f"# {pat}: {hit} of {tot} samples ({100*hit/tot:.2f}%)")
for (up, sym), n in c.most_common(12): print(f"{100*n/tot:6.2f}%  {up}   [{sym}]")
