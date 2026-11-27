import json
import sys

root = sys.argv[1].rstrip("/") + "/"
typecheck_name, effects_name = sys.argv[2:4]


def text(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False)


def relative(value):
    if isinstance(value, dict):
        return {key: relative(inner) for key, inner in value.items()}
    if isinstance(value, list):
        return [relative(inner) for inner in value]
    if isinstance(value, str) and value.startswith(root):
        return value[len(root):]
    return value


class Nodes:
    def __init__(self):
        self.nodes = []
        self.index = {}

    def shared(self, value):
        if isinstance(value, dict):
            inner = {key: self.shared(item) for key, item in value.items()}
        elif isinstance(value, list):
            inner = [self.shared(item) for item in value]
        else:
            return value
        written = text(inner)
        if written not in self.index:
            self.index[written] = len(self.nodes)
            self.nodes.append(inner)
        return {"#": self.index[written]}

    def expanded(self, value):
        if isinstance(value, dict):
            if list(value) == ["#"]:
                return self.expanded(self.nodes[value["#"]])
            return {key: self.expanded(item) for key, item in value.items()}
        if isinstance(value, list):
            return [self.expanded(item) for item in value]
        return value


with open(typecheck_name) as file:
    written = file.read()
output = json.loads(written)
condensed = {"stage": output.get("stage"), "errors": relative(output.get("errors", [])[:1])}
effects = output.get("effects")
if effects is not None and not output.get("errors"):
    nodes = Nodes()
    ids = [[int(key), nodes.shared(value)["#"]] for key, value in effects.items()]
    rebuilt = {str(key): nodes.expanded({"#": node}) for key, node in ids}
    if text(rebuilt) != text(effects):
        sys.exit(f"{typecheck_name}: the condensed effects do not rebuild Quint's")
    previous = 0
    deltas = []
    for key, node in ids:
        deltas.append([key - previous, node])
        previous = key
    condensed["nodes"] = nodes.nodes
    condensed["effects"] = deltas
with open(effects_name, "w") as file:
    file.write(text(condensed))
