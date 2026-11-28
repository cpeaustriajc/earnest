import os
import random
import re
import sys

seed, count, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
rng = random.Random(seed)
os.makedirs(out, exist_ok=True)


def atom(names):
    return rng.choice(names + ["x", "y", "1", "2", "inc(x)", "x.inc()", "(x + 1)"])


def expr(names, d=0):
    r = rng.random()
    if d > 2 or r < 0.3:
        return atom(names)
    if r < 0.55:
        return f"{expr(names, d+1)} {rng.choice(['+', '-', '*'])} {expr(names, d+1)}"
    if r < 0.65:
        return f"if ({expr(names, d+1)} > 0) {expr(names, d+1)} else {expr(names, d+1)}"
    if r < 0.75:
        return f"Set({expr(names, d+1)}, {expr(names, d+1)}).size()"
    if r < 0.85:
        return f"inc({expr(names, d+1)})"
    return f"({expr(names, d+1)})"


def cond(names):
    return f"{expr(names)} {rng.choice(['==', '>', '<=', '!='])} {expr(names)}"


counter = [0]


def fresh(p):
    counter[0] += 1
    return f"{p}{counter[0]}"


def head(names, ind, nl):
    k = rng.choice(["nondet", "def", "val", "pure val"])
    if k == "nondet":
        n = fresh("v")
        src = rng.choice(["0.to(2)", "Set(1, 2)", f"Set({expr(names)}, 3)", "0.to(x)"])
        return f"nondet {n} = {src}.oneOf()", names + [n]
    if k == "def":
        n = fresh("g")
        if rng.random() < 0.5:
            return f"def {n}(a) = a + {expr(names)}", names + [f"{n}(1)"]
        return f"def {n} = {expr(names)}", names + [n]
    if k == "val":
        n = fresh("w")
        return f"val {n} = {expr(names)}", names + [n]
    n = fresh("p")
    return f"pure val {n} = {rng.choice(['1', '2', '3 + 4', 'Set(1).size()'])}", names + [n]


def action(names, ind, d=0):
    r = rng.random()
    nl = "\n" + "  " * (ind + 1)
    if d < 2 and r < 0.3:
        return block(names, ind, d + 1)
    if r < 0.5:
        sep = "," + nl if rng.random() < 0.6 else ", "
        kw = rng.choice(["all", "any"])
        parts = [f"x' = {expr(names)}", f"y' = {expr(names)}"]
        if kw == "any":
            parts = [f"all {{ x' = {expr(names)}, y' = y }}", f"all {{ x' = x, y' = {expr(names)} }}"]
        return f"{kw} {{{nl if sep != ', ' else ' '}{sep.join(parts)}{chr(10) + '  ' * ind if sep != ', ' else ' '}}}"
    if r < 0.6:
        return f"if ({cond(names)}) {action(names, ind, d+1)} else {action(names, ind, d+1)}"
    if r < 0.7:
        return f"and {{ {cond(names)}, x' = {expr(names)}, y' = y }}"
    return f"all {{ x' = {expr(names)}, y' = y }}"


def block(names, ind, d=0):
    multi = rng.random() < 0.7
    sep = "\n" + "  " * (ind + 1) if multi else " "
    lines = []
    for _ in range(rng.randint(1, 3)):
        h, names = head(names, ind, multi)
        lines.append(h)
    lines.append(action(names, ind + 1, d))
    end = "\n" + "  " * ind if multi else " "
    return "{" + sep + sep.join(lines) + end + "}"


def module():
    counter[0] = 0
    body = block([], 1)
    return f"""module G {{
  var x: int
  var y: int
  pure def inc(n) = n + 1
  action init = all {{ x' = 0, y' = 0 }}
  action step = {body}
  val ok = x >= 0
}}
"""


TOK = re.compile(r"///[^\n]*|==|!=|<=|>=|=|'|[(){},+\-*<>.]|[A-Za-z_][A-Za-z0-9_]*|[0-9]+|\n")


def mutate(text):
    start = text.index("action step")
    stop = text.index("val ok")
    toks = [m for m in TOK.finditer(text) if start <= m.start() < stop]
    kind = rng.choice(["dangle", "drop", "extra", "doc", "eq", "prime"])
    if kind == "dangle":
        cands = [m for m in toks if m.group() in ("\n", "}", ",", ")")]
        m = rng.choice(cands)
        return text[:m.start()] + " " + rng.choice(["+", "*", "and", "=="]) + text[m.start():], kind
    if kind == "drop":
        cands = [m for m in toks if m.group() in ("}", ")", ",", "{", "(")]
        m = rng.choice(cands)
        return text[:m.start()] + text[m.end():], kind
    if kind == "extra":
        m = rng.choice(toks)
        c = rng.choice(["}", ")", ","])
        return text[:m.start()] + c + " " + text[m.start():], kind
    if kind == "doc":
        cands = [m for m in toks if m.group() == "\n"]
        if not cands:
            m = rng.choice(toks)
            return text[:m.start()] + "\n/// stray\n" + text[m.start():], kind
        m = rng.choice(cands)
        return text[:m.end()] + "    /// stray\n" + text[m.end():], kind
    if kind == "eq":
        cands = [m for m in toks if m.group() == "=="]
        if not cands:
            cands = [m for m in toks if m.group() == "="]
            m = rng.choice(cands)
            return text[:m.start()] + "==" + text[m.end():], kind
        m = rng.choice(cands)
        return text[:m.start()] + "=" + text[m.end():], kind
    cands = [m for m in toks if m.group() == "'"]
    m = rng.choice(cands)
    return text[:m.start()] + text[m.end():], kind


for i in range(count):
    base = module()
    with open(f"{out}/b{i:05}.qnt", "w") as written:
        written.write(base)
    text, kind = mutate(base)
    with open(f"{out}/m{i:05}_{kind}.qnt", "w") as written:
        written.write(text)
