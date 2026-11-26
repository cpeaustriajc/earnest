import random, sys


def model(seed):
    rnd = random.Random(seed)
    name = f"Q{seed}"
    cap = rnd.randint(1, 2)
    alias_helpers = False

    def ie(d=0):
        opts = [lambda: "cnt.get(1)", lambda: str(rnd.randint(0, 2)), lambda: "got.size()",
                lambda: "BUYERS.fold(0, (s, b) => s + cnt.get(b))", lambda: "LIMIT",
                lambda: "bumpTo(cnt.get(1), LIMIT)"]
        if d < 2:
            opts += [lambda: f"({ie(d + 1)} + {ie(d + 1)})",
                     lambda: f"(if ({be(d + 1)}) {ie(d + 1)} else {ie(d + 1)})"]
        return rnd.choice(opts)()

    def be(d=0):
        opts = [lambda: "flag", lambda: "isBuy(ev)", lambda: f"{ie(d + 1)} <= {ie(d + 1)}",
                lambda: f"{ie(d + 1)} == {ie(d + 1)}",
                lambda: f"BUYERS.forall(b => cnt.get(b) <= {rnd.randint(0, cap)})",
                lambda: f"ITEMS.exists(i => (1, i).in(got))",
                lambda: "match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }",
                lambda: f"got.forall(p => p._1 == {rnd.randint(1, 2)})"]
        if d < 2:
            opts += [lambda: f"all {{ {be(d + 1)}, {be(d + 1)} }}",
                     lambda: f"any {{ {be(d + 1)}, {be(d + 1)} }}",
                     lambda: f"not({be(d + 1)})",
                     lambda: f"({be(d + 1)} implies {be(d + 1)})"]
        return rnd.choice(opts)()

    back = rnd.choice([
        "all { cnt.get(b) > 0, cnt' = cnt.set(b, cnt.get(b) - 1), flag' = flag, ev' = Idle, got' = got }",
        "all { flag' = true, ev' = Idle, cnt' = cnt, got' = got }",
    ])
    other = rnd.choice([
        "all { flag' = flag, ev' = ev, cnt' = cnt, got' = got }",
        "all { not(flag), flag' = true, ev' = ev, cnt' = cnt, got' = got }",
    ])
    react = f"""action react = match ev {{
    | Buy(b) => all {{ {be(2)}, flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }}
    | Back(b) => {back}
    | _ => {other}
  }}"""
    branches = []
    via_pick = rnd.random() < 0.6
    if via_pick:
        branches.append("nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1)")
    if not via_pick or rnd.random() < 0.5:
        branches.append("nondet b = BUYERS.oneOf() buy(b)")
    if rnd.random() < 0.4:
        branches.append("buy(1)")
    branches.append("nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2)")
    branches.append("react")
    if rnd.random() < 0.5:
        branches.append("all { flag' = not(flag), cnt' = cnt, got' = got, ev' = ev }")
    rnd.shuffle(branches)
    step = "any {\n    " + ",\n    ".join(branches) + ",\n  }"
    overs = ["cnt", "(cnt, ev)", "{ c: cnt }", "[cnt]", "(cnt, (ev, got))", "ALLV"]
    fair_each = f"{rnd.choice(['BUYERS', 'WATCH'])}.forall(b => weakFair(buy(b), {rnd.choice(overs)}))"
    fair_one = f"weakFair(buy(1), {rnd.choice(overs)})"
    fair_take = f"BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), {rnd.choice(['got', '(got, ev)', 'ALLV'])})))"
    fair_react = "weakFair(react, ALLV)"
    fairs = [fair_each, fair_one, fair_take, fair_react, f"and {{ {fair_each}, {fair_react} }}",
             f"and {{ {fair_take}, {fair_one} }}", "weakFair(step, ALLV)"]
    goal = lambda: rnd.choice(["BUYERS.forall(b => cnt.get(b) == LIMIT)", "cnt.get(1) >= 1",
                               "got.size() == BUYERS.size() * ITEMS.size()", "flag", be(1)])
    temps = [
        f"  temporal t0 = {rnd.choice(fairs)}.implies(eventually({goal()}))",
        f"  temporal t1 = and {{ always({rnd.choice([be(1), 'BUYERS.forall(b => cnt.get(b) <= LIMIT)'])}), {rnd.choice(fairs)}.implies(eventually({goal()})) }}",
        f"  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))",
        f"  temporal t3 = {rnd.choice(fairs)}.implies(always(eventually({goal()})))",
        f"  temporal t4 = (not(flag)).implies({rnd.choice(fairs)}.implies(leadsTo({be(1)}, {goal()})))",
        f"  temporal t5 = WATCH.forall(w => always((w, \"a\").in(got) implies cnt.get(w) <= LIMIT))",
    ]
    invs = [f"  val inv{i} = {be()}" for i in range(3)]
    runs = []
    for i in range(3):
        first = rnd.choice(["init", f"startWith({rnd.randint(0, 1)})"])
        chain = first
        for _ in range(rnd.randint(1, 3)):
            chain += "." + rnd.choice(["then(step)", "then(buy(1))", "then(react)",
                                       f"then({rnd.randint(1, 2)}.reps(_ => step))", "then(buyAndReact(1))"])
        end = f".expect({be(1)})" if rnd.random() < 0.8 else ".fail()"
        runs.append(f"  run r{i}Test = {chain}{end}")
    runs.append(f"  run pickTest = {{ nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect({be(1)}) }}")
    setups = [("A", "Set(1, 2)", 'Set("a")', "Set(1, 2)"), ("B", "Set(1)", 'Set("a", "b c")', "Set(1)"), ("C", "Set(1, 2)", 'Set("a")', "Set()")]
    setup_text = "\n".join(
        f'/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5\nmodule {name}{s} {{\n  import {name}spec(BUYERS = {b}, ITEMS = {it}, WATCH = {w}, CAP = {cap}).*\n}}' for s, b, it, w in setups)
    return f"""module {name}types {{
  type Ev = Buy(int) | Back(int) | Idle
}}
module {name}helpers {{
  import {name}types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e {{ | Buy(_) => true | _ => false }}
  pure val Zero = 0
}}
module {name}spec {{
  import {name}types.*
  import {name}helpers.*
  const BUYERS: Set[int]
  const ITEMS: Set[str]
  const WATCH: Set[int]
  const CAP: int
  assume watched = WATCH.subseteq(BUYERS)
  assume capPos = CAP >= 1
  assume few = BUYERS.size() <= 3 and ITEMS.size() <= 2
  pure val LIMIT = CAP + Zero
  var cnt: int -> int
  var got: Set[(int, str)]
  var ev: Ev
  var flag: bool
  val ALLV = (cnt, got, ev, flag)
  val everything = true
  action init = all {{ cnt' = BUYERS.mapBy(b => 0), got' = Set(), ev' = Idle, flag' = false }}
  action buy(b: int): bool = all {{ cnt.get(b) < LIMIT, cnt' = cnt.set(b, bumpTo(cnt.get(b), LIMIT)), ev' = Buy(b), got' = got, flag' = flag }}
  action take(b: int, i: str): bool = all {{ not((b, i).in(got)), got' = got.union(Set((b, i))), ev' = Back(b), cnt' = cnt, flag' = flag }}
  {react}
  action step = {step}
{chr(10).join(invs)}
{chr(10).join(temps)}
  action startWith(k: int): bool = all {{ cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }}
  action buyAndReact(b: int): bool = buy(b).then(react)
{chr(10).join(runs)}
}}
{setup_text}
"""


if __name__ == "__main__":
    print(model(int(sys.argv[1])), end="")
