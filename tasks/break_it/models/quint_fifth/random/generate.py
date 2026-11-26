import random, sys

REST = {"cnt": "cnt' = cnt", "open": "open' = open", "done": "done' = done", "q": "q' = q", "ph": "ph' = ph"}


def model(seed):
    rnd = random.Random(seed)
    name = f"G{seed}"
    cap = rnd.randint(1, 2)
    kind = rnd.randint(0, 4)
    buy_keeps_q = rnd.random() < 0.3
    buy_vars = ["cnt"] + (["q"] if buy_keeps_q else [])
    keepq = ", q' = q" if buy_keeps_q else ""
    inc = "cnt' = cnt.set(b, cnt.get(b) + 1)"
    if kind == 0:
        buy = f"all {{ cnt.get(b) < CAP, {inc}{keepq} }}"
    elif kind == 1:
        buy = f"all {{ nondet k = 1.to(CAP - cnt.get(b)).oneOf() cnt' = cnt.set(b, cnt.get(b) + k){keepq} }}"
    elif kind == 2:
        buy = f"all {{ if (cnt.get(b) < CAP) {inc} else cnt' = cnt{keepq} }}"
    elif kind == 3:
        buy = f"all {{ match ph {{ | Run => all {{ cnt.get(b) < CAP, {inc} }} | Halt => cnt' = cnt }}{keepq} }}"
    else:
        buy = f"all {{ {rnd.choice(['open', 'not(done)', 'q.length() < 2'])}, bump(b){keepq} }}"

    def rest(assigned):
        return ", ".join(v for k, v in REST.items() if k not in assigned)

    def guard():
        return rnd.choice(["open", "not(open)", "q.length() > 0", "cnt.get(1) >= 1", "not(done)",
                           "B.exists(b => cnt.get(b) < CAP)"])

    def wrap(br, other):
        r = rnd.random()
        if r < 0.3:
            return f"all {{ {guard()}, {br} }}"
        if r < 0.45:
            return f"if ({guard()}) {br} else {other}"
        if r < 0.55:
            return f"match ph {{ | Run => {br} | Halt => {other} }}"
        return br

    toggle_g = rnd.choice(["", "not(done), ", "q.length() == 0, "])
    actions = {
        "toggle": (f"all {{ {toggle_g}open' = not(open) }}", ["open"]),
        "finish": (f"all {{ {rnd.choice(['B.forall(b => cnt.get(b) >= 1)', 'open', 'cnt.get(1) == CAP'])}, done' = true, ph' = Halt }}", ["done", "ph"]),
        "push": ("all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }", ["q"]),
        "pop": ("all { q.length() > 0, q' = q.tail() }", ["q"]),
    }
    branches = []
    buy_each = rnd.random() < 0.75
    buy_one = not buy_each or rnd.random() < 0.4
    idle = f"all {{ {rest([])} }}"
    if buy_each:
        if rnd.random() < 0.5:
            branches.append(wrap(f"nondet b = B.oneOf() all {{ buy(b), {rest(buy_vars)} }}", idle))
        else:
            branches.append(wrap(f"nondet p = tuples(B, Set(0, 1)).oneOf() all {{ buy(p._1), {rest(buy_vars)} }}", idle))
    if buy_one:
        branches.append(wrap(f"all {{ buy(1), {rest(buy_vars)} }}", idle))
    used = []
    for a, (_, vs) in actions.items():
        if a in ("toggle", "finish") or rnd.random() < 0.5:
            used.append(a)
            branches.append(wrap(f"all {{ {a}, {rest(vs)} }}", idle))
    if rnd.random() < 0.3:
        branches.append(idle)
    rnd.shuffle(branches)
    inner = "any {\n      " + ",\n      ".join(branches) + ",\n    }"
    outer = rnd.randint(0, 3)
    if outer == 0:
        step = f"all {{ not(done), {inner} }}"
    elif outer == 1:
        step = f"match ph {{\n    | Run => {inner}\n    | Halt => {idle}\n  }}"
    elif outer == 2:
        step = f"if (open or not(done)) {inner} else {idle}"
    else:
        step = inner

    fairs = []
    if buy_each:
        fairs.append(f"B.forall(b => weakFair(buy(b), {rnd.choice(['cnt', 'cnt'] + (['(cnt, q)', '(q, cnt)'] if buy_keeps_q else []))}))")
    if buy_one or buy_each:
        fairs.append(f"weakFair(buy(1), {rnd.choice(['cnt'] + (['(cnt, q)'] if buy_keeps_q else []))})")
    fairs.append("weakFair(toggle, open)")
    fairs.append(f"weakFair(finish, {rnd.choice(['done', '(done, ph)', 'ph'])})")
    if "pop" in used:
        fairs.append("weakFair(pop, q)")
    if "push" in used:
        fairs.append("weakFair(push, q)")
    fairs.append(f"weakFair(step, {rnd.choice(['ALLV', 'cnt', '(cnt, done)', 'q'])})")

    def fair():
        n = rnd.choice([1, 1, 2, 3])
        picked = rnd.sample(fairs, min(n, len(fairs)))
        if len(picked) == 1:
            return picked[0]
        return "and { " + ", ".join(picked) + " }"

    def goal():
        return rnd.choice(["cnt.get(1) >= 1", "B.forall(b => cnt.get(b) == CAP)", "done", "q.length() == 0",
                           "open", "not(open)", "cnt.get(2) >= 1", "ph == Halt", "q.length() > 0"])

    temps = [
        f"  temporal t0 = {fair()}.implies(eventually({goal()}))",
        f"  temporal t1 = {fair()}.implies(eventually({goal()}))",
        f"  temporal t2 = {fair()}.implies(always(eventually({goal()})))",
        f"  temporal t3 = {fair()}.implies(leadsTo({goal()}, {goal()}))",
        f"  temporal t4 = and {{ always(B.forall(b => cnt.get(b) <= CAP)), {fair()}.implies(eventually({goal()})) }}",
    ]
    invs = [f"  val inv0 = {rnd.choice(['not(done) or B.exists(b => cnt.get(b) >= 1) or open', 'q.length() <= 1', 'cnt.get(1) <= 1', 'not(done and open)'])}",
            f"  val inv1 = {rnd.choice(['B.forall(b => cnt.get(b) <= CAP)', 'not(ph == Halt) or done', 'q.length() < 2 or open'])}"]
    run_steps = ["then(step)", "then(all { buy(1), " + rest(buy_vars) + " })", "then(all { toggle, " + rest(["open"]) + " })"]
    runs = []
    for i in range(2):
        chain = "init"
        for _ in range(rnd.randint(1, 3)):
            chain += "." + rnd.choice(run_steps)
        runs.append(f"  run r{i}Test = {chain}.expect({goal()} or {goal()})")
    acts = "\n".join(f"  action {a}: bool = {body}" for a, (body, _) in actions.items())
    return f"""module {name} {{
  type Phase = Run | Halt
  pure val B = Set(1, 2)
  pure val CAP = {cap}
  var cnt: int -> int
  var open: bool
  var done: bool
  var q: List[int]
  var ph: Phase
  val ALLV = (cnt, open, done, q, ph)
  action init = all {{ cnt' = B.mapBy(b => 0), open' = false, done' = false, q' = [], ph' = Run }}
  action bump(b: int): bool = all {{ cnt.get(b) < CAP, {inc} }}
  action buy(b: int): bool = {buy}
{acts}
  action step = {step}
{chr(10).join(invs)}
{chr(10).join(temps)}
{chr(10).join(runs)}
}}
"""


if __name__ == "__main__":
    print(model(int(sys.argv[1])), end="")
