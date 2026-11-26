import random, sys

def model(seed):
    rnd = random.Random(seed)
    name = f"G{seed}"
    lim = rnd.randint(2, 4)
    keys = rnd.choice(["Set(0, 1)", "Set(0, 1, 2)", "Set(1, 2)"])
    items = rnd.choice(["Set(1, 2)", "Set(0, 2, 3)", "Set(1, 2, 3)"])
    def ie(depth=0):
        opts = [lambda: "n", lambda: "r.a", lambda: str(rnd.randint(0, 2)), lambda: "s.size()",
                lambda: "m.keys().size()", lambda: "l.length()"]
        if depth < 2:
            opts += [lambda: f"({ie(depth+1)} + {ie(depth+1)})",
                     lambda: f"({ie(depth+1)} - {ie(depth+1)})",
                     lambda: f"(if ({be(depth+1)}) {ie(depth+1)} else {ie(depth+1)})",
                     lambda: "s.fold(0, (acc, e) => acc + e)",
                     lambda: "n",
                     lambda: "l.foldl(0, (acc, e) => acc + e)"]
        return rnd.choice(opts)()
    def be(depth=0):
        opts = [lambda: "b", lambda: "r.f", lambda: f"{ie(depth+1)} <= {ie(depth+1)}",
                lambda: f"{ie(depth+1)} == {ie(depth+1)}",
                lambda: f"s.contains({rnd.randint(0,3)})",
                lambda: f"m.keys().contains({rnd.randint(0,2)})",
                lambda: f"s.exists(e => e > {rnd.randint(0,3)})",
                lambda: f"s.forall(e => e != n)",
                lambda: f"m.keys().forall(k => m.get(k) <= {rnd.randint(0,3)})",
                lambda: f"l.indices().exists(i => l[i] == {rnd.randint(0,3)})"]
        if depth < 2:
            opts += [lambda: f"({be(depth+1)} and {be(depth+1)})",
                     lambda: f"({be(depth+1)} or {be(depth+1)})",
                     lambda: f"not({be(depth+1)})",
                     lambda: f"({be(depth+1)} implies {be(depth+1)})",
                     lambda: f"({be(depth+1)} iff {be(depth+1)})"]
        return rnd.choice(opts)()
    def same(vs, changed):
        return [f"{v}' = {v}" for v in vs if v not in changed]
    VARS = ["n", "b", "s", "m", "r", "l"]
    upd = {
        "n": lambda v: f"n' = (n + {v}) % {lim + 1}",
        "b": lambda v: rnd.choice(["b' = not(b)", f"b' = {be(2)}", "b' = true"]),
        "s": lambda v: rnd.choice([f"s' = s.union(Set({v}))", f"s' = s.exclude(Set({v}))", f"s' = if (s.size() < 2) s.union(Set({v})) else Set()"]),
        "m": lambda v: rnd.choice([f"m' = m.put({v} % 3, n % 3)", f"m' = m.keys().exclude(Set({v} % 3)).mapBy(k => m.get(k))", f"m' = if (m.keys().contains({v} % 3)) m.setBy({v} % 3, x => (x + 1) % 3) else m.put({v} % 3, 0)"]),
        "r": lambda v: rnd.choice([f"r' = r.with(\"a\", ({v} + r.a) % 3)", f"r' = {{ a: n % 3, f: not(r.f) }}", f"r' = r.with(\"f\", {be(2)})"]),
        "l": lambda v: rnd.choice([f"l' = if (l.length() < 2) l.append({v} % 4) else l.slice(1, l.length())", f"l' = if (l.length() > 0) l.tail() else [{v} % 4]", f"l' = if (l.length() > 0) l.replaceAt(0, {v} % 4) else l"]),
    }
    actions = []
    count = rnd.randint(2, 4)
    for k in range(count):
        changed = rnd.sample(VARS, rnd.randint(1, 3))
        use_nd = rnd.random() < 0.6
        v = "v" if use_nd else str(rnd.randint(0, 3))
        parts = []
        if rnd.random() < 0.7:
            parts.append(be(1))
        parts += [upd[c](v) for c in changed] + same(VARS, changed)
        body = "all {\n      " + ",\n      ".join(parts) + ",\n    }"
        if use_nd:
            body = f"nondet v = {items}.oneOf()\n    {body}"
        rnd.random()
        actions.append((f"act{k}", body))
    init_n = rnd.choice(["n' = 0", f"nondet i = 0.to({lim}).oneOf() n' = i"])
    if init_n.startswith("nondet"):
        init = f"action init = {{\n    nondet i = 0.to({lim}).oneOf()\n    all {{ n' = i, b' = false, s' = Set(), m' = Map(), r' = {{ a: 0, f: true }}, l' = [] }}\n  }}"
    else:
        init = "action init = all { n' = 0, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }"
    invs = [be(0) for _ in range(rnd.randint(2, 3))]
    lines = [f"module {name} {{",
             "  var n: int", "  var b: bool", "  var s: Set[int]", "  var m: int -> int",
             "  var r: { a: int, f: bool }", "  var l: List[int]", f"  {init}"]
    for a, body in actions:
        lines.append(f"  action {a} = {body}")
    if rnd.random() < 0.5:
        half = max(1, len(actions) // 2)
        lines.append(f"  action left = any {{ {', '.join(a for a, _ in actions[:half])} }}")
        rest = ["left"] + [a for a, _ in actions[half:]]
        lines.append(f"  action step = any {{ {', '.join(rest)} }}")
    else:
        lines.append(f"  action step = any {{ {', '.join(a for a, _ in actions)} }}")
    for i, inv in enumerate(invs):
        lines.append(f"  val inv{i} = {inv}")
    lines.append("}")
    return name, "\n".join(lines) + "\n", [f"inv{i}" for i in range(len(invs))]

if __name__ == "__main__":
    name, text, invs = model(int(sys.argv[1]))
    print(text)
