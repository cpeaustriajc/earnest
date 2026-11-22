# Sums user samples by the earnest function whose source lines hold the first earnest frame
# (function = the addr2line name of that frame), for the functions named in the old and new write-ups.
import collections, re, sys
exec(open(sys.path[0] + "/attr.py").read().split("EARNEST = re.compile")[0])
E = re.compile(r"/crates/(earnest_[a-z_]+)/src/(.*?):(\d+)")
c = collections.Counter(); tot = 0
for s in samples:
    if not s.get("chain"): continue
    tot += 1
    if not s["dso"].endswith("earnest-sym"): continue
    ch = frames.get(s["off"]) or []
    hit = next(((f, m) for f, l in ch for m in [E.search(l)] if m), None)
    if hit:
        f, m = hit
        name = re.sub(r"<[^<>]*>", "", re.sub(r"<[^<>]*>", "", f))
        c[m.group(2) + " " + name.split("::")[-1]] += 1
    else:
        c["(no earnest line) " + s["sym"][:70]] += 1
for k, n in c.most_common(int(sys.argv[3])): print(f"{100*n/tot:6.2f}% {n:6d}  {k}")
