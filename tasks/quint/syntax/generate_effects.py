import os
import random
import sys

folder, count, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
MISTAKES = [
    "none",
    "action_in_val",
    "action_in_def",
    "temporal_in_action",
    "read_in_pure",
    "update_twice_in_all",
    "update_twice_in_any",
    "nondet_impure",
    "nondet_not_one_of",
    "one_of_outside",
    "nondet_top",
    "override_impure",
    "any_different",
    "action_in_and",
]


class Program:
    def __init__(self, rng, mistake):
        self.rng = rng
        self.mistake = mistake
        self.variables = [f"x{index}" for index in range(rng.randint(1, 4))]
        self.ints = []
        self.bools = []
        self.actions = []
        self.lines = []
        self.counter = 0

    def name(self, prefix):
        self.counter += 1
        return f"{prefix}{self.counter}"

    def integer(self, depth, local=()):
        rng = self.rng
        choices = ["literal", "var"]
        if local:
            choices.append("local")
        if depth > 0:
            choices += ["plus", "if", "fold", "let", "call", "lambda"]
        kind = rng.choice(choices)
        if kind == "literal":
            return str(rng.randint(0, 9))
        if kind == "var":
            return rng.choice(self.variables + ["s.size()"])
        if kind == "local":
            return rng.choice(local)
        if kind == "plus":
            return f"({self.integer(depth - 1, local)} + {self.integer(depth - 1, local)})"
        if kind == "if":
            return f"(if ({self.boolean(depth - 1, local)}) {self.integer(depth - 1, local)} else {self.integer(depth - 1, local)})"
        if kind == "fold":
            return f"s.fold({self.integer(depth - 1, local)}, (a, b) => a + b + {self.integer(depth - 1, local + ('a', 'b'))})"
        if kind == "let":
            name = self.name("l")
            return f"{{ val {name} = {self.integer(depth - 1, local)} {name} + {self.integer(depth - 1, local + (name,))} }}"
        if kind == "call" and self.ints:
            name, arity = rng.choice(self.ints)
            arguments = ", ".join(self.integer(depth - 1, local) for _ in range(arity))
            return f"{name}({arguments})" if arity else name
        if kind == "lambda":
            return f"s.map(n => n + {self.integer(depth - 1, local + ('n',))}).size()"
        return str(rng.randint(0, 9))

    def boolean(self, depth, local=()):
        rng = self.rng
        choices = ["compare", "compare"]
        if depth > 0:
            choices += ["and", "or", "not", "exists", "call"]
        kind = rng.choice(choices)
        if kind == "compare":
            operator = rng.choice([">", "<", "==", "!=", ">="])
            return f"({self.integer(max(depth - 1, 0), local)} {operator} {self.integer(max(depth - 1, 0), local)})"
        if kind == "and":
            return f"and {{ {self.boolean(depth - 1, local)}, {self.boolean(depth - 1, local)} }}"
        if kind == "or":
            return f"({self.boolean(depth - 1, local)} or {self.boolean(depth - 1, local)})"
        if kind == "not":
            return f"not({self.boolean(depth - 1, local)})"
        if kind == "exists":
            return f"s.exists(n => {self.boolean(depth - 1, local + ('n',))})"
        if kind == "call" and self.bools:
            name, arity = rng.choice(self.bools)
            arguments = ", ".join(self.integer(depth - 1, local) for _ in range(arity))
            return f"{name}({arguments})" if arity else name
        return "true"

    def value(self, variable, depth, local):
        if variable == "s":
            return f"Set({self.integer(depth, local)})"
        return self.integer(depth, local)

    def action(self, depth, variables, local=()):
        rng = self.rng
        choices = ["all"]
        if len(variables) == 1:
            choices.append("assign")
        if depth > 0:
            choices += ["any", "if", "nondet", "call", "guarded"]
        kind = rng.choice(choices)
        if kind == "assign":
            return f"{variables[0]}' = {self.value(variables[0], depth, local)}"
        if kind == "all":
            if len(variables) == 1:
                return f"all {{ {variables[0]}' = {self.value(variables[0], depth, local)} }}"
            cut = rng.randint(1, len(variables) - 1)
            return f"all {{ {self.action(depth - 1, variables[:cut], local)}, {self.action(depth - 1, variables[cut:], local)} }}"
        if kind == "any":
            return f"any {{ {self.action(depth - 1, variables, local)}, {self.action(depth - 1, variables, local)} }}"
        if kind == "if":
            return f"if ({self.boolean(depth - 1, local)}) {self.action(depth - 1, variables, local)} else {self.action(depth - 1, variables, local)}"
        if kind == "nondet":
            name = self.name("n")
            return f"{{ nondet {name} = s.oneOf() {self.action(depth - 1, variables, local + (name,))} }}"
        if kind == "guarded":
            return f"all {{ {self.boolean(depth - 1, local)}, {self.action(depth - 1, variables, local)} }}"
        if kind == "call":
            matching = [name for name, written in self.actions if written == variables]
            if matching:
                return rng.choice(matching)
        return f"all {{ {', '.join(f'{variable}{chr(39)} = {self.value(variable, 0, local)}' for variable in variables)} }}"

    def subset(self):
        variables = self.variables + ["s"]
        chosen = sorted(self.rng.sample(variables, self.rng.randint(1, len(variables))), key=variables.index)
        return chosen

    def body(self):
        rng = self.rng
        for _ in range(rng.randint(3, 12)):
            kind = rng.choice(["pure", "val", "def", "action", "action", "temporal"])
            if kind == "pure":
                name = self.name("p")
                arity = rng.randint(0, 2)
                params = [f"a{index}" for index in range(arity)]
                expression = self.pure_integer(2, tuple(params))
                header = f"pure def {name}({', '.join(params)})" if arity else f"pure val {name}"
                self.lines.append(f"  {header} = {expression}")
                self.ints.append((name, arity))
            elif kind == "val":
                name = self.name("v")
                self.lines.append(f"  val {name} = {self.boolean(2)}")
                self.bools.append((name, 0))
            elif kind == "def":
                name = self.name("d")
                self.lines.append(f"  def {name}(a) = {self.integer(2, ('a',))}")
                self.ints.append((name, 1))
            elif kind == "action":
                name = self.name("act")
                variables = self.subset()
                self.lines.append(f"  action {name} = {self.action(2, variables)}")
                self.actions.append((name, variables))
            else:
                name = self.name("t")
                options = [f"always({self.boolean(1)})", f"eventually({self.boolean(1)})"]
                if self.actions:
                    action, variables = rng.choice(self.actions)
                    options.append(f"weakFair({action}, {variables[0]})")
                    options.append(f"always({action}.orKeep({variables[0]}))")
                    options.append(f"always(enabled({action}) implies eventually({self.boolean(1)}))")
                self.lines.append(f"  temporal {name} = {rng.choice(options)}")

    def pure_integer(self, depth, params):
        rng = self.rng
        if depth == 0 or rng.random() < 0.3:
            return rng.choice(list(params) + [str(rng.randint(0, 9))])
        return f"({self.pure_integer(depth - 1, params)} + {self.pure_integer(depth - 1, params)})"

    def plant(self):
        rng = self.rng
        x = rng.choice(self.variables)
        others = [variable for variable in self.variables if variable != x]
        line = None
        if self.mistake == "action_in_val":
            line = f"  val planted = {x}' = {self.integer(1)}"
        elif self.mistake == "action_in_def":
            line = f"  def planted(a) = {x}' = a"
        elif self.mistake == "temporal_in_action":
            line = f"  action planted = always({self.boolean(1)})"
        elif self.mistake == "read_in_pure":
            line = f"  pure val planted = {x} + 1"
        elif self.mistake == "update_twice_in_all":
            line = f"  action planted = all {{ {x}' = 1, {self.action(1, [x])} }}"
        elif self.mistake == "update_twice_in_any":
            line = f"  action planted = any {{ {x}' = 1, all {{ {x}' = 2, {x}' = 3 }} }}"
        elif self.mistake == "nondet_impure":
            line = f"  action planted = {{ nondet n = Set({x}' = 1).oneOf() {x}' = 2 }}"
        elif self.mistake == "nondet_not_one_of":
            line = f"  action planted = {{ nondet n = {self.integer(1)} {x}' = n }}"
        elif self.mistake == "one_of_outside":
            line = f"  val planted = s.oneOf() > {self.integer(1)}"
        elif self.mistake == "nondet_top":
            line = "  nondet planted = s.oneOf()"
        elif self.mistake == "any_different" and others:
            line = f"  action planted = any {{ {x}' = 1, all {{ {x}' = 2, {others[0]}' = 3 }} }}"
        elif self.mistake == "action_in_and" and others:
            line = f"  action planted = {x}' = 1 and {others[0]}' = 2"
        if line is not None:
            self.lines.insert(rng.randint(0, len(self.lines)), line)

    def text(self, index):
        rng = self.rng
        self.body()
        self.plant()
        declarations = [f"  var {variable}: int" for variable in self.variables] + ["  var s: Set[int]"]
        main = [f"module G{index} {{"] + declarations + self.lines + ["}"]
        if self.mistake == "override_impure" or rng.random() < 0.2:
            value = f"{self.variables[0]} + 1" if self.mistake == "override_impure" else str(rng.randint(1, 5))
            inner = [
                "module Inner {",
                "  const N: int",
                "  var y: int",
                "  action init = y' = N",
                "  action step = y' = y + N",
                "  val small = y < N",
                "}",
            ]
            main.insert(-1, f"  import Inner(N = {value}) as I")
            main.insert(-1, f"  action withInner = all {{ I::step, {', '.join(f'{v}{chr(39)} = {v}' for v in self.variables)}, s' = s }}")
            main.insert(-1, "  val innerSmall = I::small")
            return "\n".join(inner + [""] + main) + "\n"
        return "\n".join(main) + "\n"


if len(sys.argv) > 4:
    MISTAKES = sys.argv[4].split(",")
os.makedirs(folder, exist_ok=True)
rng = random.Random(seed)
for index in range(count):
    mistake = MISTAKES[index % len(MISTAKES)]
    program = Program(rng, mistake)
    with open(os.path.join(folder, f"G{index:04d}_{mistake}.qnt"), "w") as file:
        file.write(program.text(index))
