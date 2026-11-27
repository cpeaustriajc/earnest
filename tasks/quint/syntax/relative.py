import json
import sys

root = sys.argv[1].rstrip("/") + "/"


def relative(value):
    if isinstance(value, dict):
        return {key: relative(inner) for key, inner in value.items()}
    if isinstance(value, list):
        return [relative(inner) for inner in value]
    if isinstance(value, str) and value.startswith(root):
        return value[len(root):]
    return value


for name in sys.argv[2:]:
    try:
        with open(name) as file:
            written = json.load(file)
    except FileNotFoundError:
        continue
    written.pop("table", None)
    with open(name, "w") as file:
        json.dump(relative(written), file, separators=(",", ":"))
