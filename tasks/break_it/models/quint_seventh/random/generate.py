import random, sys

def model(seed):
    r = random.Random(seed)
    big = r.random() < 0.35
    B = r.choice(["2^31 - 2", "2^62 - 2", "-(2^62)"]) if big else "0"
    K = r.randint(2, 4)
    div = r.random() < 0.5
    def lin():
        return r.choice(["x - B", "y", "x - B + y", "y - (x - B)", "2 * y - 1"])
    def cond():
        e = lin()
        if div and r.random() < 0.5:
            e = f"({e}) {r.choice(['/', '%'])} {r.choice([2, 3])}"
        return f"{e} {r.choice(['<', '<=', '>', '>=', '==', '!='])} {r.randint(-2, 3)}"
    acts = {}
    acts["inc"] = f"all {{ x < B + {K}, x' = x + 1, y' = y }}"
    acts["dec"] = f"all {{ y > -{K}, y' = y - 1, x' = x }}"
    d = r.choice([2, 3])
    acts["mix"] = f"all {{ {cond()}, y' = if (y > -{K} and y < {K}) (y * {r.choice([-1, 1, 2])}) {r.choice(['/', '%']) if div else '+'} {d} else 0, x' = x }}"
    acts["pick"] = f"nondet v = (-1).to(1).oneOf() all {{ y + v >= -{K}, y + v <= {K}, y' = y + v, x' = x }}"
    acts["reset"] = f"all {{ x >= B + {K}, x' = B, y' = y }}"
    used = [a for a in acts if r.random() < 0.7] or ["inc"]
    if "inc" not in used and r.random() < 0.5:
        used.append("inc")
    lines = [f"/// @check", f"module G{seed} {{", f"  pure val B = {B}", "  var x: int", "  var y: int"]
    if r.random() < 0.5:
        lines.append(f"  action init = all {{ x' = B, y' = {r.randint(-1, 1)} }}")
    else:
        lines.append(f"  action init = nondet v = (-1).to(1).oneOf() all {{ x' = B, y' = v }}")
    for a in used:
        lines.append(f"  action {a} = {acts[a]}")
    lines.append("  action step = any { " + ", ".join(used) + " }")
    props = []
    for i in range(2):
        lines.append(f"  /// @check\n  val inv{i} = {cond()} or {cond()}")
        props.append(("inv", f"inv{i}"))
    for i in range(2):
        a = r.choice(used + ["step"])
        vs = r.choice(["(x, y)", "(y, x)", "x", "y"])
        if r.random() < 0.6:
            body = f"eventually({cond()})"
        else:
            body = f"leadsTo({cond()}, {cond()})" if r.random() < 0.5 else f"always(eventually({cond()}))"
        lines.append(f"  /// @check\n  temporal t{i} = weakFair({a}, {vs}).implies({body})")
        props.append(("temp", f"t{i}"))
    def step_expr():
        c = r.random()
        if c < 0.5:
            return r.choice(used + ["step"])
        if c < 0.75:
            return f"all {{ {cond()}, {acts[r.choice(['inc', 'dec', 'mix', 'pick'])]} }}"
        n = r.choice([3, 10, 120]) if r.random() < 0.2 else r.choice([3, 10])
        return f"(nondet w = 0.to({n - 1}).oneOf() all {{ w {r.choice(['<', '>=', '!='])} {r.randint(0, 3)}, x' = x + w % 2, y' = y }})"
    for i in range(4):
        shape = r.random()
        top = ""
        if r.random() < 0.6:
            size = r.choice([3, 5, 99, 100, 150]) if r.random() < 0.3 else r.choice([2, 3, 5])
            base = r.choice(["0", "B", "-3", "2^62"])
            top = f"nondet n = ({base}).to({base} + {size - 1}).oneOf() "
            if r.random() < 0.3:
                top += f"nondet m = Set(-1, 1).oneOf() "
        nv = "n" if top else "0"
        if shape < 0.15:
            body = f"(({nv} - B) {r.choice(['/', '%', '+'])} {r.choice([-2, 2, 3])}) {r.choice(['<', '==', '>='])} {r.randint(-2, 2)}"
        elif shape < 0.25:
            body = f"assert({nv} {r.choice(['<', '>=', '!='])} {r.randint(0, 3)})"
        else:
            first = "init" if (not top or r.random() < 0.5) else f"all {{ {nv} {r.choice(['<', '>=', '!='])} {r.randint(0, 3)}, x' = B, y' = 0 }}"
            chain = first
            for _ in range(r.randint(0, 3)):
                k = r.random()
                if k < 0.15:
                    chain += f".then({r.randint(1, 3)}.reps(_ => {step_expr()}))"
                elif k < 0.3:
                    chain += f".expect({cond()})"
                else:
                    chain += f".then({step_expr()})"
            if top and r.random() < 0.4:
                chain += f".then(all {{ {nv} {r.choice(['<', '>=', '!='])} {r.randint(0, 3)}, x' = x, y' = y }})"
            end = r.random()
            if end < 0.3:
                chain += f".expect({cond()})"
            elif end < 0.4:
                chain += ".fail()"
            body = chain
        lines.append(f"  run r{i}Test = {top}{body}")
    lines.append("}")
    return "\n".join(lines) + "\n", {"big": big, "div": div, "props": props}

if __name__ == "__main__":
    import json
    text, meta = model(int(sys.argv[1]))
    open(sys.argv[2], "w").write(text)
    print(json.dumps(meta))
