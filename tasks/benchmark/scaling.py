"""Measures earnest and TLC on the grid models from 1,000 to 10,000,000 states, and draws the chart.

Usage:
  python3 -I tasks/benchmark/scaling.py measure <earnest> <tla2tools.jar> <runs> <workers>[,...] <out.json>
  python3 -I tasks/benchmark/scaling.py chart <in.json> <out.svg>

`measure` exports each grid model with `earnest tla` into a temporary folder, so TLC checks the
same state space. For each model and number of workers it runs each tool once unmeasured, then
`<runs>` times, and writes the median wall time and the largest peak resident set of any measured
run to JSON. GNU time (`/usr/bin/time`, Linux) reads the peak resident set: it forks each run from
its own small process, so the peak is the tool's, not this script's. It fails when the two tools
count different distinct states. A TLC run that takes longer than `TLC_SECONDS` is stopped and
written down as a timeout. Java options set in the environment are dropped, so TLC runs with the
JVM's defaults, as `earnest confirm --tlc` starts it. When `<out.json>` exists, its cases with
other numbers of workers are kept. `chart` draws the cases with the most workers.
"""
import hashlib
import json
import math
import os
import platform
import re
import signal
import statistics
import subprocess
import sys
import tempfile
import time

MODELS = [("grid_1k", "Grid1k", 1_000), ("grid_100k", "Grid100k", 100_000),
          ("grid_1m", "Grid1m", 1_000_000), ("grid_10m", "Grid10m", 10_000_000)]
STATE_LIMIT = "20000000"
TLC_SECONDS = 900
TLC_RELEASES = {"fdffef247f563ef6edbfd44958d2a82db79e7564e07cb673262d896353fa6713": "1.8.0"}
GNU_TIME = "/usr/bin/time"
JAVA_ENVIRONMENT_DROPPED = ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS")


def timed(argv, seconds):
    environment = {key: value for key, value in os.environ.items() if key not in JAVA_ENVIRONMENT_DROPPED}
    with tempfile.TemporaryFile() as output, tempfile.NamedTemporaryFile("r") as peak:
        actions = [(os.POSIX_SPAWN_DUP2, output.fileno(), 1), (os.POSIX_SPAWN_DUP2, output.fileno(), 2)]
        measured = [GNU_TIME, "--format", "%M", "--output", peak.name, *argv]
        start = time.perf_counter()
        pid = os.posix_spawn(GNU_TIME, measured, environment, file_actions=actions, setpgroup=0)
        deadline = start + seconds
        while True:
            done, status, _ = os.wait4(pid, os.WNOHANG)
            if done:
                break
            if time.perf_counter() > deadline:
                os.killpg(pid, signal.SIGKILL)
                os.wait4(pid, 0)
                return None, None, ""
            time.sleep(0.001)
        wall = time.perf_counter() - start
        output.seek(0)
        text = output.read().decode(errors="replace")
        memory = int(peak.read().split()[-1]) / 1024
    if os.waitstatus_to_exitcode(status) != 0:
        raise SystemExit(f"{argv[0]} exited with {status}:\n{text[-2000:]}")
    return wall, memory, text


def earnest_run(binary, model_file, workers):
    argv = [binary, "verify", "--no-cache", "--json", "--jobs", str(workers), "--max-states", STATE_LIMIT,
            model_file]
    wall, memory, text = timed(argv, 3600)
    model = json.loads(text)["models"][0]
    if model["status"] != "pass":
        raise SystemExit(f"earnest: {model_file} is {model['status']}")
    return wall, memory, model["states"]


