# Group seconds before (old write-up's shares x this machine's median CPU of 9c5413e) and after
# (this profile's shares x this machine's median CPU of 10015fc), and the store work next to the floor.
import json, sys
S, OLD = sys.argv[1], sys.argv[2]
cpu_old = {"10m_j1": 3.396, "10m_j4": 4.641, "100m_j1": 50.26, "100m_j4": 59.486}
cpu_new = {"10m_j1": 2.88, "10m_j4": 3.782, "100m_j1": 39.975, "100m_j4": 50.583, "wide_j1": 0.323}
floor = {"10m": (2.788, 1.564, 2.078), "100m": (48.599, 26.199, 35.766)}
names = {"a":"(a) index of known states","b":"(b) packed rows","c":"(c) part tables, claim and merge","d":"(d) actions and rules","e":"(e) hashing","f":"(f) allocation and page faults","g":"(g) threads, waiting, other"}
O = {t: json.load(open(f"{OLD}/prof_{t}_groups.json")) for t in cpu_old}
N = {t: json.load(open(f"{S}/raw/prof_{t}_groups.json")) for t in cpu_new}
sh = lambda d, gs: sum(d["groups"].get(g, 0) for g in gs) / d["total"]
print("| Group | " + " | ".join(f"{t} before | {t} after | change" for t in cpu_old) + " | wide_j1 after |")
print("|---|" + "---:|---:|---:|" * 4 + "---:|")
for g in "abcdefg":
    row = [names[g]]
    for t in cpu_old:
        b, a = sh(O[t], g) * cpu_old[t], sh(N[t], g) * cpu_new[t]
        row += [f"{b:.2f}", f"{a:.2f}", f"{a-b:+.2f}"]
    row.append(f"{sh(N['wide_j1'], g)*cpu_new['wide_j1']:.3f}")
    print("| " + " | ".join(row) + " |")
print("| total | " + " | ".join(f"{cpu_old[t]:.2f} | {cpu_new[t]:.2f} | {cpu_new[t]-cpu_old[t]:+.2f}" for t in cpu_old) + f" | {cpu_new['wide_j1']:.3f} |")
print()
print("| Run | abce before | abce after | abcef after | ratio abce/ii | ratio abcef/ii | floor i | ii | iii | (d) before | (d) after | whole after |")
print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
for t in cpu_old:
    f = floor[t.split("_")[0]]
    b = sh(O[t], "abce") * cpu_old[t]; a = sh(N[t], "abce") * cpu_new[t]; af = sh(N[t], "abcef") * cpu_new[t]
    print(f"| {t} | {b:.2f} | {a:.2f} | {af:.2f} | {a/f[1]:.2f} | {af/f[1]:.2f} | {f[0]:.2f} | {f[1]:.2f} | {f[2]:.2f} | {sh(O[t],'d')*cpu_old[t]:.2f} | {sh(N[t],'d')*cpu_new[t]:.2f} | {cpu_new[t]:.2f} |")
