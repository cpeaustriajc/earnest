# For the DWARF call-chain recording: for samples whose leaf symbol matches, counts the first callers that are
# earnest functions (inlined frames included, as perf script prints them).
import collections, re, sys
path, pat = sys.argv[1], re.compile(sys.argv[2])
samples, cur = [], []
for line in open(path, errors="replace"):
    if not line.strip():
        if cur: samples.append(cur)
        cur = []
        continue
    m = re.match(r"\s*[0-9a-f]+\s+(.*)$", line)
    if m: cur.append(m.group(1))
if cur: samples.append(cur)
tot = len(samples); c = collections.Counter(); hit = 0
for s in samples:
    if not pat.search(s[0]): continue
    hit += 1
    callers = [re.sub(r"<[^<>]*>", "", re.sub(r"<[^<>]*>", "", f)).replace(" (inlined)", "") for f in s[1:]]
    callers = [f for f in callers if "earnest" in f][:3]
    c[" < ".join(x[-60:] for x in callers)] += 1
print(f"# {sys.argv[2]}: {hit} of {tot} samples ({100*hit/tot:.2f}%)")
for k, n in c.most_common(8): print(f"{100*n/tot:6.2f}%  {k}")
