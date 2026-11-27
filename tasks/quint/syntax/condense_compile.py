import json
import os
import sys
from collections import defaultdict

LEFT_OUT = ["types", "effects"]
SMALLEST_SHARED = 24


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


def field_names_arguments(value, found):
    if isinstance(value, dict):
        if value.get("kind") == "app" and value.get("opcode") == "fieldNames":
            for argument in value.get("args", []):
                found.append(str(argument["id"]))
        for item in value.values():
            field_names_arguments(item, found)
    elif isinstance(value, list):
        for item in value:
            field_names_arguments(item, found)


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
        by_file[args.split()[-1]].append((args, stdout, stderr))
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
            variables = [
                str(declaration["id"])
                for module in output["modules"]
                for declaration in module["declarations"]
                if declaration["kind"] == "var"
            ]
            types = output.get("types", {})
            variable_types = {id: types[id] for id in sorted(set(variables), key=int) if id in types}
            arguments = []
            field_names_arguments(output["modules"], arguments)
            argument_types = {id: types[id] for id in sorted(set(arguments), key=int) if id in types}
            for key in LEFT_OUT:
                output.pop(key, None)
            entry = {
                "args": args,
                "output": encoded(output, pool, index),
                "variableTypes": encoded(variable_types, pool, index),
                "fieldNamesTypes": encoded(argument_types, pool, index),
            }
            if decoded(entry["output"], pool) != output:
                sys.exit(f"{args}: the condensed recording does not give back Quint's")
            written.append(entry)
        target = os.path.join(out, file + ".compile.json")
        os.makedirs(os.path.dirname(target), exist_ok=True)
        with open(target, "w") as handle:
            handle.write(compact({"pool": pool, "checks": written}))
            handle.write("\n")


main()
