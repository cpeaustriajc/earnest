#!/usr/bin/env python3
"""Parses measure/raw/block7_floor.log into a CSV and prints the table of smallest and median time_s."""
import csv, re, statistics, sys

log = sys.argv[1]
runs, cur = [], None
for line in open(log):
    line = line.rstrip("\n")
    if (m := re.match(r"### run binary=(\S+) args='([^']+)' repeat=(\d+)", line)):
        cur = {"binary": m[1], "args": m[2], "repeat": int(m[3]), "states": "", "time_s": ""}
    elif cur and (m := re.match(r"states=(\d+)", line)): cur["states"] = int(m[1])
    elif cur and (m := re.match(r"time_s=([\d.]+)", line)): cur["time_s"] = float(m[1])
    elif cur and line.startswith("### end"):
        runs.append(cur); cur = None
with open(log[:-4] + ".csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=["binary", "args", "repeat", "states", "time_s"]); w.writeheader(); w.writerows(runs)
print("| Binary | Arguments | States | Runs | Smallest time_s | Median time_s | Largest time_s |")
print("|---|---|---:|---:|---:|---:|---:|")
keys = []
for r in runs:
    k = (r["args"], r["binary"])
    if k not in keys: keys.append(k)
names = {"floor": "floor (no prefetch on x86-64)", "floor_x86": "floor_x86 (with _mm_prefetch)"}
for args, b in keys:
    rs = [r for r in runs if (r["args"], r["binary"]) == (args, b)]
    t = [r["time_s"] for r in rs]
    st = ",".join(sorted({str(r["states"]) for r in rs}))
    print(f"| {names[b]} | `{args}` | {st} | {len(rs)} | {min(t):.4f} s | {statistics.median(t):.4f} s | {max(t):.4f} s |")
