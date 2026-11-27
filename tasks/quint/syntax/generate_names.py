"""Writes Quint files that stress name resolution, for comparing earnest with `quint parse`.

Usage: python3 -I generate_names.py <folder> <count> [seed]

Each file holds a chain of modules: shadowing at several depths, a name reached by two
routes (a plain import and an export), instances nested three deep with constants, names used
before they are defined, qualified and aliased imports, and now and then a cycle, a name that
is not there or a conflict, so that refusals are compared too.
"""

import os
import random
import sys

folder, count = sys.argv[1], int(sys.argv[2])
seed = int(sys.argv[3]) if len(sys.argv) > 3 else 1
NAMES = ["a", "b", "c", "x", "y", "z", "k", "v"]


def expression(rng, scope, depth):
    if depth <= 0 or rng.random() < 0.25:
        if scope and rng.random() < 0.8:
            return rng.choice(scope)
        return str(rng.randint(0, 9))
    choice = rng.randrange(6)
    if choice == 0:
        name = rng.choice(NAMES)
        return f"Set({expression(rng, scope, depth - 1)}).map({name} => {expression(rng, scope + [name], depth - 1)}).size()"
    if choice == 1:
        name = rng.choice(NAMES)
        return f"{{ val {name} = {expression(rng, scope, depth - 1)} {expression(rng, scope + [name], depth - 1)} }}"
    if choice == 2:
        name = rng.choice(NAMES)
        param = rng.choice(NAMES)
        return f"{{ def {name}({param}) = {expression(rng, scope + [param], depth - 1)} {name}({expression(rng, scope, depth - 1)}) }}"
    if choice == 3:
        return f"({expression(rng, scope, depth - 1)} + {expression(rng, scope, depth - 1)})"
    if choice == 4:
        return f"if ({expression(rng, scope, depth - 1)} > 0) {expression(rng, scope, depth - 1)} else {expression(rng, scope, depth - 1)}"
    name = rng.choice(NAMES)
    return f"0.to(3).fold({expression(rng, scope, depth - 1)}, ({name}, i) => {name} + i)"


def module(rng, index, previous):
    name = f"M{index}"
    lines = [f"module {name} {{"]
    scope = []
    if previous:
        target = rng.choice(previous)
        form = rng.randrange(6)
        if form == 0:
            lines.append(f"  import {target}.*")
            scope += [f"d{target[1:]}", f"e{target[1:]}"]
        elif form == 1:
            lines.append(f"  import {target} as Q{index}")
            scope.append(f"Q{index}::d{target[1:]}")
        elif form == 2:
            lines.append(f"  import {target}.d{target[1:]}")
            scope.append(f"d{target[1:]}")
        elif form == 3:
            lines.append(f"  import {target}")
            scope.append(f"{target}::e{target[1:]}")
        elif form == 4:
            lines.append(f"  import {target}.*")
            lines.append(f"  export {target}.*")
            scope.append(f"d{target[1:]}")
        else:
            lines.append(f"  import {target}.*")
            lines.append(f"  import {target}.d{target[1:]}")
            scope.append(f"d{target[1:]}")
    if rng.random() < 0.05:
        scope.append("nowhere")
    later = f"f{index}"
    lines.append(f"  val d{index} = {expression(rng, scope + [later], 3)}")
    lines.append(f"  def e{index}(p) = {expression(rng, scope + ['p', later], 3)}")
    lines.append(f"  val {later} = {expression(rng, scope, 2)}")
    if rng.random() < 0.05:
        lines.append(f"  val d{index} = 1")
    if rng.random() < 0.05:
        lines.append(f"  def g{index}(n) = g{index}(n)")
    lines.append("}")
    return name, lines


def instances(rng, index):
    base = f"I{index}"
    return [
        f"module {base}A {{",
        "  const K: int",
        "  var s: int",
        "  val value = K",
        "  action init = s' = K",
        "}",
        f"module {base}B {{",
        "  const L: int",
        f"  import {base}A(K = L) as A",
        "  val value = A::value + L",
        "}",
        f"module {base}C {{",
        "  const M: int",
        f"  import {base}B(L = M + 1) as B",
        f"  import {base}A(K = M).*",
        "  val total = B::value + value",
        "}",
        f"module {base}D {{",
        f"  import {base}C(M = {rng.randint(0, 5)}) as C",
        "  val top = C::total",
        "}",
    ]


os.makedirs(folder, exist_ok=True)
for number in range(count):
    rng = random.Random(seed * 100003 + number)
    lines = []
    previous = []
    for index in range(rng.randint(2, 5)):
        name, written = module(rng, index, previous)
        lines += written
        previous.append(name)
    lines += instances(rng, number)
    with open(os.path.join(folder, f"names{number}.qnt"), "w") as file:
        file.write("\n".join(lines) + "\n")
