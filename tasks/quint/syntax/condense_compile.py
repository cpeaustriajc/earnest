import json
import os
import sys
from collections import defaultdict

SMALLEST_SHARED = 24
HASHED = [] if os.environ.get("KEEP") == "all" else ["table", "types", "effects"]


def compact(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False)


def encoded(value, pool, index):
    if isinstance(value, dict):
        inner = {key: encoded(item, pool, index) for key, item in value.items()}
    elif isinstance(value, list):
        inner = [encoded(item, pool, index) for item in value]
    else:
        return value
    text = compact(inner)
    if len(text) <= SMALLEST_SHARED:
        return inner
    if text not in index:
        index[text] = len(pool)
        pool.append(inner)
    return {"$": index[text]}


def decoded(value, pool):
    if isinstance(value, dict):
        if set(value) == {"$"}:
            return decoded(pool[value["$"]], pool)
        return {key: decoded(item, pool) for key, item in value.items()}
    if isinstance(value, list):
        return [decoded(item, pool) for item in value]
    return value


def fnv(written):
    value = 0xCBF29CE484222325
    for byte in written.encode("utf-8"):
        value = ((value ^ byte) * 0x100000001B3) % (1 << 64)
    return f"{value:016x}"


def first_error(text):
    for line in text.splitlines():
        line = line.strip()
        if line:
            return line
    return ""


def main():
    root, out, scratch = sys.argv[1:4]
    by_file = defaultdict(list)
    names = sorted(
        (name for name in os.listdir(scratch) if name.endswith(".args")),
        key=lambda name: int(name.split(".")[0]),
    )
    for name in names:
        stem = os.path.join(scratch, name[: -len(".args")])
        args = open(stem + ".args").read().strip()
        stdout = open(stem + ".out").read()
        stderr = open(stem + ".err").read()
        by_file[args.split("\t")[-1]].append((args, stdout, stderr))
    for file, checks in sorted(by_file.items()):
        pool, index, written = [], {}, []
        for args, stdout, stderr in checks:
            try:
                output = json.loads(stdout)
            except ValueError:
                output = None
            if output is None:
                lines = [line.strip() for line in (stderr + "\n" + stdout).splitlines()]
                errors = [line for line in lines if line.startswith(("Error", "error"))]
                written.append({"args": args, "error": errors[0] if errors else first_error(stderr)})
                continue
            if compact(output) != stdout:
                sys.exit(f"{args}: Quint's stdout is not the compact JSON of what it holds")
            keys = [[key, len(compact(value).encode("utf-8")), fnv(compact(value))] for key, value in output.items()]
            rebuilt = "{" + ",".join(f"{compact(key)}:{compact(value)}" for key, value in output.items()) + "}"
            if rebuilt != stdout:
                sys.exit(f"{args}: Quint's stdout is not its keys' texts joined")
            kept = {key: value for key, value in output.items() if key not in HASHED}
            entry = {"args": args, "keys": keys, "output": encoded(kept, pool, index)}
            if decoded(entry["output"], pool) != kept:
                sys.exit(f"{args}: the condensed recording does not give back Quint's")
            written.append(entry)
        target = os.path.join(out, file + ".compile.json")
        os.makedirs(os.path.dirname(target), exist_ok=True)
        with open(target, "w") as handle:
            handle.write(compact({"pool": pool, "checks": written}))
            handle.write("\n")


main()
