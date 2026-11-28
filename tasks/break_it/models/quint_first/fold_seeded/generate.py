import functools, random, sys

SHAPES = [
    ("set", ("int",)),
    ("list", ("int",)),
    ("map", ("int",), ("int",)),
    ("tup", ("int",), ("int",)),
    ("rec", ("int",), ("int",)),
    ("set", ("list", ("int",))),
    ("list", ("set", ("int",))),
    ("tup", ("set", ("int",)), ("int",)),
    ("map", ("int",), ("set", ("int",))),
    ("map", ("set", ("int",)), ("int",)),
    ("rec", ("list", ("int",)), ("set", ("int",))),
    ("set", ("tup", ("int",), ("int",))),
    ("list", ("map", ("int",), ("int",))),
    ("tup", ("bool",), ("list", ("int",))),
]


def value(rnd, shape):
    kind = shape[0]
    if kind == "int":
        return ("int", rnd.randint(0, 3))
    if kind == "bool":
        return ("bool", rnd.random() < 0.5)
    if kind == "set":
        items = {}
        for _ in range(rnd.randint(0, 3)):
            item = value(rnd, shape[1])
            items.setdefault(key(item), item)
        return ("set", list(items.values()))
    if kind == "list":
        return ("list", [value(rnd, shape[1]) for _ in range(rnd.randint(0, 3))])
    if kind == "map":
        pairs = {}
        for _ in range(rnd.randint(0, 3)):
            item = value(rnd, shape[1])
            pairs[key(item)] = (item, value(rnd, shape[2]))
        return ("map", list(pairs.values()))
    if kind == "tup":
        return ("tup", [value(rnd, shape[1]), value(rnd, shape[2])])
    return ("rec", [("a", value(rnd, shape[1])), ("b", value(rnd, shape[2]))])


def key(v):
    return repr(normal(v))


def normal(v):
    kind = v[0]
    if kind == "set":
        return ("set", tuple(sorted(key(i) for i in v[1])))
    if kind == "map":
        return ("map", tuple(sorted((key(k), key(val)) for k, val in v[1])))
    if kind in ("list", "tup"):
        return (kind, tuple(normal(i) for i in v[1]))
    if kind == "rec":
        return ("rec", tuple((n, normal(i)) for n, i in v[1]))
    return v


def items_of(v):
    kind = v[0]
    if kind == "set":
        return sorted(v[1], key=functools.cmp_to_key(tlc))
    return v[1]


def each(left, right):
    for one, other in zip(left, right):
        c = tlc(one, other)
        if c:
            return c
    return 0


def tlc(left, right):
    kind = left[0]
    if kind in ("int", "bool"):
        return (left[1] > right[1]) - (left[1] < right[1])
    if len(left[1]) != len(right[1]):
        return len(left[1]) - len(right[1])
    if kind == "set":
        return each(items_of(left), items_of(right))
    if kind in ("list", "tup"):
        return each(left[1], right[1])
    if kind == "map":
        one = sorted(left[1], key=functools.cmp_to_key(lambda x, y: tlc(x[0], y[0])))
        other = sorted(right[1], key=functools.cmp_to_key(lambda x, y: tlc(x[0], y[0])))
        return each([k for k, _ in one], [k for k, _ in other]) or each(
            [v for _, v in one], [v for _, v in other]
        )
    return each([v for _, v in left[1]], [v for _, v in right[1]])


def quint(v):
    kind = v[0]
    if kind == "int":
        return str(v[1])
    if kind == "bool":
        return "true" if v[1] else "false"
    if kind == "set":
        return "Set(" + ", ".join(quint(i) for i in v[1]) + ")"
    if kind == "list":
        return "[" + ", ".join(quint(i) for i in v[1]) + "]"
    if kind == "map":
        return "Map(" + ", ".join(f"{quint(k)} -> {quint(i)}" for k, i in v[1]) + ")"
    if kind == "tup":
        return "(" + ", ".join(quint(i) for i in v[1]) + ")"
    return "{ " + ", ".join(f"{n}: {quint(i)}" for n, i in v[1]) + " }"


def typed(shape):
    kind = shape[0]
    if kind == "int":
        return "int"
    if kind == "bool":
        return "bool"
    if kind == "set":
        return f"Set[{typed(shape[1])}]"
    if kind == "list":
        return f"List[{typed(shape[1])}]"
    if kind == "map":
        return f"{typed(shape[1])} -> {typed(shape[2])}"
    if kind == "tup":
        return f"({typed(shape[1])}, {typed(shape[2])})"
    return f"{{ a: {typed(shape[1])}, b: {typed(shape[2])} }}"


def module(seed, count):
    rnd = random.Random(seed)
    lines = [
        "module fold_seeded {",
        "  var n: int",
        "  action init = n' = 0",
        "  action step = all { n < 1, n' = n + 1 }",
    ]
    names = []
    for index in range(count):
        shape = SHAPES[index % len(SHAPES)]
        items = {}
        while len(items) < 4:
            v = value(rnd, shape)
            items.setdefault(key(v), v)
        written = list(items.values())
        rnd.shuffle(written)
        ordered = sorted(written, key=functools.cmp_to_key(tlc))
        name = f"fold{index}"
        names.append(name)
        set_text = "Set(" + ", ".join(quint(v) for v in written) + ")"
        list_text = "[" + ", ".join(quint(v) for v in ordered) + "]"
        kind = typed(shape)
        lines.append(f"  pure val {name}Set: Set[{kind}] = {set_text}")
        lines.append(f"  pure val {name}Order: List[{kind}] = {list_text}")
        lines.append(
            f"  val {name} = {name}Set.fold([], (taken, each) => taken.append(each)) == {name}Order"
        )
    lines.append("}")
    return "\n".join(lines) + "\n", names


if __name__ == "__main__":
    text, names = module(int(sys.argv[1]), int(sys.argv[2]))
    sys.stdout.write(text)
    sys.stderr.write(",".join(names) + "\n")
