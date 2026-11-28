import os
import random
import sys

seed, count, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
rng = random.Random(seed)
os.makedirs(out, exist_ok=True)
NAMES = ["x", "y", "w0", "w1", "f", "g"]


def atom():
    return rng.choice(["1", "true", "false", "()", "{w1}", "x", "y", "{()}", "[]", "\"s\""] + NAMES)


counter = [0]


def e(d):
    r = rng.random()
    if d > 4 or r < 0.22:
        return atom()
    if r < 0.40:
        q = rng.choice(["val", "def", "pure val", "nondet", "val", "def"])
        counter[0] += 1
        n = f"l{counter[0]}"
        head = f"{q} {n}" if q != "def" or rng.random() < 0.5 else f"def {n}({rng.choice(['p' + str(d), f'p{d}, r{d}', '_'])})"
        if q in ("val", "pure val") and rng.random() < 0.15:
            head = f"{q} ({n}, _)" if rng.random() < 0.5 else f"{q} {{ {n} }}"
        return f"{head} = {e(d+1)} {e(d+1)}"
    if r < 0.55:
        return f"{e(d+1)} {rng.choice(['+', '-', '*', '==', 'and', 'or', 'iff', 'implies', '->', '<'])} {e(d+1)}"
    if r < 0.60:
        return f"-{e(d+1)}"
    if r < 0.66:
        return f"{e(d+1)}[{e(d+1)}]"
    if r < 0.72:
        return f"{rng.choice(['f', 'g', 'and', 'or', 'implies', 'Set', 'List'])}({e(d+1)})"
    if r < 0.76:
        return f"{rng.choice(['and', 'or'])} {{ {e(d+1)}, {e(d+1)} }}"
    if r < 0.80:
        return f"match {e(d+1)} {{ _ => {e(d+1)} }}"
    if r < 0.84:
        return f"if ({e(d+1)}) {e(d+1)} else {e(d+1)}"
    if r < 0.88:
        return f"({e(d+1)}, {e(d+1)})"
    if r < 0.91:
        return f"{{ f: {e(d+1)} }}"
    if r < 0.94:
        return f"{{ ...{e(d+1)}, f: {e(d+1)} }}"
    if r < 0.97:
        return f"{rng.choice([f'p{d}', f'(p{d}, r{d})', '_'])} => {e(d+1)}"
    return f"({e(d+1)})"


for i in range(count):
    decls = " ".join(f"val v{k} = {e(0)}" for k in range(rng.randint(1, 3)))
    text = f"module G {{ var x: int var y: int val w0 = 1 val w1 = 2 def f(q) = q def g(q) = q {decls} }}\n"
    if rng.random() < 0.5:
        text = text.replace(" ", "", rng.randint(0, 6))
    with open(f"{out}/n{i:05}.qnt", "w") as written:
        written.write(text)
