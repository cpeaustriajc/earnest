import random, sys
def model(seed):
    r = random.Random(seed)
    L = ["/// @check", f"module H{seed} {{", "  var x: int", "  var y: int",
         "  action init = all { x' = 0, y' = 0 }"]
    def c():
        return f"{r.choice(['x', 'y', 'x + y', 'x - y', 'n', 'w'])} {r.choice(['<', '<=', '>', '==', '!='])} {r.randint(-1, 3)}"
    def act(depth=0, inpick=False):
        k = r.random()
        if k < 0.25 or depth > 1:
            return f"all {{ x' = x + {r.randint(-1, 2)}, y' = y }}"
        if k < 0.4:
            return f"all {{ {c() if inpick else c().replace('w', 'x')}, x' = x, y' = y + 1 }}".replace(" n ", " x ").replace("{ n", "{ x")
        if k < 0.6:
            return f"any {{ {act(depth + 1, inpick)}, {act(depth + 1, inpick)} }}"
        if k < 0.75:
            g = c().replace("w", "x") if not inpick else c()
            return f"if ({g}) {act(depth + 1, inpick)} else {act(depth + 1, inpick)}"
        size = r.choice([2, 3, 5])
        return f"(nondet w = 0.to({size - 1}).oneOf() all {{ {c().replace('n', 'w')}, x' = x + w, y' = y }})"
    def fix(e, has_n):
        if not has_n:
            e = e.replace(" n ", " x ").replace("(n ", "(x ").replace("{ n ", "{ x ")
        return e.replace("{ w ", "{ x ").replace("(w ", "(x ") if "nondet w" not in e else e
    for i in range(6):
        has_n = r.random() < 0.6
        top = f"nondet n = 0.to({r.choice([1, 2, 4])}).oneOf() " if has_n else ""
        chain = "init"
        for _ in range(r.randint(1, 3)):
            k = r.random()
            if k < 0.2:
                chain += f".then({r.randint(1, 3)}.reps(_ => {act()}))"
            elif k < 0.35:
                chain += f".expect({c()})"
            else:
                chain += f".then({act()})"
        e = r.random()
        if e < 0.2:
            chain += ".fail()"
        elif e < 0.45:
            chain += f".expect({c()})"
        L.append(f"  run r{i}Test = {top}{fix(chain, has_n)}")
    L.append("}")
    text = "\n".join(L) + "\n"
    text = text.replace(" w ", " x ").replace("(w ", "(x ") if False else text
    return text
if __name__ == "__main__":
    print(model(int(sys.argv[1])))
