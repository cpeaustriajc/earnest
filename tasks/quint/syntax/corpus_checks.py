import os
import re
import sys

folder = sys.argv[1]
MODULE = re.compile(r"^\s*module\s+(\w+)\s*\{", re.MULTILINE)
lines = []
for directory, _, files in sorted(os.walk(folder)):
    if "node_modules" in directory:
        continue
    for name in sorted(files):
        if not name.endswith(".qnt"):
            continue
        path = os.path.join(directory, name)
        text = open(path, errors="replace").read()
        starts = [(match.start(), match.group(1)) for match in MODULE.finditer(text)]
        for at, (start, module) in enumerate(starts):
            end = starts[at + 1][0] if at + 1 < len(starts) else len(text)
            body = text[start:end]
            if re.search(r"\baction\s+init\b", body) and re.search(r"\baction\s+step\b", body):
                relative = os.path.relpath(path, folder)
                lines.append(f"compile --target json --main {module} --init init --step step {relative}")
print("\n".join("\t".join(line.split(" ")) for line in lines))
