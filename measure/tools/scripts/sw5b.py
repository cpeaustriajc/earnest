import os, re, subprocess, sys, json
B = os.path.dirname(os.path.abspath(__file__))
S = os.path.dirname(B)
TIP = os.environ.get("TIP", os.path.join(B, "earnest-fae"))
PROBE = os.path.join(B, "earnest-probe")
REF = os.environ.get("REF", os.path.join(S, "joined", "earnest-joined"))
ROWS = os.path.join(S, "rows", "earnest-rows-joined")
MODES = {"verify": ["verify", "--no-cache", "--json"], "report": ["report", "--max-arrows", "100000"]}
INSTR = os.environ.get("INSTR", "1") == "1"


def norm(s):
    s = re.sub(r'"milliseconds": \d+', '"milliseconds": 0', s)
    s = re.sub(r'\d+(\.\d+)? ?(ms|s)\b', 'T', s)
    return s


def run(binary, args, path, env=None):
    e = dict(os.environ)
    for k in ("FORCE_LEVEL", "FORCE_ROUND", "FORCE_DELAY_US", "EARNEST_OWN"):
        e.pop(k, None)
    e.update(env or {})
    p = subprocess.run([binary, *args, path], capture_output=True, text=True, timeout=900, cwd=os.path.dirname(path), env=e)
    return p.returncode, norm(p.stdout), p.stderr


def instructions(binary, path, jobs, limit):
    p = subprocess.run(["/usr/bin/time", "-l", binary, "verify", "--no-cache", "--json", "--jobs", str(jobs), "--max-states", str(limit), path],
                       capture_output=True, text=True, timeout=900, cwd=os.path.dirname(path))
    m = re.search(r"(\d+)\s+instructions retired", p.stderr)
    return int(m.group(1)) if m else None


def levels(path):
    _, _, err = run(PROBE, MODES["verify"] + ["--jobs", "1", "--max-states", "100000000"], path)
    return [tuple(map(int, m)) for m in re.findall(r"INST level (\d+)\.\.(\d+)", err)]


def rounds_by_level(err):
    most = {}
    for k, a in re.findall(r"INST round (\d+) level (\d+)\.\.", err):
        most[int(a)] = max(most.get(int(a), 1), int(k))
    return most


def limits_for(path, inside):
    lv = levels(path)
    ends = sorted({e for _, e in lv})
    out = set()
    edge = set()
    for e in ends:
        for d in range(-2, 3):
            if e + d >= 1:
                out.add(e + d); edge.add(e + d)
    for (s0, e0), (s, e) in zip(lv, lv[1:]):
        n = e - s
        if e0 - s0 < 512:
            continue
        for k in range(1, inside + 1):
            x = s + (n * k) // (inside + 1)
            out.update([x, x + 1])
    return sorted(out), edge, lv


if __name__ == "__main__":
    path = os.path.abspath(sys.argv[1])
    inside = int(sys.argv[2]) if len(sys.argv) > 2 else 6
    limits, edge, lv = limits_for(path, inside)
    runs = 0; diffs = []; edge_runs = 0; edge_diffs = 0
    most = (0, None)
    worst_ratio = (0.0, None)
    for limit in limits:
        for mode in ("verify", "report"):
            args = MODES[mode] + ["--max-states", str(limit)]
            ref = run(REF, args + ["--jobs", "1"], path)
            cells = [(f"cli{j}", TIP, ["--jobs", str(j)], None) for j in (1, 2, 4, 10)]
            cells += [(f"own{j}", PROBE, ["--jobs", "1"], {"EARNEST_OWN": str(j)}) for j in (2, 4, 10)]
            cells += [(f"probe-help{j}", PROBE, ["--jobs", str(j)], None) for j in (4, 10)]
            for name, binary, extra, env in cells:
                tip = run(binary, args + extra, path, env); runs += 1
                if limit in edge:
                    edge_runs += 1
                if binary == PROBE:
                    for a, k in rounds_by_level(tip[2]).items():
                        if k > most[0]:
                            most = (k, (limit, mode, name, a))
                    if re.search(r"CHECK .* bad \[(.+)\]", tip[2]):
                        print("CHECKBAD", limit, mode, name, re.search(r"bad \[(.+)\]", tip[2]).group(1), flush=True)
                if tip[0] != ref[0] or tip[1] != ref[1] or "panick" in tip[2]:
                    diffs.append((limit, mode, name))
                    if limit in edge:
                        edge_diffs += 1
                    base = os.path.join(B, "swdiff", f"{os.path.basename(path)}.{mode}.{limit}.{name}")
                    os.makedirs(os.path.dirname(base), exist_ok=True)
                    open(base + ".tip", "w").write(tip[1] + "\nERR " + tip[2]); open(base + ".ref", "w").write(ref[1] + "\nERR " + ref[2])
                    print("DIFF", limit, mode, name, flush=True)
        if INSTR:
            for j in (4, 10):
                a = instructions(TIP, path, j, limit); r = instructions(ROWS, path, j, limit)
                if a and r and a / r > worst_ratio[0]:
                    worst_ratio = (a / r, (limit, j, a, r))
    print(json.dumps({"model": os.path.basename(path), "levels": len(lv), "limits": len(limits), "runs": runs,
                      "diffs": len(diffs), "diff_list": diffs[:20], "edge_runs": edge_runs, "edge_diffs": edge_diffs,
                      "most_rounds_one_level": most, "worst_instr_ratio_j4_j10": worst_ratio}), flush=True)
