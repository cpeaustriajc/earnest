#!/usr/bin/env python3
"""Turns a log of run_block.sh into a CSV of every run and a Markdown summary.

usage: parse_block.py <log> <expected_states>
Wall time: "Elapsed (wall clock) time" of /usr/bin/time -v (0.01 s). CPU time: user + system of /usr/bin/time -v.
"""
import csv, re, statistics, sys

log, want_states = sys.argv[1], int(sys.argv[2])

def secs(t):
    s = 0.0
    for p in t.split(":"):
        s = s * 60 + float(p)
    return s

runs, cur, head = [], None, []
for line in open(log):
    line = line.rstrip("\n")
    if line.startswith("# "): head.append(line)
    if (m := re.match(r"### run round=(-?\d+) variant=(\S+) build=(\S+) jobs=(\d+) cpus=(\S+)", line)):
        cur = dict(round=int(m[1]), variant=m[2], build=m[3], jobs=int(m[4]), cpus=m[5], status="", states="",
                   json_ms="", wall_s="", user_s="", sys_s="", cpu_s="", max_rss_kb="", vol_cs="", invol_cs="", exit="")
        continue
    if cur is None: continue
    if (m := re.match(r'\s*"status": "(\w+)"', line)): cur["status"] = m[1]
    elif (m := re.match(r'\s*"states": (\d+)', line)): cur["states"] = int(m[1])
    elif (m := re.match(r'\s*"milliseconds": (\d+)', line)): cur["json_ms"] = int(m[1])
    elif (m := re.match(r"\s*User time \(seconds\): ([\d.]+)", line)): cur["user_s"] = float(m[1])
    elif (m := re.match(r"\s*System time \(seconds\): ([\d.]+)", line)): cur["sys_s"] = float(m[1])
    elif (m := re.match(r"\s*Elapsed \(wall clock\) time.*: ([\d:.]+)$", line)): cur["wall_s"] = secs(m[1])
    elif (m := re.match(r"\s*Maximum resident set size \(kbytes\): (\d+)", line)): cur["max_rss_kb"] = int(m[1])
    elif (m := re.match(r"\s*Voluntary context switches: (\d+)", line)): cur["vol_cs"] = int(m[1])
    elif (m := re.match(r"\s*Involuntary context switches: (\d+)", line)): cur["invol_cs"] = int(m[1])
    elif (m := re.match(r"### end exit=(-?\d+)", line)):
        cur["exit"] = int(m[1]); cur["cpu_s"] = round(cur["user_s"] + cur["sys_s"], 2)
        runs.append(cur); cur = None

fields = list(runs[0].keys())
with open(log[:-4] + ".csv", "w", newline="") as f:
    w = csv.DictWriter(f, fieldnames=fields); w.writeheader(); w.writerows(runs)

for h in head: print(h)
bad = [r for r in runs if r["status"] != "pass" or r["states"] != want_states or r["exit"] != 0]
print(f"runs={len(runs)} counted={sum(r['round'] >= 1 for r in runs)} not_as_expected={len(bad)}")
for r in bad: print("NOT AS EXPECTED:", r)
print()
variants = []
for r in runs:
    if r["variant"] not in variants: variants.append(r["variant"])
print("| Variant | Build | Workers | CPUs | Runs | Wall min | Wall median | Wall max | CPU min | CPU median | CPU max | Median user | Median sys | Median peak memory |")
print("|---|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
for v in variants:
    rs = [r for r in runs if r["variant"] == v and r["round"] >= 1]
    W = [r["wall_s"] for r in rs]; C = [r["cpu_s"] for r in rs]
    md = statistics.median
    print(f"| {v} | {rs[0]['build']} | {rs[0]['jobs']} | {rs[0]['cpus']} | {len(rs)} | {min(W):.2f} | {md(W):.2f} | {max(W):.2f} | "
          f"{min(C):.2f} | {md(C):.2f} | {max(C):.2f} | {md([r['user_s'] for r in rs]):.2f} | {md([r['sys_s'] for r in rs]):.2f} | "
          f"{md([r['max_rss_kb'] for r in rs])/1024:.0f} MiB |")
print()
rounds = sorted({r["round"] for r in runs})
print("Every run, wall s / CPU s (round 0 and below not counted):")
print()
print("| Variant | " + " | ".join(str(x) for x in rounds) + " |")
print("|---|" + "---:|" * len(rounds))
for v in variants:
    cells = []
    for x in rounds:
        rr = [r for r in runs if r["variant"] == v and r["round"] == x]
        cells.append(f"{rr[0]['wall_s']:.2f} / {rr[0]['cpu_s']:.2f}" if rr else "")
    print(f"| {v} | " + " | ".join(cells) + " |")
