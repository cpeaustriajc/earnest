import glob
import re
import sys

EXAMPLES = sys.argv[1]
TASKS = "../.."

SHAPES = [
    ("one module with vars, init, step and an invariant", r"(?s)\bvar \w+.*action init.*action step", "models/counter.qnt"),
    ("several modules in one file", r"(?s)\nmodule .*\nmodule ", "setups/clock.qnt"),
    ("types module imported by a spec", r"import \w*([Tt]ype|[Dd]ef|[Bb]ase)\w*\.\*", "review/typesmodule/light.qnt"),
    ("spec with constants", r"(?m)^\s*const \w+", "setups/clock.qnt"),
    ("setup module: import M(C = v).*", r"import \w+\([^)]*=[^)]*\)\.\*", "setups/lamp.qnt"),
    ("instance under a name: import M(C = v) as A, then A::name", r"import \w+\([^)]*=[^)]*\) as \w+", "first/TwoInstances.qnt"),
    ("modules with their own variables joined by import M as A", r"(?s)var \w+.*import \w+ as \w+.*::", "samples/parts.qnt"),
    ("import M.* from another file", r"import \w+\.\*\s+from", "shapes/from.qnt"),
    ("export", r"(?m)^\s*export ", "shapes/export.qnt"),
    ("assume on constants", r"\bassume\b", "review/specassume/specassume.qnt"),
    ("library of pure def and pure val", r"pure (def|val)", "setups/pure.qnt"),
    ("init picks a start with nondet", r"(?s)action init[^=]*=\s*\{?\s*nondet", "models/starts.qnt"),
    ("step is any { ... } of guarded actions", r"\bany\s*\{", "setups/lamp.qnt"),
    ("helper action with arguments fed by nondet picks", r"(?s)nondet \w+ = [^\n]*oneOf\(\).{0,200}\w+\(\w+", "setups/clock.qnt"),
    ("match on a sum type", r"\bmatch\b", "review/sumtype/sumtype.qnt"),
    ("sum type with a payload", r"\| \w+\(\w+", "samples/board.qnt"),
    ("pure transition function returning a record result", r"(pure )?def \w+\([^)]*\)\s*:\s*(\{[^}]*\w+: [^}]*\}|\w*(Result|Res)\b)", "samples/ledger.qnt"),
    ("map of maps", r"->\s*\(\w+\s*->|\.get\([^)]*\)\.get\(", "samples/board.qnt"),
    ("set of records as a pool", r"var \w+\s*:\s*Set\[(\{|[A-Z]\w*)", "samples/pool.qnt"),
    ("list state grown with append", r"\.append\(", "setups/readsetup.qnt"),
    ("nondet start or pick from powerset, setOfMaps or tuples", r"nondet \w+ = [^\n]*(powerset|setOfMaps|tuples)", "samples/pool.qnt"),
    ("record updated with with(...)", r"\.with\(", "setups/readsetup.qnt"),
    ("map updated with set or setBy", r"\.set\(|\.setBy\(", "samples/ledger.qnt"),
    ("per-process map over 1.to(N) with a forall invariant", r"(?s)1\.to\(\w+\).*forall", "samples/ring.qnt"),
    ("fold over a set or map in a val", r"\.fold\(", "models/bank.qnt"),
    ("temporal property without fairness", r"temporal \w+ = (always|eventually)", "setups/clock.qnt"),
    ("temporal property under weakFair or strongFair", r"weakFair|strongFair", "setups/clock.qnt"),
    ("run: init.then(...).expect(...)", r"(?s)\.then\(.*\.expect\(", "setups/runs.qnt"),
    ("run that must fail: .fail()", r"\.fail\(", "models/runs_ints.qnt"),
    ("run with arguments called by other runs", r"run \w+\(\w+", "first/Helpers.qnt"),
    ("runs of pure definitions alone: no var, no init, all { assert(...) }", r"(?s)^(?!.*\bvar ).*run \w+ = (\{[^}]*)?all \{\s*assert\(", "samples/tally.qnt"),
]


def folder_files(folder):
    return sorted(glob.glob(folder + "/**/*.qnt", recursive=True))


def text(path):
    with open(path) as source:
        return source.read()


def main():
    files = folder_files(EXAMPLES)
    print(len(files), "examples")
    for name, pattern, sample in SHAPES:
        count = sum(1 for each in files if re.search(pattern, text(each)))
        holds = re.search(pattern, text(f"{TASKS}/quint/{sample}" if not sample.startswith("samples/") else sample.split("/", 1)[1])) is not None
        print(f"{count:3}  {'ok ' if holds else 'NO '} {sample:36} {name}")


main()
