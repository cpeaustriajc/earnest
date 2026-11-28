import random, sys

SETS = ["Set(0).filter(v => v > 0)", "Set(1)", "Set(1, 2)", "Set(1, 2, 3)", "Set(0, 1, 2, 3)", "0.to(4)"]
BIG_SETS = ["0.to(119)", "0.to(99)", "0.to(98)", "{x}.to({x} + 119)", "0.to({x} * 60)"]
BIG_TESTS = ["true", "p < 100", "p == 500", "p % 2 == {x} % 2", "p >= {x}", "p != 3"]
BIG_STEPS = [
    "nondet p = {set}.oneOf() all {{ {test}, x' = {x} + p % 2 }}",
    "nondet q = Set(0, 1).oneOf() nondet p = {set}.oneOf() all {{ {test} or q == 1, x' = {x} + q }}",
    "nondet p = {set}.oneOf() nondet q = Set(0, 1).oneOf() all {{ {test}, q == p % 2, x' = {x} + q }}",
    "any {{ nondet p = {set}.oneOf() all {{ {test}, x' = {x} + 1 }}, all {{ {x} > 1, x' = {x} }} }}",
    "nondet p = {set}.oneOf() if (p < 50) x' = {x} + 1 else all {{ {test}, x' = {x} }}",
]


def condition(rnd, names, reads=True):
    picked = rnd.choice(names)
    state = [f"x < 3", f"x != {picked}", f"x + {picked} <= 3", "x >= 0"] if reads else []
    return rnd.choice([f"{picked} < 2", f"{picked} != 1", f"{picked} >= 2", "true"] + state
                      + ([f"{names[0]} + {names[1]} == 3"] if len(names) > 1 else []))


def big_step(rnd, x="x"):
    shape, set, test = rnd.choice(BIG_STEPS), rnd.choice(BIG_SETS), rnd.choice(BIG_TESTS)
    return shape.format(set=set.format(x=x), test=test.format(x=x), x=x)


def run(rnd, number, big=False):
    names = ["n"] + (["k"] if rnd.random() < 0.4 else [])
    tops = "".join(f"nondet {name} = {rnd.choice(SETS)}.oneOf() " for name in names)
    if rnd.random() < 0.2:
        tops += f"val m = {names[0]} + 1 "
        names = names + ["m"]
    inner = rnd.random() < 0.2

    def first():
        if big and rnd.random() < 0.3:
            return f"({big_step(rnd, '0')})"
        roll = rnd.random()
        if roll < 0.4:
            return "init"
        if roll < 0.55:
            return f"all {{ assert({condition(rnd, names, False)}), x' = {rnd.choice(names)} }}"
        if inner and roll < 0.7:
            return "pickStart"
        return f"all {{ {condition(rnd, names, False)}, x' = {rnd.choice(names)} }}"

    def step():
        if big and rnd.random() < 0.5:
            return big_step(rnd)
        roll = rnd.random()
        if roll < 0.25:
            return "step"
        if roll < 0.35:
            return f"{rnd.randint(1, 2)}.reps(_ => {rnd.choice(['step', 'all { ' + condition(rnd, names) + ', x' + chr(39) + ' = x + 1 }'])})"
        if roll < 0.45:
            return f"chain(({condition(rnd, names, False)}))"
        if inner and roll < 0.6:
            return "pickStep"
        return f"all {{ {condition(rnd, names)}, x' = x + {rnd.choice(names + ['1'])} }}"

    body = first()
    for _ in range(rnd.randint(0, 3)):
        body = f"{body}.then({step()})"
        if rnd.random() < 0.2:
            body = f"{body}.expect({condition(rnd, names)})"
    if rnd.random() < 0.15:
        body = f"{body}.fail()"
    return f"  run r{number}Test = {{ {tops}{body} }}", inner


def model(seed, big=False):
    rnd = random.Random(seed)
    text, inner = run(rnd, 0, big)
    return f"""/// @check
module S{seed} {{
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  action chain(go: bool): bool = step.then(all {{ go, x' = x }})
  action pickStart = {{ nondet p = Set(0, 1, 2).oneOf() x' = p }}
  action pickStep = {{ nondet p = Set(1, 2).oneOf() x' = x + p }}
{text}
}}
""", inner


if __name__ == "__main__":
    print(model(int(sys.argv[1]), sys.argv[2:] == ["big"])[0], end="")
