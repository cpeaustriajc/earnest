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
condensed = {
    "errors": parse["errors"],
    "order": [[declaration["id"] for declaration in module["declarations"]] for module in parse["modules"]],
    "depth": depths(parse["modules"], {}),
}
with open(phases_name, "w") as file:
    json.dump(phases, file, separators=(",", ":"))
with open(parse_name, "w") as file:
    json.dump(condensed, file, separators=(",", ":"))
