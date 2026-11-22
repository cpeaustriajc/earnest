import json, subprocess
S="/SCRATCH"
cpu = {"10m_j1": (2.88, 2.702), "10m_j4": (3.782, 3.617), "100m_j1": (39.975, 39.74), "100m_j4": (50.583, 50.186), "wide_j1": (0.323, 0.315)}
names = {"a":"(a) index of known states","b":"(b) packed rows","c":"(c) part tables, claim and merge","d":"(d) actions and rules","e":"(e) hashing","f":"(f) allocation and page faults","g":"(g) threads, waiting, other"}
D = {t: json.load(open(f"{S}/raw/prof_{t}_groups.json")) for t in cpu}
print("| Group | " + " | ".join(f"{t} share | {t} CPU s" for t in cpu) + " |")
print("|---|" + "---:|---:|" * len(cpu))
for g in "abcdefg":
    row = [names[g]]
    for t in cpu:
        d = D[t]; sh = d["groups"].get(g, 0) / d["total"]
        row += [f"{100*sh:.1f}%", f"{sh*cpu[t][0]:.2f}"]
    print("| " + " | ".join(row) + " |")
print("| samples | " + " | ".join(f"{D[t]['total']} | {cpu[t][0]:.2f} (median)" for t in cpu) + " |")
print()
print("| Run | a+b+c+e share | CPU s (median CPU) | CPU s (smallest CPU) | (d) CPU s | (d) smallest |")
print("|---|---:|---:|---:|---:|---:|")
for t in cpu:
    d = D[t]; sh = sum(d["groups"].get(g,0) for g in "abce")/d["total"]; shd = d["groups"].get("d",0)/d["total"]
    print(f"| {t} | {100*sh:.1f}% | {sh*cpu[t][0]:.2f} | {sh*cpu[t][1]:.2f} | {shd*cpu[t][0]:.2f} | {shd*cpu[t][1]:.2f} |")
print()
for t in cpu:
    d = D[t]
    print(f"### {t} top functions")
    syms = [k for k, n, g in d["symbols"][:10]]
    dem = subprocess.run(["rustfilt"], input="\n".join(syms), capture_output=True, text=True).stdout.splitlines()
    for (k, n, g), name in zip(d["symbols"][:10], dem):
        gs = ", ".join(f"{x} {100*v/n:.0f}%" for x, v in sorted(g.items(), key=lambda x: -x[1]))
        print(f"| {100*n/d['total']:.2f}% | `{name[:150]}` | {gs} |")
    print()
for t in cpu:
    d=D[t]; print("### subgroups", t)
    for g, s, n in d["subgroups"][:24]: print(f"| {g} | {s[:100]} | {100*n/d['total']:.2f}% |")
    print("kernel sites")
    for a,b,n in d["kernel_sites"][:6]: print(f"| {a} | {b[:80]} | {100*n/d['total']:.2f}% |")
