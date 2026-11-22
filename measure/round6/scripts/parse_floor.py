#!/usr/bin/env python3
"""Parses measure/round6/raw/task3_floor.log into a CSV and prints a summary and every run."""
import csv, re, statistics, sys
log = sys.argv[1]
runs, cur = [], None
for line in open(log):
    line = line.rstrip("\n")
    if (m := re.match(r"### run binary=(\S+) args='([^']+)' repeat=(\d+)", line)):
        cur = {"binary": m[1], "args": m[2], "repeat": int(m[3]), "states": "", "checksum": "", "parent_checksum": "", "time_s": "", "exit": ""}
    elif cur and (m := re.match(r"(states|checksum|parent_checksum|time_s)=(\S+)", line)): cur[m[1]] = m[2]
    elif cur and (m := re.match(r"### end exit=(\d+)", line)):
        cur["exit"] = int(m[1]); runs.append(cur); cur = None
with open(log[:-4] + ".csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=list(runs[0])); w.writeheader(); w.writerows(runs)
print("| Variant | Arguments | States | Checksums | Runs | Smallest | Median | Largest | Every counted run (s) |")
print("|---|---|---:|---|---:|---:|---:|---:|---|")
for args in ["7 serial", "8 serial"]:
    for b in ["floor_i", "floor_ii", "floor_iii"]:
        rs = [r for r in runs if r["args"] == args and r["binary"] == b and r["repeat"] >= 1]
        if not rs: continue
        t = [float(r["time_s"]) for r in rs]
        st = ",".join(sorted({r["states"] for r in rs})); ck = ",".join(sorted({r["checksum"] + "/" + r["parent_checksum"] for r in rs}))
        bad = sum(r["exit"] != 0 for r in rs)
        print(f"| {b} | `{args}` | {st} | {ck} | {len(rs)}{' BAD EXIT '+str(bad) if bad else ''} | {min(t):.4f} | {statistics.median(t):.4f} | {max(t):.4f} | {', '.join(f'{x:.4f}' for x in t)} |")
