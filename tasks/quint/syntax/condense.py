import json
import sys

root = sys.argv[1].rstrip("/") + "/"
phases_name, parse_name, map_name = sys.argv[2:5]


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
    return json.dumps(value, separators=(",", ":"))


def without_depth(value):
    if isinstance(value, dict):
        return {key: without_depth(inner) for key, inner in value.items() if key != "depth"}
    if isinstance(value, list):
        return [without_depth(inner) for inner in value]
    return value


def by_id(module):
    return {**module, "declarations": sorted(module["declarations"], key=lambda declaration: declaration["id"])}


def depths(value, found):
    if isinstance(value, dict):
        if "depth" in value and "id" in value:
            found[str(value["id"])] = value["depth"]
        for inner in value.values():
            depths(inner, found)
    elif isinstance(value, list):
        for inner in value:
            depths(inner, found)
    return found


def nodes(value, found):
    if isinstance(value, dict):
        if "id" in value:
            found.setdefault(str(value["id"]), value)
        for inner in value.values():
            nodes(inner, found)
    elif isinstance(value, list):
        for inner in value:
            nodes(inner, found)
    return found


def encode(value, index):
    if isinstance(value, list):
        return [encode(inner, index) for inner in value]
    if not isinstance(value, dict):
        return value
    base = index.get(str(value.get("id"))) if "id" in value else None
    if base is not None:
        kept = [key for key in base if key in value]
        extra = [key for key in value if key not in base]
        if list(value) == kept + extra and all(text(value[key]) == text(base[key]) for key in kept):
            encoded = {"$": value["id"]}
            removed = [key for key in base if key not in value]
            if removed:
                encoded["-"] = removed
            if extra:
                encoded["+"] = [[key, encode(value[key], index)] for key in extra]
            return encoded
    return {key: encode(inner, index) for key, inner in value.items()}


def decode(value, index):
    if isinstance(value, list):
        return [decode(inner, index) for inner in value]
    if not isinstance(value, dict):
        return value
    if "$" in value:
        base = index[str(value["$"])]
        removed = value.get("-", [])
        decoded = {key: inner for key, inner in base.items() if key not in removed}
        for key, inner in value.get("+", []):
            decoded[key] = decode(inner, index)
        return decoded
    return {key: decode(inner, index) for key, inner in value.items()}


def with_depth(declaration, depth):
    if str(declaration["id"]) in depth:
        return {**declaration, "depth": depth[str(declaration["id"])]}
    return declaration


def ordered(module, order, depth):
    by = {declaration["id"]: declaration for declaration in module["declarations"]}
    return {**module, "declarations": [with_depth(by[declaration], depth) for declaration in order]}


phases = read(phases_name)
parse = read(parse_name)
source_map = read(map_name)
if source_map != phases["sourceMap"]:
    sys.exit(f"{map_name}: quint parse --source-map differs from phase 2's source map")
if not parse["errors"]:
    ours = [by_id(module) for module in phases["modules"]]
    theirs = [by_id(without_depth(module)) for module in parse["modules"]]
    if ours != theirs:
        sys.exit(f"{parse_name}: quint parse's modules differ from phase 2's beyond order and depth")
index = nodes(phases["modules"], {})
order = [[declaration["id"] for declaration in module["declarations"]] for module in parse["modules"]]
depth = depths(parse["modules"], {})
condensed = {
    "stage": parse.get("stage"),
    "warnings": parse.get("warnings"),
    "errors": parse["errors"],
    "order": order,
    "depth": depth,
    "table": encode(parse.get("table"), index),
}
if not phases["errors"]:
    rebuilt = {
        "stage": condensed["stage"],
        "warnings": condensed["warnings"],
        "modules": [
            ordered(module, module_order, depth)
            for module, module_order in zip(phases["modules"], order)
        ],
        "table": decode(condensed["table"], index),
        "errors": condensed["errors"],
    }
    if text(rebuilt) != text(parse):
        sys.exit(f"{parse_name}: the condensed recording does not give back quint parse's output")
with open(phases_name, "w") as file:
    json.dump(phases, file, separators=(",", ":"))
with open(parse_name, "w") as file:
    json.dump(condensed, file, separators=(",", ":"))
