import random, sys

CMDS = ["Inc(1)", "Inc(2)", 'Put(("a", 1))', 'Put(("b", 2))', "Clear"]


def model(seed, plain=False):
    rnd = random.Random(seed)
    name = f"H{seed}"
    lim = rnd.randint(2, 3)
    keys = rnd.choice(['Set("a")', 'Set("a", "b")'])
    cmds = "Set(" + ", ".join(rnd.sample(CMDS, rnd.randint(2, 3))) + ")"
    whole = rnd.random() < 0.5

    def ie(d=0):
        opts = [lambda: "n", lambda: str(rnd.randint(0, 2)), lambda: "q.length()",
                lambda: "m.keys().size()", lambda: "LIM",
                lambda: "useOp(v => v + 1, n)", lambda: "pair._1"]
        if d < 2:
            opts += [lambda: f"({ie(d + 1)} + {ie(d + 1)})",
                     lambda: f"(if ({be(d + 1)}) {ie(d + 1)} else {ie(d + 1)})",
                     lambda: "m.keys().fold(0, (acc, k) => acc + m.get(k))",
                     lambda: "q.foldl(0, (acc, c) => acc + weight(c))",
                     lambda: "both((a, b) => a + b, n, pair._1)"]
        return rnd.choice(opts)()

    def be(d=0):
        opts = [lambda: "flag", lambda: f"{ie(d + 1)} <= {ie(d + 1)}",
                lambda: f"{ie(d + 1)} == {ie(d + 1)}",
                lambda: "last == Clear", lambda: f"isInc(last)",
                lambda: f'm.keys().contains("{rnd.choice("ab")}")',
                lambda: f"q.select(c => isInc(c)).length() <= {rnd.randint(0, 2)}",
                lambda: f"KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= {rnd.randint(0, 3)})",
                lambda: f'pair._2 == "{rnd.choice(["x", "y"])}"']
        if d < 2:
            opts += [lambda: f"({be(d + 1)} and {be(d + 1)})",
                     lambda: f"({be(d + 1)} or {be(d + 1)})",
                     lambda: f"not({be(d + 1)})",
                     lambda: f"({be(d + 1)} implies {be(d + 1)})"]
        return rnd.choice(opts)()

    def plain_ie(d=0):
        opts = [lambda: "n", lambda: str(rnd.randint(0, 2)), lambda: "q.length()",
                lambda: "m.keys().size()", lambda: "LIM", lambda: "pair._1"]
        if d < 2:
            opts += [lambda: f"({plain_ie(d + 1)} + {plain_ie(d + 1)})"]
        return rnd.choice(opts)()

    def plain_be(d=0):
        opts = [lambda: "flag", lambda: f"{plain_ie(d + 1)} <= {plain_ie(d + 1)}",
                lambda: f"{plain_ie(d + 1)} == {plain_ie(d + 1)}", lambda: "last == Clear",
                lambda: f'm.keys().contains("{rnd.choice("ab")}")',
                lambda: f'pair._2 == "{rnd.choice(["x", "y"])}"']
        if d < 2:
            opts += [lambda: f"({plain_be(d + 1)} and {plain_be(d + 1)})",
                     lambda: f"({plain_be(d + 1)} or {plain_be(d + 1)})",
                     lambda: f"not({plain_be(d + 1)})"]
        return rnd.choice(opts)()

    VARS = ["n", "m", "q", "last", "flag", "pair"]

    def keep(*changed):
        return [f"{v}' = {v}" for v in VARS if v not in changed]

    def act(parts):
        return "all {\n      " + ",\n      ".join(parts) + ",\n    }"

    pool = {
        "enqueue": f"""all {{
    q.length() < 2,
    nondet nc = {cmds}.oneOf()
    {act([f"q' = q.append(nc)"] + keep("q"))}
  }}""",
        "process": """all {
    q.length() > 0,
    match q.head() {
      | Inc(k) => all { n' = useOp(v => (v + k) % (LIM + 1), n), q' = q.tail(), last' = q.head(), m' = m, flag' = flag, pair' = pair }
      | Put(p) => all { KEYS.contains(p._1), m' = m.put(p._1, p._2), q' = q.tail(), last' = q.head(), n' = n, flag' = flag, pair' = pair }
      | _ => all { m' = Map(), q' = q.tail(), last' = Clear, n' = n, flag' = flag, pair' = pair }
    }
  }""",
        "spend": f"""all {{
    nondet nk = m.keys().filter(k => m.get(k) > 0).oneOf()
    nondet amt = 1.to(m.get(nk)).oneOf()
    {act(["m' = m.set(nk, m.get(nk) - amt)", f"pair' = (amt, \"{rnd.choice(['x', 'y'])}\")"] + keep("m", "pair"))}
  }}""",
        "lower": f"""all {{
    nondet nv = 0.to(n).oneOf()
    {act([f"{be(1)}", "n' = nv"] + keep("n"))}
  }}""",
        "toggle": f"""{act([be(1), "flag' = not(flag)", f"pair' = (pair._1, if (flag) \"x\" else \"y\")"] + keep("flag", "pair"))}""",
        "drop": f"""all {{
    nondet ni = q.indices().oneOf()
    {act(["q' = q.indices().exclude(Set(ni)).fold(List(), (l, i) => l.append(q[i]))", "last' = q[ni]"] + keep("q", "last"))}
  }}""",
    }
    names = ["enqueue", "process"] + rnd.sample(["spend", "lower", "toggle", "drop"], rnd.randint(1, 3))
    acts = "\n".join(f"  action {a} = {pool[a]}" for a in names)
    step = "any { " + ", ".join(names) + " }"
    invs = [f"  val inv{i} = {be()}" for i in range(3)]
    fair_target = rnd.choice(["step"] + names)
    goals = [plain_be(1) if plain else be(1) for _ in range(4)]
    temps = [
        f"  temporal t0 = eventually({goals[0]})",
        f"  temporal t1 = weakFair(step, vars) implies eventually({goals[1]})",
        f"  temporal t2 = weakFair({fair_target}, vars) implies always(eventually({goals[2]}))",
        f"  temporal t3 = weakFair(step, vars) implies always({goals[3]} implies eventually(q.length() == 0))",
        f"  temporal t4 = not(eventually({goals[0]}))",
    ]
    runs = []
    for i in range(3):
        chain = "init"
        for _ in range(rnd.randint(1, 3)):
            chain += f".then({rnd.choice(names + ['step'])})"
        end = rnd.choice([f".expect({be(1)})", ".fail()" if rnd.random() < 0.3 else f".expect({be(1)})"])
        runs.append(f"  run r{i}Test = {chain}{end}")
    runs.append(f"  run repsTest = init.then({rnd.randint(1, 3)}.reps(_ => step)).expect({be(1)})")
    spec = f"""module Spec{seed} {{
  type Cmd = Inc(int) | Put((str, int)) | Clear
  const LIM: int
  const KEYS: Set[str]
  const START: (int, str)
  var n: int
  var m: str -> int
  var q: List[Cmd]
  var last: Cmd
  var flag: bool
  var pair: (int, str)
  val vars = (n, m, q, last, flag, pair)
  pure def useOp(f: int => int, v: int): int = f(v)
  pure def both(g: (int, int) => int, a: int, b: int): int = g(b, a)
  pure def isInc(c: Cmd): bool = match c {{ | Inc(_) => true | _ => false }}
  pure def weight(c: Cmd): int = match c {{ | Inc(k) => k | Put(p) => p._2 | Clear => 0 }}
  action init = all {{
    nondet f0 = Bool.oneOf()
    all {{ n' = 0, m' = Map(), q' = [], last' = Clear, flag' = f0, pair' = START }}
  }}
{acts}
  action step = {step}
{chr(10).join(invs)}
{chr(10).join(temps)}
{chr(10).join(runs)}
}}
"""
    start = f'({rnd.randint(0, 2)}, "{rnd.choice(["x", "y"])}")'
    if whole:
        setup = f"""module {name} {{
  import Spec{seed}(LIM = {lim}, KEYS = {keys}, START = {start}).*
}}
"""
        prefix = ""
    else:
        setup = f"""module {name} {{
  pure val Lim = {lim}
  import Spec{seed}(LIM = Lim, KEYS = {keys}, START = {start}) as C
}}
"""
        prefix = "C::"
    return spec + "\n" + setup, name, prefix


if __name__ == "__main__":
    text, name, prefix = model(int(sys.argv[1]), plain=sys.argv[2:] == ["plain"])
    print(text, end="")
