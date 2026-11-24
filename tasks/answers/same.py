"""Checks that the release `earnest` of the working tree gives the same answers as the one of a commit.

Usage: python3 -I tasks/answers/same.py <commit>

Builds both, then compares their printed output, with times taken out, in two ways:

- the sweep: every model in `models/` checked by `verify` and by `report` at state limits around
  the end of every level and at six places inside every level that follows one of 512 states or
  more, by the tree
  at 1, 2, 4 and 10 workers against the commit at 1 worker;
- every model file of `tasks/models`, `tasks/benchmark/models` and `tasks/conformance`, by
  `verify` with room for 20 million states and by `report` at its default limit, at 1 and at 4
  workers, the tree against the commit at the same number.

A run that panics or is ended by a signal, such as the kernel's for lack of memory, is a
difference.

Prints one line for each sweep and one for the model files, with the runs of the tree and the
differences, then the first differences, and exits 1 when there is any.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
MODELS = os.path.join(HERE, "models")
SWEEP_WORKERS = (1, 2, 4, 10)
FILE_WORKERS = (1, 4)
INSIDE_A_LEVEL = 6
SMALLEST_LEVEL_SWEPT_INSIDE = 512
NEAR_A_LEVEL_END = 2
LARGEST_STATE_LIMIT = 20_000_000
RUN_SECONDS = 900
PARALLEL = 4
SHOWN = 10
MODES = {
    "verify": ["verify", "--no-cache", "--json"],
    "report": ["report", "--max-arrows", "100000"],
}
FOLDERS = ("tasks/models", "tasks/benchmark/models", "tasks/conformance")


def without_times(text):
    text = re.sub(r'"milliseconds": \d+', '"milliseconds": 0', text)
    return re.sub(r"\d+(\.\d+)? ?(ms|s)\b", "T", text)


def run(binary, args, path):
    done = subprocess.run([binary, *args, path], capture_output=True, text=True, timeout=RUN_SECONDS,
                          cwd=os.path.dirname(path))
    return done.returncode, without_times(done.stdout), done.stderr


def ended(result):
    code, _, errors = result
    return code >= 0 and "panick" not in errors


def build(folder, target):
    subprocess.run(["cargo", "build", "--release", "--locked", "-p", "earnest-cli"], cwd=folder, check=True,
                   env={**os.environ, "CARGO_TARGET_DIR": target})
    return os.path.join(target, "release", "earnest")


def reference(commit, scratch):
    tree = os.path.join(scratch, "reference")
    subprocess.run(["git", "worktree", "add", "--detach", tree, commit], cwd=ROOT, check=True,
                   capture_output=True)
    try:
        built = build(tree, os.path.join(ROOT, "target", "reference"))
        kept = os.path.join(scratch, "earnest-reference")
        shutil.copy2(built, kept)
        return kept
    finally:
        subprocess.run(["git", "worktree", "remove", "--force", tree], cwd=ROOT, check=True)


def limits_of(levels):
    limits = set()
    for end in sorted({end for _, end in levels}):
        limits.update(end + step for step in range(-NEAR_A_LEVEL_END, NEAR_A_LEVEL_END + 1) if end + step >= 1)
    for (before_start, before_end), (start, end) in zip(levels, levels[1:]):
        if before_end - before_start < SMALLEST_LEVEL_SWEPT_INSIDE:
            continue
        for k in range(1, INSIDE_A_LEVEL + 1):
            inside = start + ((end - start) * k) // (INSIDE_A_LEVEL + 1)
            limits.update([inside, inside + 1])
    return sorted(limits)


def sweep_one(binaries, job):
    tree, commit = binaries
    path, limit, mode = job
    args = MODES[mode] + ["--max-states", str(limit)]
    expected = run(commit, args + ["--jobs", "1"], path)
    found = [] if ended(expected) else [f"{os.path.basename(path)} {mode} --max-states {limit}: the commit"]
    for workers in SWEEP_WORKERS:
        got = run(tree, args + ["--jobs", str(workers)], path)
        if got[:2] != expected[:2] or not ended(got):
            found.append(f"{os.path.basename(path)} {mode} --max-states {limit} --jobs {workers}")
    return len(SWEEP_WORKERS), found


def file_one(binaries, job):
    tree, commit = binaries
    path, mode, workers = job
    limit = ["--max-states", str(LARGEST_STATE_LIMIT)] if mode == "verify" else []
    args = MODES[mode] + limit + ["--jobs", str(workers)]
    expected, got = run(commit, args, path), run(tree, args, path)
    same = got[:2] == expected[:2] and ended(got) and ended(expected)
    return 1, [] if same else [f"{os.path.relpath(path, ROOT)} {mode} --jobs {workers}"]


def sweep_of(name, levels):
    path = os.path.join(MODELS, name)
    return [(path, limit, mode) for limit in limits_of([tuple(level) for level in levels]) for mode in MODES]


def model_files():
    found = []
    for folder in FOLDERS:
        for place, _, names in os.walk(os.path.join(ROOT, folder)):
            found += [os.path.join(place, name) for name in names if name.endswith((".model.ts", ".rs"))]
    return sorted(found)


def counted(pool, binaries, work, one):
    results = list(pool.map(lambda job: one(binaries, job), work))
    return sum(runs for runs, _ in results), [found for _, each in results for found in each]


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: python3 -I tasks/answers/same.py <commit>")
    with tempfile.TemporaryDirectory() as scratch:
        commit = reference(sys.argv[1], scratch)
        tree = build(ROOT, os.path.join(ROOT, "target"))
        binaries = (tree, commit)
        levels = json.load(open(os.path.join(MODELS, "levels.json")))
        files = [(path, mode, workers) for path in model_files() for mode in MODES for workers in FILE_WORKERS]
        with ThreadPoolExecutor(PARALLEL) as pool:
            parts = [(f"sweep of {name}", counted(pool, binaries, sweep_of(name, levels[name]), sweep_one))
                     for name in sorted(levels)]
            parts.append(("every model file", counted(pool, binaries, files, file_one)))
    differences = []
    for name, (runs, found) in parts:
        print(f"{name}: {runs} runs, {len(found)} differences")
        differences += found
    for found in differences[:SHOWN]:
        print(f"differs: {found}")
    sys.exit(1 if differences else 0)


if __name__ == "__main__":
    main()
