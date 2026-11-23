# Counts user samples by the innermost earnest source line (and by the innermost line of any crate).
import collections, re, sys
sys.argv = [sys.argv[0]] + sys.argv[1:]
exec(open(sys.path[0] + "/attr.py").read().split("EARNEST = re.compile")[0])
E = re.compile(r"/crates/(earnest_[a-z_]+)/src/(.*?:\d+)")
c1 = collections.Counter(); c2 = collections.Counter(); tot = 0
for s in samples:
    if not s.get("chain"): continue
    tot += 1
    if not s["dso"].endswith("earnest-sym"): continue
    ch = frames.get(s["off"]) or []
    inner = ch[0][1] if ch else "??"
    inner = re.sub(r".*/(src/.*)", r"\1", inner).split(" ")[0]
    e = next((m.group(1) + "/" + m.group(2) for f, l in ch for m in [E.search(l)] if m), "-")
    c1[(e, inner)] += 1
for (e, inner), n in c1.most_common(int(sys.argv[3]) if len(sys.argv) > 3 else 30):
    print(f"{100*n/tot:6.2f}% {n:6d}  {e:45s} innermost {inner}")
