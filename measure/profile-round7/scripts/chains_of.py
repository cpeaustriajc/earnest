# For the samples whose first earnest frame is FILE:LINE, counts the whole inlined chain of earnest frames.
import collections, re, sys
target = sys.argv.pop(3)
exec(open(sys.path[0] + "/attr.py").read().split("EARNEST = re.compile")[0])
E = re.compile(r"/crates/(earnest_[a-z_]+)/src/(.*?:\d+)")
c = collections.Counter(); tot = 0
for s in samples:
    if not s.get("chain"): continue
    tot += 1
    if not s["dso"].endswith("earnest-sym"): continue
    ch = frames.get(s["off"]) or []
    es = [(re.sub(r"<[^<>]*>", "", f)[-50:], m.group(2)) for f, l in ch for m in [E.search(l)] if m]
    if es and es[0][1] == target: c[" < ".join(f"{b}" for a, b in es) + "  [" + s["sym"][:60] + "]"] += 1
for k, n in c.most_common(6): print(f"{100*n/tot:6.2f}% {k}")
