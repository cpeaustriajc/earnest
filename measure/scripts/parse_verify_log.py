#!/usr/bin/env python3
"""Turns a log of run_verify_block.sh into a CSV of runs and a table of smallest and median values.

usage: parse_verify_log.py <log> [expected_status expected_states]
Writes <log minus .log>.csv next to the log and prints the summary as Markdown.
Wall time is the "Elapsed (wall clock) time" of /usr/bin/time -v (resolution 0.01 s).
"""
import csv, re, statistics, sys

log = sys.argv[1]
want_status = sys.argv[2] if len(sys.argv) > 2 else None
want_states = int(sys.argv[3]) if len(sys.argv) > 3 else None

def elapsed_seconds(text):
    parts = [float(p) for p in text.split(":")]
    seconds = 0.0
    for p in parts:
        seconds = seconds * 60 + p
    return seconds

runs, cur, head = [], None, []
for line in open(log):
    line = line.rstrip("\n")
    if line.startswith("# "):
        head.append(line)
    m = re.match(r"### run round=(-?\d+) build=(\w) jobs=(\d+)", line)
    if m:
        cur = {"round": int(m[1]), "build": m[2], "jobs": int(m[3]), "status": "", "states": "",
               "json_ms": "", "wall_s": "", "max_rss_kb": "", "exit": "", "outer_wall_ms": ""}
        continue
    if cur is None:
        continue
    if (m := re.match(r'\s*"status": "(\w+)"', line)): cur["status"] = m[1]
    elif (m := re.match(r'\s*"states": (\d+)', line)): cur["states"] = int(m[1])
    elif (m := re.match(r'\s*"milliseconds": (\d+)', line)): cur["json_ms"] = int(m[1])
    elif (m := re.match(r"\s*Elapsed \(wall clock\) time.*: ([\d:.]+)$", line)): cur["wall_s"] = elapsed_seconds(m[1])
    elif (m := re.match(r"\s*Maximum resident set size \(kbytes\): (\d+)", line)): cur["max_rss_kb"] = int(m[1])
    elif (m := re.match(r"### end exit=(-?\d+) outer_wall_ns=(\d+)", line)):
        cur["exit"] = int(m[1]); cur["outer_wall_ms"] = round(int(m[2]) / 1e6, 1)
        runs.append(cur); cur = None

out = log[:-4] + ".csv"
fields = ["round", "build", "jobs", "status", "states", "json_ms", "wall_s", "max_rss_kb", "exit", "outer_wall_ms"]
with open(out, "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=fields); w.writeheader(); w.writerows(runs)

for h in head: print(h)
bad = [r for r in runs if (want_status and r["status"] != want_status) or (want_states is not None and r["states"] != want_states)]
print(f"runs={len(runs)} measured={sum(r['round'] >= 1 for r in runs)} not_as_expected={len(bad)}")
for r in bad: print("NOT AS EXPECTED:", r)
print()
print("| Build | Workers | Rounds | Status | States | Smallest wall time | Median wall time | Largest wall time | Smallest peak memory | Median peak memory |")
print("|---|---:|---:|---|---:|---:|---:|---:|---:|---:|")
combos = []
for r in runs:
    k = (r["build"], r["jobs"])
    if k not in combos: combos.append(k)
for b, j in sorted(combos, key=lambda k: (k[1], k[0])):
    rs = [r for r in runs if r["build"] == b and r["jobs"] == j and r["round"] >= 1]
    walls = [r["wall_s"] for r in rs]; rss = [r["max_rss_kb"] / 1024 for r in rs]
    statuses = ",".join(sorted({str(r["status"]) for r in rs})); states = ",".join(sorted({str(r["states"]) for r in rs}))
    print(f"| {b} | {j} | {len(rs)} | {statuses} | {states} | {min(walls):.2f} s | {statistics.median(walls):.2f} s | {max(walls):.2f} s | {min(rss):.0f} MiB | {statistics.median(rss):.0f} MiB |")
