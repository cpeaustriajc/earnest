import glob
import os
import re
import sys

root = sys.argv[1]
SHAPES = "tasks/quint/shapes"
MODULE_FOLDERS = ["tasks/quint/imports"]
SAMPLES = "crates/earnest_quint_syntax/tests/samples/compile"
COMPILE = "compile --target json"

lines = []
for path in sorted(glob.glob(os.path.join(root, SHAPES, "*.qnt"))):
    relative = os.path.relpath(path, root)
    text = open(path).read()
    if "module U" not in text:
        continue
    values = set(re.findall(r"val (\w+) =", text)) | ({"farOk"} if "Far" in text else set())
    names = set(re.findall(r"module (\w+)", text)) | set(re.findall(r" as (\w+)", text))
    names |= {"Far"} if "Far" in text else set()
    names.discard("U")
    prefixes = [""] + [name + "::" for name in names]
    prefixes += [a + "::" + b + "::" for a in names for b in names if a != b]
    for value in sorted(values):
        for prefix in sorted(prefixes):
            lines.append(
                f"{COMPILE} --main U --init uInit --step uStep --invariant {prefix}{value} {relative}"
            )
for folder in MODULE_FOLDERS + [SAMPLES]:
    for path in sorted(glob.glob(os.path.join(root, folder, "*.qnt"))):
        relative = os.path.relpath(path, root)
        for module in re.findall(r"^\s*module (\w+)", open(path).read(), re.MULTILINE):
            lines.append(f"{COMPILE} --main {module} --init init --step step {relative}")
samples = os.path.join(SAMPLES, "{}.qnt")
lines += [
    f"{COMPILE} --main Lamp --init init --step step --invariant small {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step step --invariant small,known {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step flip --invariant n<5 {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step step --invariant true {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step step --temporal flips {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step step --invariant small --temporal flips,counts {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step n {samples.format('adds')}",
    f"{COMPILE} --main Lamp --init init --step missing {samples.format('adds')}",
    f"{COMPILE} --main Dim --init start --step go --invariant dimOk,L::small {samples.format('adds')}",
    f"{COMPILE} --main Dim --init L::init --step L::step --temporal L::flips {samples.format('adds')}",
    f"{COMPILE} --main Main --init init --step step --invariant ok {samples.format('drops')}",
    f"{COMPILE} --main Spec --init init --step step --invariant named,called,viaParam,valued,literal {samples.format('fieldnames')}",
    f"{COMPILE} --main Main --init init --step step --invariant named,called,viaParam,valued,literal {samples.format('fieldnames')}",
    f"{COMPILE} --main Through --init init --step step --invariant pOk {samples.format('crashes')}",
    f"{COMPILE} --main Setup --init init --step step --invariant sOk {samples.format('crashes')}",
]
print("\n".join(lines))
