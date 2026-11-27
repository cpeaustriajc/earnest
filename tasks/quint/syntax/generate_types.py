import os
import random
import sys

folder, count, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
random.seed(seed)

BASE = ["int", "str", "bool"]
FIELDS = ["a", "b", "c", "d"]


def random_type(depth):
    if depth <= 0 or random.random() < 0.35:
        return random.choice(BASE)
    shape = random.choice(["set", "list", "pair", "record", "option", "map", "either"])
    if shape in ("set", "list", "option"):
        return (shape, random_type(depth - 1))
    if shape in ("pair", "map", "either"):
        return (shape, random_type(depth - 1), random_type(depth - 1))
    names = random.sample(FIELDS, random.randint(1, 3))
    return ("record", tuple((name, random_type(depth - 1)) for name in sorted(names)))


def shown(written):
    if isinstance(written, str):
        return written
    shape = written[0]
    if shape == "set":
        return f"Set[{shown(written[1])}]"
    if shape == "list":
        return f"List[{shown(written[1])}]"
    if shape == "option":
        return f"Option[{shown(written[1])}]"
    if shape == "pair":
        return f"({shown(written[1])}, {shown(written[2])})"
    if shape == "map":
        return f"{shown(written[1])} -> {shown(written[2])}"
    if shape == "either":
        return f"Either[{shown(written[1])}, {shown(written[2])}]"
    return "{ " + ", ".join(f"{name}: {shown(inner)}" for name, inner in written[1]) + " }"


class Program:
    def __init__(self):
        self.values = []
        self.counter = 0

    def fresh(self, prefix):
        self.counter += 1
        return f"{prefix}{self.counter}"

    def value_of(self, written):
        found = [name for name, known in self.values if known == written]
        if found and random.random() < 0.5:
            return random.choice(found)
        return None

    def expression(self, written, depth):
        known = self.value_of(written)
        if known is not None:
            return known
        if depth > 0 and random.random() < 0.3:
            return self.wrapped(written, depth)
        if written == "int":
            return random.choice(["1", "2", "0", "(1 + 2)", "size(Set(1))", "List(1, 2).length()"])
        if written == "str":
            return random.choice(['"x"', '"y"', '""'])
        if written == "bool":
            return random.choice(["true", "false", "(1 < 2)", "Set(1).contains(1)"])
        shape = written[0]
        if shape == "set":
            inner = [self.expression(written[1], depth - 1) for _ in range(random.randint(1, 2))]
            return f"Set({', '.join(inner)})"
        if shape == "list":
            inner = [self.expression(written[1], depth - 1) for _ in range(random.randint(0, 2))]
            if not inner:
                return f"List({self.expression(written[1], depth - 1)}).tail()"
            return f"List({', '.join(inner)})"
        if shape == "option":
            if random.random() < 0.3:
                return "None"
            return f"Some({self.expression(written[1], depth - 1)})"
        if shape == "either":
            if random.random() < 0.5:
                return f"Left({self.expression(written[1], depth - 1)})"
            return f"Right({self.expression(written[2], depth - 1)})"
        if shape == "pair":
            return f"({self.expression(written[1], depth - 1)}, {self.expression(written[2], depth - 1)})"
        if shape == "map":
            key = self.expression(written[1], depth - 1)
            return f"Map({key} -> {self.expression(written[2], depth - 1)})"
        fields = ", ".join(f"{name}: {self.expression(inner, depth - 1)}" for name, inner in written[1])
        return "{ " + fields + " }"

    def wrapped(self, written, depth):
        choice = random.choice(["id", "compose", "ite", "let", "first", "getOr", "field", "fold", "lambda", "match", "local", "twice", "pipe"])
        inner = self.expression(written, depth - 1)
        if choice == "id":
            return f"identity({inner})"
        if choice == "compose":
            return f"compose(identity, x => x, {inner})"
        if choice == "ite":
            return f"(if ({self.expression('bool', depth - 1)}) {inner} else {self.expression(written, depth - 1)})"
        if choice == "let":
            name = self.fresh("l")
            return f"{{ val {name} = {inner}\n {name} }}"
        if choice == "first":
            return f"first(({inner}, {self.expression(random_type(1), depth - 1)}))"
        if choice == "getOr":
            return f"getOr(Some({inner}), {self.expression(written, depth - 1)})"
        if choice == "field":
            other = random_type(1)
            return f"{{ a: {inner}, z: {self.expression(other, depth - 1)} }}.a"
        if choice == "fold":
            return f"List({inner}).foldl({self.expression(written, depth - 1)}, (acc, x) => x)"
        if choice == "local":
            name = self.fresh("local")
            other = self.expression(random_type(1), depth - 1)
            return f"{{ def {name}(y) = (y, y)._1\n val {self.fresh('ignored')} = {name}({other})\n {name}({inner}) }}"
        if choice == "twice":
            return f"twice(x => identity(x), {inner})"
        if choice == "pipe":
            return f"pipe({inner}, x => Some(x), o => getOr(o, {self.expression(written, depth - 1)}))"
        if choice == "lambda":
            return f"apply(v => v, {inner})"
        return f"match Some({inner}) {{ | Some(v) => v | None => {self.expression(written, depth - 1)} }}"