def tlc_run(jar, folder, name, workers):
    with tempfile.TemporaryDirectory() as meta:
        argv = ["java", "-cp", jar, "tlc2.TLC", "-workers", str(workers), "-metadir", meta,
                "-config", os.path.join(folder, f"{name}.cfg"), os.path.join(folder, f"{name}.tla")]
        wall, memory, text = timed(argv, TLC_SECONDS)
    if wall is None:
        return None, memory, None
    if "No error has been found" not in text:
        raise SystemExit(f"TLC found an error in {name}:\n{text[-2000:]}")
    return wall, memory, int(re.findall(r"([\d,]+) distinct states found", text)[-1].replace(",", ""))


def measure_case(run, runs):
    run()
    walls, memories, counts = [], [], set()
    for _ in range(runs):
        wall, memory, states = run()
        if wall is None:
            return {"timeout_seconds": TLC_SECONDS}
        memories.append(memory)
        walls.append(wall)
        counts.add(states)
    if len(counts) != 1:
        raise SystemExit(f"state counts differ between runs: {counts}")
    return {"median_seconds": round(statistics.median(walls), 4), "walls": [round(w, 4) for w in sorted(walls)],
            "peak_mib": round(max(memories), 1), "states": counts.pop()}


def processor():
    with open("/proc/cpuinfo") as cpuinfo:
        for line in cpuinfo:
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    return platform.processor()


def measure(binary, jar, runs, workers_list, out):
    root = os.path.dirname(os.path.abspath(__file__))
    binary, jar = os.path.abspath(binary), os.path.abspath(jar)
    java = subprocess.run(["java", "-version"], capture_output=True, text=True,
                          env={k: v for k, v in os.environ.items() if k not in JAVA_ENVIRONMENT_DROPPED})
    tlc = subprocess.run(["java", "-cp", jar, "tlc2.TLC", "-help"], capture_output=True, text=True)
    with open(jar, "rb") as file:
        digest = hashlib.sha256(file.read()).hexdigest()
    with open("/proc/meminfo") as meminfo:
        memory_kib = int(meminfo.readline().split()[1])
    result = {
        "machine": {"processor": processor(), "cores": os.cpu_count(), "memory_gib": round(memory_kib / 2**20, 1),
                    "os": platform.platform()},
        "earnest": subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip(),
        "java": java.stderr.strip().splitlines()[0],
        "tlc": TLC_RELEASES.get(digest, "unknown release"), "tlc_sha256": digest,
        "tlc_build": next((line.split("Version")[1].strip() for line in tlc.stdout.splitlines() if "Version" in line),
                          ""),
        "tlc_timeout_seconds": TLC_SECONDS, "cases": [],
    }
    with tempfile.TemporaryDirectory() as exports:
        for file, name, expected in MODELS:
            model_file = os.path.join(root, "models", f"{file}.model.ts")
            folder = os.path.join(exports, file)
            subprocess.run([binary, "tla", model_file, "--out", folder], check=True, capture_output=True)
            for workers in workers_list:
                earnest = measure_case(lambda: earnest_run(binary, model_file, workers), runs)
                tlc = measure_case(lambda: tlc_run(jar, folder, name, workers), runs)
                for tool, case in (("earnest", earnest), ("TLC", tlc)):
                    if "states" in case and case["states"] != expected:
                        raise SystemExit(f"{tool} counted {case['states']} states of {name}, not {expected}")
                result["cases"].append({"model": file, "states": expected, "workers": workers, "runs": runs,
                                        "earnest": earnest, "tlc": tlc})
                print(json.dumps(result["cases"][-1]), file=sys.stderr)
    if os.path.exists(out):
        with open(out) as file:
            kept = [case for case in json.load(file)["cases"] if case["workers"] not in workers_list]
        result["cases"] = sorted(kept + result["cases"], key=lambda case: (-case["workers"], case["states"]))
    with open(out, "w") as file:
        json.dump(result, file, indent=2)
        file.write("\n")


