#!/bin/bash
from=$1; to=$2; out=$3; mkdir -p "$out"
E=${EARNEST:-earnest}
here="$(cd "$(dirname "$0")" && pwd)"
for s in $(seq "$from" "$to"); do
  timeout 60 python3 -I "$here"/gen101.py "$s" "$out" 2>/dev/null || echo "$s generror"
done
cd "$out" || exit 1
"$E" verify --json --jobs 1 . > jobs1.json 2>/dev/null
"$E" verify --json --jobs 4 --no-cache . > jobs4.json 2>/dev/null
"$E" verify --json --jobs 4 . > cached.json 2>/dev/null
python3 -I - "$out" <<'PY'
import glob, json, os, sys
out = sys.argv[1]
expected = {}
for path in glob.glob(os.path.join(out, "h*.expect")):
    module = os.path.basename(path)[:-len(".expect")]
    for line in open(path):
        if line.strip():
            run, verdict = line.split()[:2]
            expected[(module, run)] = verdict
for name in ("jobs1", "jobs4", "cached"):
    report = json.load(open(os.path.join(out, name + ".json")))
    got = {}
    for model in report.get("models", []):
        for rule in model["rules"]:
            got[(model["name"], rule["id"])] = rule["verdict"]
    unreadable = {u.get("module") or u.get("name") for u in report.get("unreadable", [])}
    agree = differ = refused = 0
    for (module, run), verdict in sorted(expected.items()):
        found = got.get((module, run))
        if found is None:
            refused += 1
        elif found == verdict:
            agree += 1
        else:
            differ += 1
            print(name, module, run, "oracle", verdict, "earnest", found)
    print(f"{name}: {len(expected)} runs, {agree} agree, {differ} differ, {refused} not played")
PY
