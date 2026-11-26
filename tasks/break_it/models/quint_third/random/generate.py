import random, sys


def model(seed):
    rnd = random.Random(seed)
    name = f"R{seed}"
    buyers = rnd.choice(["Set(1, 2)", "Set(1, 2, 3)"])
    cap = rnd.randint(1, 2)
    cycle = rnd.random() < 0.5
    sometimes = rnd.random() < 0.5
    direct = rnd.random() < 0.5
    VARS = ["cnt", "last", "flag", "tok"]

    def ie(d=0):
        opts = [lambda: "last", lambda: str(rnd.randint(0, 2)), lambda: "tok._1",
                lambda: f"cnt.get({rnd.randint(1, 2)})",
                lambda: "BUYERS.fold(0, (s, b) => s + cnt.get(b))", lambda: "apply(v => v + 1, last)"]
        if d < 2:
            opts += [lambda: f"({ie(d + 1)} + {ie(d + 1)})",
                     lambda: f"(if ({be(d + 1)}) {ie(d + 1)} else {ie(d + 1)})"]
        return rnd.choice(opts)()

    def be(d=0):
        opts = [lambda: "flag", lambda: "tok._2", lambda: f"{ie(d + 1)} <= {ie(d + 1)}",
                lambda: f"{ie(d + 1)} == {ie(d + 1)}",
                lambda: f"BUYERS.forall(b => cnt.get(b) <= {rnd.randint(0, cap)})",
                lambda: f"BUYERS.exists(b => cnt.get(b) == {rnd.randint(0, cap)})"]
        if d < 2:
            opts += [lambda: f"all {{ {be(d + 1)}, {be(d + 1)} }}",
                     lambda: f"any {{ {be(d + 1)}, {be(d + 1)} }}",
                     lambda: f"({be(d + 1)} and {be(d + 1)})",
                     lambda: f"not({be(d + 1)})",
                     lambda: f"({be(d + 1)} implies {be(d + 1)})"]
        return rnd.choice(opts)()

    def keep(*changed):
        return [f"{v}' = {v}" for v in VARS if v not in changed]

    last_change = "if (cnt.get(b) == 0) b else last" if sometimes else "b"
    buy = f"""action buy(b: int): bool = all {{
    cnt.get(b) < CAP,
    cnt' = cnt.set(b, cnt.get(b) + 1),
    last' = {last_change},
  }}"""
    reset = """action reset(b: int): bool = all {
    cnt.get(b) == CAP,
    cnt' = cnt.set(b, 0),
    last' = 0,
  }"""
    guarded = f"""action guarded(a: bool): bool = all {{ {be(1)}, a }}"""
    branches = [f"nondet b = BUYERS.oneOf() all {{ buy(b), flag' = flag, tok' = tok }}"]
    if direct:
        branches.append("all { buy(1), flag' = flag, tok' = tok }")
    if cycle:
        branches.append(f"nondet b = BUYERS.oneOf() all {{ reset(b), flag' = flag, tok' = tok }}")
    branches.append("all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok }")
    branches.append(f"guarded(all {{ tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }})")
    if rnd.random() < 0.5:
        branches.append("all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag }")
    step = "any {\n    " + ",\n    ".join(branches) + ",\n  }"
    overs = ["cnt", "last", "(cnt, last)", "{ c: cnt }", "[last]", "(cnt, (last))"]
    over = rnd.choice(overs)
    c = rnd.randint(1, 2)
    fair_each = f"BUYERS.forall(b => weakFair(buy(b), {over}))"
    fair_one = f"weakFair(buy({c}), {rnd.choice(overs)})"
    fair_step = "weakFair(step, (cnt, last, flag, tok))"
    goal = lambda: rnd.choice([f"BUYERS.forall(b => cnt.get(b) == CAP)", f"cnt.get({c}) == CAP",
                               f"last == {rnd.randint(0, 2)}", be(1)])
    fairs = [fair_each, fair_one, fair_step, fair_each + f".and({fair_step})"]
    true_always = lambda: rnd.choice([be(1), "BUYERS.forall(b => cnt.get(b) <= CAP)", "tok._1 < 3"])
    bare = lambda: rnd.choice([be(1), "last == 0", "not(flag)"])
    temps = [
        f"  temporal t0 = {rnd.choice(fairs)}.implies(eventually({goal()}))",
        f"  temporal t1 = and {{ always({true_always()}), {rnd.choice(fairs)}.implies(eventually({goal()})), {bare()} }}",
        f"  temporal t2 = {rnd.choice(fairs)}.implies(leadsTo({be(1)}, {goal()}))",
        f"  temporal t3 = BUYERS.forall(b => weakFair(buy(b), {over}).implies(eventually(cnt.get(b) >= {rnd.randint(1, cap)})))",
        f"  temporal t4 = and {{ {rnd.choice(fairs)}.implies(always(eventually({goal()}))), and {{ always({true_always()}), {fair_one}.implies(eventually({goal()})) }} }}",
    ]
    invs = [f"  val inv{i} = {be()}" for i in range(3)]
    helpers = f"""  action start(k: int): bool = all {{ cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }}
  action buyTwice(b: int): bool = all {{ buy(b), flag' = flag, tok' = tok }}.then(all {{ buy(b), flag' = flag, tok' = tok }})"""
    runs = []
    for i in range(4):
        first = rnd.choice(["init", f"start({rnd.randint(0, 1)})", "all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }"])
        chain = first
        for _ in range(rnd.randint(1, 3)):
            chain += "." + rnd.choice([f"then(step)", f"then(buyTwice({rnd.randint(1, 2)}))",
                                       f"then({rnd.randint(1, 2)}.reps(_ => step))", f"then(guarded(all {{ tok' = (2, true), cnt' = cnt, last' = last, flag' = flag }}))"])
        end = f".expect({rnd.choice([be(1), 'BUYERS.forall(b => cnt.get(b) <= CAP)', 'tok._1 <= 2'])})" if rnd.random() < 0.8 else ".fail()"
        runs.append(f"  run r{i}Test = {chain}{end}")
    runs.append(f"  run nondetTest = {{ nondet k = Set(0, 1).oneOf() start(k).then(step).expect({be(1)}) }}")
    return f"""module {name} {{
  pure val BUYERS = {buyers}
  pure val CAP = {cap}
  var cnt: int -> int
  var last: int
  var flag: bool
  var tok: (int, bool)
  pure def apply(f: int => int, v: int): int = f(v)
  action init = all {{ cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = false, tok' = (0, false) }}
  {buy}
  {reset}
  {guarded}
  action step = {step}
{chr(10).join(invs)}
{chr(10).join(temps)}
{helpers}
{chr(10).join(runs)}
}}
"""


if __name__ == "__main__":
    print(model(int(sys.argv[1])), end="")