STYLE = """
    :root {
      color-scheme: light dark;
      --ink: #0a0a0a;
      --muted: #737373;
      --hairline: rgba(115, 115, 115, 0.22);
      --axis: rgba(115, 115, 115, 0.45);
      --earnest: #0a0a0a;
      --tlc: #dc2626;
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --ink: #f0f6fc;
        --muted: #8b949e;
        --hairline: rgba(240, 246, 252, 0.14);
        --axis: rgba(240, 246, 252, 0.38);
        --earnest: #f0f6fc;
        --tlc: #f87171;
      }
    }
    text { font-family: ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; }
    .title { fill: var(--ink); font-size: 14px; font-weight: 600; }
    .note { fill: var(--muted); font-size: 11px; }
    .tick { fill: var(--muted); font-size: 11px; }
    .grid { stroke: var(--hairline); stroke-width: 1; }
    .axis { stroke: var(--axis); stroke-width: 1; }
    .line-earnest { stroke: var(--earnest); stroke-width: 2; fill: none; }
    .line-tlc { stroke: var(--tlc); stroke-width: 2; fill: none; }
    .dot-earnest { fill: var(--earnest); }
    .dot-tlc { fill: var(--tlc); }
    .label-earnest { fill: var(--earnest); font-size: 11px; font-weight: 600; }
    .label-tlc { fill: var(--tlc); font-size: 11px; font-weight: 600; }
"""

def states_label(states):
    if states >= 1_000_000:
        return f"{states // 1_000_000}M"
    return f"{states // 1_000}k"


def seconds_label(seconds):
    if seconds < 1:
        return f"{seconds * 1000:.0f} ms"
    if seconds < 10:
        return f"{seconds:.2f} s"
    return f"{seconds:.1f} s"


def mib_label(mib):
    return f"{mib / 1024:.2f} GiB" if mib >= 1024 else f"{mib:.0f} MiB"


def axis_ticks(ticks, values):
    present = [v for v in values if v is not None]
    high = next(v for v, _ in ticks if v >= max(present))
    low = max(v for v, _ in ticks if v <= min(present) / 2)
    return [t for t in ticks if low <= t[0] <= high]


def panel(x0, title, note, states, values, ticks, label):
    top, height = 76, 224
    left, right, bottom = x0 + 56, x0 + 344, top + height
    low, high = math.log10(ticks[0][0]), math.log10(ticks[-1][0])
    fewest, most = math.log10(states[0]), math.log10(states[-1])

    def px(count):
        return left + (right - left) * (math.log10(count) - fewest) / (most - fewest)

    def py(value):
        return bottom - height * (math.log10(value) - low) / (high - low)

    parts = [f'  <text x="{x0}" y="32" class="title">{title}</text>',
             f'  <text x="{x0}" y="50" class="note">{note}</text>']
    for value, text in ticks:
        y = py(value)
        parts.append(f'  <line x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}" class="grid"/>')
        parts.append(f'  <text x="{left - 10}" y="{y + 4:.1f}" class="tick" text-anchor="end">{text}</text>')
    parts.append(f'  <line x1="{left}" y1="{bottom}" x2="{right}" y2="{bottom}" class="axis"/>')
    for count in states:
        parts.append(f'  <text x="{px(count):.1f}" y="{bottom + 18}" class="tick" text-anchor="middle">'
                     f'{states_label(count)}</text>')
    parts.append(f'  <text x="{(left + right) / 2:.1f}" y="{bottom + 36}" class="note" text-anchor="middle">'
                 'distinct states, log scale</text>')
    for tool in ("tlc", "earnest"):
        points = [(px(c), py(v)) for c, v in zip(states, values[tool]) if v is not None]
        path = " ".join(f"{'M' if i == 0 else 'L'} {x:.1f} {y:.1f}" for i, (x, y) in enumerate(points))
        parts.append(f'  <path d="{path}" class="line-{tool}"/>')
        for index, (count, value) in enumerate(zip(states, values[tool])):
            x = px(count)
            if value is None:
                parts.append(f'  <text x="{x:.1f}" y="{top - 4}" class="label-{tool}" text-anchor="end">'
                             'stopped: timeout</text>')
                continue
            y = py(value)
            if tool == "tlc":
                anchor, dx, dy = ("start", -4, -11) if index == 0 else ("middle", 0, -11)
            else:
                anchor, dx, dy = ("end", -8, -8) if index == len(states) - 1 else ("start", 7, 17)
            parts.append(f'  <circle cx="{x:.1f}" cy="{y:.1f}" r="3.5" class="dot-{tool}"/>')
            parts.append(f'  <text x="{x + dx:.1f}" y="{y + dy:.1f}" class="label-{tool}" '
                         f'text-anchor="{anchor}">{label(value)}</text>')
    return parts