PRELUDE = """  type Option[a] = Some(a) | None
  type Either[a, b] = Left(a) | Right(b)
  def identity(x) = x
  def compose(f, g, x) = f(g(x))
  def apply(f, x) = f(x)
  def first(p) = p._1
  def getOr(o, d) = match o { | Some(v) => v | None => d }
  def mapOption(o, f) = match o { | Some(v) => Some(f(v)) | None => None }
  def getA(r) = r.a
  def setA(r, v) = r.with("a", v)
  def spreadA(r, v) = { ...r, a: v }
  def throughA(r) = getA(setA(r, getA(r)))
  def swap(p) = (p._2, p._1)
  def both(e, f, g) = match e { | Left(x) => f(x) | Right(y) => g(y) }
  def twice(f, x) = f(f(x))
  def pipe(x, f, g) = g(f(x))
  def keysOf(m) = m.keys()
  def options(s) = s.map(x => Some(x))
"""

PLANTED = [
    "  val planted = 1 + \"one\"\n",
    "  val planted = identity(1, 2)\n",
    "  val planted = getA({ b: 1 })\n",
    "  val planted = { a: 1 }.with(\"b\", 2)\n",
    "  val planted = getOr(Some(1), \"x\")\n",
    "  val planted = Set(1, \"a\")\n",
    "  val planted: List[int] = List(\"a\")\n",
    "  val planted = mapOption(Some(1), x => x and true)\n",
    "  def planted(x) = x(x)\n",
    "  val planted = both(Left(1), x => x + 1, y => y.length())\n",
    "  val planted = match Some(1) { | Some(v) => v }\n",
    "  def planted(x: a): a = x + 1\n",
    "  val planted = variant(\"Missing\", 1)\n",
]

os.makedirs(folder, exist_ok=True)
for number in range(count):
    program = Program()
    lines = []
    for _ in range(random.randint(3, 9)):
        written = random_type(3)
        name = program.fresh("v")
        annotation = f": {shown(written)}" if random.random() < 0.5 else ""
        lines.append(f"  val {name}{annotation} = {program.expression(written, 3)}\n")
        program.values.append((name, written))
    uses = []
    for name, written in program.values:
        if isinstance(written, tuple) and written[0] == "record" and written[1][0][0] == "a":
            uses.append(f"  val {program.fresh('u')} = throughA({name})\n")
            uses.append(f"  val {program.fresh('u')} = spreadA({name}, getA({name}))\n")
        if isinstance(written, tuple) and written[0] == "pair":
            uses.append(f"  val {program.fresh('u')} = swap(swap({name}))\n")
        if isinstance(written, tuple) and written[0] == "option":
            uses.append(f"  val {program.fresh('u')} = mapOption({name}, x => Set(x))\n")
        if isinstance(written, tuple) and written[0] == "either":
            uses.append(f"  val {program.fresh('u')} = both({name}, x => Some(x), y => None)\n")
    body = PRELUDE + "".join(lines) + "".join(uses)
    if random.random() < 0.3:
        body += random.choice(PLANTED)
    with open(os.path.join(folder, f"T{number}.qnt"), "w") as file:
        file.write(f"module T{number} {{\n{body}}}\n")
