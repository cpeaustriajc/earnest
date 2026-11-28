import glob
import json
import os
import re
import subprocess
import tempfile

QUINT = os.environ["EARNEST_QUINT"]
EARNEST = os.environ["EARNEST"]
HERE = os.getcwd()
SCRATCH = tempfile.mkdtemp()

SAMPLES = {
    "board": ("board", ["xNeverBehind", "xHasTwo"], [], True),
    "ledger": ("ledger", ["conserved", "noDebt", "someoneEmpty"], [], True),
    "parts": ("house", ["tickBound", "neverLit"], [], True),
    "pool": ("pool", ["notBoth", "allDone"], [], True),
    "relay": ("relay", ["counted"], [], True),
    "ring": ("Two", ["mutex"], ["moves"], False),
    "tally": ("tally", [], [], False),
    "tallyfails": ("tallyfails", [], [], False),
}


def quint(*arguments):
    return subprocess.run([QUINT, *arguments], capture_output=True, text=True)


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as out:
        out.write(text.replace(HERE + "/", ""))


def read(path):
    with open(path) as source:
        return source.read()


def states_by_main():
    run = subprocess.run(
        [EARNEST, "confirm", "--quint", HERE], capture_output=True, text=True
    )
    return {
        match[1]: int(match[2])
        for match in re.finditer(r"^same\s+(\S+)\s+(\d+) states", run.stdout, re.M)
    }


def verdicts(name, main, invariants, temporals):
    found = {}
    for each in invariants:
        run = quint("verify", "--backend", "tlc", "--main", main, "--invariant", each, name + ".qnt")
        found[each] = "holds" if run.returncode == 0 else "broken"
    for each in temporals:
        run = quint("verify", "--backend", "tlc", "--main", main, "--temporal", each, name + ".qnt")
        found[each] = "holds" if run.returncode == 0 else "broken"
    return found


def runs(name, main):
    run = quint("test", "--main", main, "--backend", "typescript", name + ".qnt")
    found = {}
    for match in re.finditer(r"^\s+ok (\S+) passed", run.stdout, re.M):
        found[match[1].split("::")[-1]] = "holds"
    for match in re.finditer(r"^\s+\d+\) (\S+):", run.stdout, re.M):
        found[match[1].split("::")[-1]] = "fails"
    return found


def main():
    for old in glob.glob("fixtures/**/*.json", recursive=True):
        os.remove(old)
    states = states_by_main()
    expected = {}
    for name, (module, invariants, temporals, compiles) in sorted(SAMPLES.items()):
        source = name + ".qnt"
        parse = quint("parse", "--out", SCRATCH + "/sample.parse.json", "--source-map", SCRATCH + "/sample.sourcemap.json", source)
        assert parse.returncode == 0, parse.stderr
        write(f"fixtures/parsed/{name}.parse.json", read(SCRATCH + "/sample.parse.json"))
        write(f"fixtures/parsed/{name}.sourcemap.json", read(SCRATCH + "/sample.sourcemap.json"))
        typecheck = quint("typecheck", "--out", SCRATCH + "/sample.typecheck.json", source)
        if typecheck.returncode == 0:
            write(f"fixtures/typechecked/{name}.json", read(SCRATCH + "/sample.typecheck.json"))
        if compiles:
            compile_arguments = ["compile", "--target", "json", "--main", module, "--init", "init", "--step", "step"]
            if invariants:
                compile_arguments += ["--invariant", ",".join(invariants)]
            target = f"fixtures/compiled/{module}@{','.join(invariants)}@.json".replace("::", ".")
            os.makedirs(os.path.dirname(target), exist_ok=True)
            with open(target, "w") as out:
                compiled = subprocess.run([QUINT, *compile_arguments, source], stdout=out, stderr=subprocess.PIPE, text=True)
            assert compiled.returncode == 0, compiled.stderr
        expected[name] = {
            "module": module,
            "states": states.get(module),
            "rules": verdicts(name, module, invariants, temporals),
            "runs": runs(name, module),
        }
    write("fixtures/expected.json", json.dumps(expected, indent=2, sort_keys=True) + "\n")


main()