def chart(source, out):
    with open(source) as file:
        result = json.load(file)
    workers = max(case["workers"] for case in result["cases"])
    cases = sorted((c for c in result["cases"] if c["workers"] == workers), key=lambda c: c["states"])
    states = [c["states"] for c in cases]
    times = {tool: [c[tool].get("median_seconds") for c in cases] for tool in ("earnest", "tlc")}
    memory = {tool: [c[tool].get("peak_mib") for c in cases] for tool in ("earnest", "tlc")}
    time_ticks = axis_ticks([(0.001, "1 ms"), (0.01, "10 ms"), (0.1, "100 ms"), (1, "1 s"), (10, "10 s"),
                             (100, "100 s"), (1000, "1,000 s")], times["earnest"] + times["tlc"])
    memory_ticks = axis_ticks([(1, "1 MiB"), (4, "4 MiB"), (16, "16 MiB"), (64, "64 MiB"), (256, "256 MiB"), (1024, "1 GiB"),
                               (4096, "4 GiB"), (16384, "16 GiB")], memory["earnest"] + memory["tlc"])

    def listed(values, label):
        return ", ".join(f"{label(v)} at {states_label(c)}" if v is not None else f"timeout at {states_label(c)}"
                         for c, v in zip(states, values))

    tlc = f"TLC {result['tlc']}"
    summary = (f"Wall time and peak memory of {result['earnest']} and {tlc} on the same grid models, "
               f"{workers} workers each. Lower is better. Median wall time, earnest: "
               f"{listed(times['earnest'], seconds_label)}; TLC: {listed(times['tlc'], seconds_label)}. "
               f"Peak memory, earnest: {listed(memory['earnest'], mib_label)}; "
               f"TLC: {listed(memory['tlc'], mib_label)}.")
    parts = [f'<svg width="800" height="384" viewBox="0 0 800 384" fill="none" xmlns="http://www.w3.org/2000/svg" '
             f'role="img" aria-label="{summary}">', f"  <style>{STYLE}  </style>"]
    parts += panel(24, "Wall time", f"median of {cases[0]['runs']} runs, {workers} workers, log scale", states,
                   times, time_ticks, seconds_label)
    parts += panel(424, "Peak memory", "largest resident set of any run, log scale", states, memory,
                   memory_ticks, mib_label)
    parts += ['  <line x1="24" y1="364" x2="42" y2="364" class="line-earnest"/>',
              f'  <text x="50" y="368" class="label-earnest">{result["earnest"]}</text>',
              '  <line x1="164" y1="364" x2="182" y2="364" class="line-tlc"/>',
              f'  <text x="190" y="368" class="label-tlc">{tlc}</text>',
              '  <text x="768" y="368" class="note" text-anchor="end">'
              'Same TLA+ export, same distinct states. Lower is better.</text>',
              "</svg>"]
    with open(out, "w") as file:
        file.write("\n".join(parts) + "\n")


if __name__ == "__main__":
    if sys.argv[1] == "measure":
        measure(sys.argv[2], sys.argv[3], int(sys.argv[4]), [int(w) for w in sys.argv[5].split(",")], sys.argv[6])
    elif sys.argv[1] == "chart":
        chart(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(__doc__)
