import json
import os
import sys

sys.setrecursionlimit(1_000_000)

root = sys.argv[1].rstrip("/") + "/"
typecheck_name, inferred_name, types_name = sys.argv[2:5]
parse_name = sys.argv[5] if len(sys.argv) > 5 else None


def relative(value):
    if isinstance(value, dict):
        return {key: relative(inner) for key, inner in value.items()}
    if isinstance(value, list):
        return [relative(inner) for inner in value]
    if isinstance(value, str) and value.startswith(root):
        return value[len(root):]
    return value


def read(name):
    with open(name) as file:
        return relative(json.load(file))


def text(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False)


def patches(parsed, typed, path, found):
    if isinstance(parsed, dict) and isinstance(typed, dict) and list(parsed) == list(typed):
        for key in parsed:
            patches(parsed[key], typed[key], path + [key], found)
    elif isinstance(parsed, list) and isinstance(typed, list) and len(parsed) == len(typed):
        for at, (inner, other) in enumerate(zip(parsed, typed)):
            patches(inner, other, path + [at], found)
    elif text(parsed) != text(typed):
        found.append([path, typed])


if os.path.exists(inferred_name):
    inferred = read(inferred_name)
    written = read(typecheck_name) if os.path.exists(typecheck_name) else {}
    if "types" in written and not inferred["errors"] and "failure" not in inferred:
        assert text(inferred["types"]) == text(written["types"]), typecheck_name
    schemes = []
    printed = []
    index = {}
    types = {}
    for key, scheme in inferred["types"].items():
        shown = text(scheme)
        if shown not in index:
            index[shown] = len(schemes)
            schemes.append(scheme)
            printed.append(inferred["printed"][key])
        assert printed[index[shown]] == inferred["printed"][key], key
        types[key] = index[shown]
    condensed = {"schemes": schemes, "printed": printed, "types": types, "errors": inferred["errors"][:3]}
    if "failure" in inferred:
        condensed["failure"] = inferred["failure"]
    if parse_name and os.path.exists(parse_name) and "types" in written and not written["errors"]:
        parsed = read(parse_name)
        found = []
        patches(parsed["modules"], written["modules"], ["modules"], found)
        patches(parsed["table"], written["table"], ["table"], found)
        condensed["patches"] = found
    if "errors" in written:
        condensed["typecheck"] = [error["explanation"] for error in written["errors"][:3]]
    rebuilt = {key: schemes[at] for key, at in types.items()}
    assert text(rebuilt) == text(inferred["types"]), inferred_name
    with open(types_name, "w") as file:
        file.write(text(condensed))
