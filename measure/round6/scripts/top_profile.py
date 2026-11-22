#!/usr/bin/env python3
"""From a task2 profile log: total samples, the N largest symbol lines, and the user/kernel split
(sum of the shares of every [.] line and every [k] line of the --percent-limit 0 report)."""
import re, sys
log, n = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 12
lines = open(log).read().split("\n")
sym = []
in_sym = False
for l in lines:
    if "--sort symbol" in l and l.startswith("$ perf report"): in_sym = True; continue
    if "--sort dso" in l and l.startswith("$ perf report"): in_sym = False
    if in_sym and l.startswith("# Samples"): print(l)
    if in_sym and (m := re.match(r"\s+([\d.]+)%\s+\[(.)\]\s+(\S+)", l)): sym.append((float(m[1]), m[2], m[3]))
user = sum(p for p, k, _ in sym if k == "."); kern = sum(p for p, k, _ in sym if k == "k")
other = sum(p for p, k, _ in sym if k not in ".k")
print(f"user [.] {user:.2f}%  kernel [k] {kern:.2f}%  other {other:.2f}%  lines {len(sym)}")
for p, k, s in sym[:n]: print(f"{p:6.2f}%  [{k}]  {s}")
