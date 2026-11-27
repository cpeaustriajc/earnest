#!/usr/bin/env python3
"""Writes the deep models: one check per shape and depth, each with one property that holds and
one that is broken, both reading the deep expression. Run from anywhere:

    python3 tasks/quint/deep/generate.py [--slow] [folder] [depth...]

With no arguments it writes the samples kept in the repository, in this folder: every shape at
100, 300 and 500, except the slow shapes, at 100 only. Other depths are for measuring, and go to
another folder.
"""

import argparse
import pathlib

KEPT = (100, 300, 500)

# Shapes whose type is as deep as the expression. Each costs minutes past 100 deep, so the
# repository keeps them at 100 only: earnest's type inference in a debug build (the corpus tests
# of earnest_quint_syntax read every .qnt file) takes 24 s for sets_300, 142 s for records_300
# and over 300 s for lambdas_300, and Quint 0.33.1 needs 81 s and 370 s to compile sets_300 and
# sets_500, 573 s for records_300, about 30 minutes to record lambdas_300 and over 30 minutes
# to compile lambdas_999.
SLOW = ("sets", "records", "lambdas")

# x goes 0, 1, 2, 3, 0, ...: a property "<= 3" holds, "<= 2" is broken at x = 2 or 3.
HEADER = """/// @check
module {name} {{
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
"""


def wrap(depth, opening, inner, closing):
    return opening * depth + inner + closing * depth


def nots(depth, bound):
    inner = f"x <= {bound}" if depth % 2 == 0 else f"x > {bound}"
    return wrap(depth, "not(", inner, ")")


def sets(depth, inner):
    return wrap(depth, "Set(", inner, ")")


def parens(depth, bound):
    return wrap(depth, "(", "x", ")") + f" <= {bound}"


def chain(depth):
    arms = "".join(f"if (x == {each}) {each} else " for each in range(depth))
    return f"{arms}{depth}"


def ifs(depth):
    return wrap(depth, "if (x >= 0) ", "x", " else -1")


def record(depth):
    return wrap(depth, "{ a: ", "x", " }")


def fields(depth):
    return "deep" + ".a" * depth


def ands(depth, bound):
    return wrap(depth - 1, "and { x >= 0, ", f"x <= {bound}", " }")


def ors(depth, bound):
    return wrap(depth - 1, "or { x > 9, ", f"x <= {bound}", " }")


def lambdas(depth, bound):
    opened = "Set(x).exists(a1 => " + "".join(
        f"Set(a{each - 1}).exists(a{each} => " for each in range(2, depth + 1)
    )
    return opened + f"a{depth} <= {bound}" + ")" * depth


def lets(depth, bound):
    lines = ["{", "    val a1 = x"]
    lines += [f"    val a{each} = a{each - 1}" for each in range(2, depth + 1)]
    lines += [f"    a{depth} <= {bound}", "  }"]
    return "\n".join(lines)


def sums(depth, bound):
    return " + ".join(["0"] * depth + ["x"]) + f" <= {bound}"


def implies(depth, bound):
    return wrap(depth, "x >= 0 implies (", f"x <= {bound}", ")")


def shapes(depth):
    """Each shape: (name, definitions, the property that holds, the one that is broken)."""
    return [
        ("nots", "", nots(depth, 3), nots(depth, 2)),
        ("sets", f"  val deep = {sets(depth, 'x')}\n", "deep.size() == 1", f"deep == {sets(depth, '0')}"),
        ("parens", "", parens(depth, 3), parens(depth, 2)),
        ("elseifs", f"  val deep = {chain(depth)}\n", "deep == x", "deep != 2"),
        ("ifs", f"  val deep = {ifs(depth)}\n", "deep == x", "deep <= 2"),
        ("records", f"  val deep = {record(depth)}\n", f"{fields(depth)} == x", f"{fields(depth)} <= 2"),
        ("ands", "", ands(depth, 3), ands(depth, 2)),
        ("ors", "", ors(depth, 3), ors(depth, 2)),
        ("lambdas", "", lambdas(depth, 3), lambdas(depth, 2)),
        ("lets", "", lets(depth, 3), lets(depth, 2)),
        ("sums", "", sums(depth, 3), sums(depth, 2)),
        ("implies", "", implies(depth, 3), implies(depth, 2)),
    ]


def model(name, definitions, holds, broken):
    return (
        HEADER.format(name=name)
        + definitions
        + f"  /// @check\n  val holds = {holds}\n"
        + f"  /// @check\n  val broken = {broken}\n"
        + "}\n"
    )


def main():
    here = pathlib.Path(__file__).resolve().parent
    arguments = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    arguments.add_argument(
        "--slow",
        action="store_true",
        help=f"also write {', '.join(SLOW)} past 100 deep: minutes each for earnest's debug type "
        "inference and for Quint 0.33.1 to record (see SLOW in this file)",
    )
    arguments.add_argument("folder", nargs="?", type=pathlib.Path, default=here)
    arguments.add_argument("depths", nargs="*", type=int)
    asked = arguments.parse_args()
    depths = asked.depths or list(KEPT)
    asked.folder.mkdir(parents=True, exist_ok=True)
    for depth in depths:
        for shape, definitions, holds, broken in shapes(depth):
            if shape in SLOW and depth > 100 and not asked.slow and not asked.depths:
                continue
            name = f"{shape}_{depth}"
            (asked.folder / f"{name}.qnt").write_text(model(name, definitions, holds, broken))


if __name__ == "__main__":
    main()
