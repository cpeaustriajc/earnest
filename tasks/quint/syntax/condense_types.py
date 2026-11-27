import json
import os
import sys

root = sys.argv[1].rstrip("/") + "/"
typecheck_name, inferred_name, types_name = sys.argv[2:5]


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


if os.path.exists(inferred_name):
    inferred = read(inferred_name)
    written = read(typecheck_name) if os.path.exists(typecheck_name) else {}
    if "types" in written and not inferred["errors"] and "failure" not in inferred:
        assert text(inferred["types"]) == text(written["types"]), typecheck_name
    schemes = []
    index = {}
    types = {}
    for key, scheme in inferred["types"].items():
        shown = text(scheme)
        if shown not in index:
            index[shown] = len(schemes)
            schemes.append(scheme)
        types[key] = index[shown]
    condensed = {"schemes": schemes, "types": types, "errors": inferred["errors"][:3]}
    if "failure" in inferred:
        condensed["failure"] = inferred["failure"]
    if "errors" in written:
        condensed["typecheck"] = [error["explanation"] for error in written["errors"][:3]]
    rebuilt = {key: schemes[at] for key, at in types.items()}
    assert text(rebuilt) == text(inferred["types"]), inferred_name
    with open(types_name, "w") as file:
        file.write(text(condensed))
