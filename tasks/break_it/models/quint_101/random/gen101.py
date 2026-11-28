#!/usr/bin/env python3
"""Random Quint runs that call runs with a `nondet` of their own, join chains with `if`, `and`,
`or` and `any`, and repeat 0 times, with an exact oracle of `quint test`'s possible outcomes.

The oracle transcribes Quint 0.33.1's evaluator (runtime/impl/nondet.js, builtins.js and
VarStorage.js) with the state it carries: an outcome is ('T', state), ('F', state) or
('E', code), where a state is (current, next), each a pair of values or None for unset.
A `nondet` over fewer than 100 values retries while its body is false, from the state the false
try left; over 100 or more it keeps its pick. `any` restores only the next variables between
branches; `or` restores nothing; `then` shifts and errors (QNT513) on a false first part; `reps`
errors on a false round and makes no step for a count of 0; `expect` errors (QNT508) on a false
action or condition and keeps the shift; reading an unset variable is QNT502. A test fails when
some sample ends in F or E.

usage: gen101.py SEED OUTDIR   -> writes OUTDIR/hSEED.qnt and OUTDIR/hSEED.expect
"""
import functools
import itertools
import random
import sys

BIG = 100
UNSET = (None, None)


class Unset(Exception):
    pass


def ev(e, env, cur):
    t = e[0]
    if t == 'lit':
        return e[1]
    if t == 'var':
        if e[1] in ('x', 'y'):
            value = cur[0 if e[1] == 'x' else 1]
            if value is None:
                raise Unset()
            return value
        return env[e[1]]
    if t == 'bin':
        a, b = ev(e[2], env, cur), ev(e[3], env, cur)
        return {'+': a + b, '-': a - b, '<': a < b, '==': a == b, '!=': a != b,
                '>': a > b, '>=': a >= b}[e[1]]
    if t == 'range':
        return list(range(ev(e[1], env, cur), ev(e[2], env, cur) + 1))
    raise ValueError(e)


def frozen(e):
    if isinstance(e, (list, tuple)):
        return tuple(frozen(each) for each in e)
    return e


def shifted(state):
    return (state[1], UNSET)


class Oracle:
    def __init__(self, defs):
        self.defs = defs

    def run(self, e):
        return self.go(e, (), (UNSET, UNSET))

    @functools.lru_cache(maxsize=None)
    def go(self, e, env_items, state):
        env = dict(env_items)
        try:
            return frozenset(self.step(e, env, state))
        except Unset:
            return frozenset({('E', 'QNT502')})

    def sub(self, e, env, state):
        return self.go(e, tuple(sorted(env.items())), state)

    def step(self, e, env, state):
        t = e[0]
        cur, nxt = state
        if t == 'all':
            values = list(nxt)
            for part in e[1]:
                if part[0] == 'assert':
                    if not ev(part[1], env, cur):
                        return {('E', 'QNT508')}
                elif part[0] == 'guard':
                    if not ev(part[1], env, cur):
                        return {('F', state)}
                else:
                    values[0 if part[1] == 'x' else 1] = ev(part[2], env, cur)
            return {('T', (cur, tuple(values)))}
        if t == 'init':
            return self.sub(self.defs['init'], env, state)
        if t == 'call':
            return self.sub(self.defs[e[1]], env, state)
        if t == 'nondet':
            values = ev(e[2], env, cur)
            if not values:
                return {('F', state)}
            if len(values) >= BIG:
                found = set()
                for value in values:
                    found |= self.sub(e[3], {**env, e[1]: value}, state)
                return found
            found = set()

            def tries(position, at, count):
                out = set()
                for o in self.sub(e[3], {**env, e[1]: values[position]}, at):
                    if o[0] == 'F' and count + 1 < len(values):
                        out |= tries((position + 1) % len(values), o[1], count + 1)
                    else:
                        out.add(o)
                return out
            for start in range(len(values)):
                found |= tries(start, state, 0)
            return found
        if t == 'any':
            found = set()
            for order in itertools.permutations(e[1]):
                def tries(k, at):
                    if k == len(order):
                        return {('F', (at[0], nxt))}
                    out = set()
                    for o in self.sub(order[k], env, at):
                        if o[0] == 'F':
                            out |= tries(k + 1, (o[1][0], nxt))
                        else:
                            out.add(o)
                    return out
                found |= tries(0, state)
            return found
        if t == 'or':
            def tries(k, at):
                out = set()
                for o in self.sub(e[1][k], env, at):
                    if o[0] == 'F' and k + 1 < len(e[1]):
                        out |= tries(k + 1, o[1])
                    else:
                        out.add(o)
                return out
            return tries(0, state)
        if t == 'and':
            def tries(k, at):
                out = set()
                for o in self.sub(e[1][k], env, at):
                    if o[0] == 'T' and k + 1 < len(e[1]):
                        out |= tries(k + 1, o[1])
                    else:
                        out.add(o)
                return out
            return tries(0, state)
        if t == 'ite':
            return self.sub(e[2] if ev(e[1], env, cur) else e[3], env, state)
        if t == 'then':
            out = set()
            for o in self.sub(e[1], env, state):
                if o[0] == 'T':
                    out |= self.sub(e[2], env, shifted(o[1]))
                elif o[0] == 'F':
                    out.add(('E', 'QNT513'))
                else:
                    out.add(o)
            return out
        if t == 'reps':
            frontier = {('T', state)}
            out = set()
            for i in range(e[1]):
                nf = set()
                for o in frontier:
                    at = o[1] if i == 0 else shifted(o[1])
                    for r in self.sub(e[2], {**env, 'i': i}, at):
                        if r[0] == 'T':
                            nf.add(r)
                        elif r[0] == 'F':
                            out.add(('E', 'QNT513'))
                        else:
                            out.add(r)
                frontier = nf
            return out | frontier
        if t == 'expect':
            out = set()
            for o in self.sub(e[1], env, state):
                if o[0] == 'F':
                    out.add(('E', 'QNT508'))
                elif o[0] == 'E':
                    out.add(o)
                else:
                    after = shifted(o[1])
                    try:
                        holds = ev(e[2], env, after[0])
                    except Unset:
                        out.add(('E', 'QNT502'))
                        continue
                    out.add(('T', (after[0], o[1][1])) if holds else ('E', 'QNT508'))
            return out
        if t == 'fail':
            out = set()
            for o in self.sub(e[1], env, state):
                if o[0] == 'T':
                    out.add(('F', o[1]))
                elif o[0] == 'F':
                    out.add(('T', o[1]))
                else:
                    out.add(o)
            return out
        raise ValueError(e)


class Gen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.calls = {}
        self.count = 0

    def name(self):
        self.count += 1
        return f"v{self.count}"

    def arith(self, names, reads):
        r = self.r
        opts = [('lit', r.randint(0, 3))] + [('var', n) for n in names]
        if reads:
            opts += [('var', 'x'), ('var', 'y')]
        a = r.choice(opts)
        if r.random() < 0.4:
            a = ('bin', r.choice(['+', '-']), a, r.choice(opts))
        return a

    def cond(self, names, reads):
        return ('bin', self.r.choice(['<', '==', '!=', '>', '>=']),
                self.arith(names, reads), self.arith(names, reads))

    def size(self):
        return self.r.choice([1, 2, 3, 3, 4, 5, 99, 100])

    def action(self, names, reads):
        r = self.r
        if r.random() < 0.2:
            v = self.name()
            return ('nondet', v, ('range', ('lit', 0), ('lit', self.size() - 1)),
                    self.action(names + [v], reads))
        parts = []
        if r.random() < 0.08:
            parts.append(('assert', self.cond(names, reads)))
        if r.random() < 0.4:
            parts.append(('guard', self.cond(names, reads)))
        parts.append(('assign', 'x', self.arith(names, reads)))
        parts.append(('assign', 'y', self.arith(names, reads)))
        return ('all', parts)

    def first(self, names):
        r = self.r
        if r.random() < 0.4:
            return ('init',)
        if r.random() < 0.25 and len(self.calls) < 4:
            return self.called(names, inside=False)
        return self.action(names, reads=False)

    def called(self, names, inside):
        r = self.r
        key = f"c{len(self.calls)}"
        v = self.name()
        start = ('init',) if r.random() < 0.95 else self.action([v], reads=True)
        body = self.chain([v], start, depth=1, steps=r.randint(1, 2))
        self.calls[key] = ('nondet', v, ('range', ('lit', 0), ('lit', self.size() - 1)), body)
        return ('call', key)

    def chain(self, names, start, depth=0, steps=None):
        r = self.r
        e = start
        for _ in range(r.randint(0, 3) if steps is None else steps):
            k = r.random()
            if k < 0.25 and depth == 0 and len(self.calls) < 4:
                e = ('then', e, self.called(names, inside=True))
            elif k < 0.33:
                count = r.choice([0, 0, 1, 2])
                e = ('then', e, ('reps', count, self.action(names + ['i'], reads=True)))
            else:
                e = ('then', e, self.action(names, reads=True))
            if r.random() < 0.12:
                e = ('expect', e, self.cond(names, reads=True))
        if depth == 0 and r.random() < 0.1:
            e = ('fail', e)
        return e

    def top(self):
        r = self.r
        names = []
        wrap = []
        for _ in range(r.choice([0, 0, 1, 1, 2])):
            v = self.name()
            names.append(v)
            wrap.append((v, r.choice([1, 2, 3, 5, 100])))
        kind = r.choice(['chain', 'chain', 'ite', 'or', 'and', 'any'])
        if kind == 'chain':
            body = self.chain(names, self.first(names))
        elif kind == 'ite':
            body = ('ite', self.cond(names, reads=False),
                    self.chain(names, self.first(names)), self.chain(names, self.first(names)))
        else:
            parts = [self.chain(names, self.first(names)) for _ in range(r.choice([2, 2, 3]))]
            body = (kind, parts)
        for v, k in reversed(wrap):
            body = ('nondet', v, ('range', ('lit', 0), ('lit', k - 1)), body)
        return body


def rexp(e):
    t = e[0]
    if t == 'lit':
        return str(e[1])
    if t == 'var':
        return e[1]
    if t == 'bin':
        return f"({rexp(e[2])} {e[1]} {rexp(e[3])})"
    if t == 'range':
        return f"{rexp(e[1])}.to({rexp(e[2])})"
    raise ValueError(e)


def render(e):
    t = e[0]
    if t == 'all':
        out = []
        for p in e[1]:
            if p[0] == 'assert':
                out.append(f"assert({rexp(p[1])})")
            elif p[0] == 'guard':
                out.append(rexp(p[1]))
            else:
                out.append(f"{p[1]}' = {rexp(p[2])}")
        return "all { " + ", ".join(out) + " }"
    if t == 'init':
        return "init"
    if t == 'call':
        return e[1]
    if t == 'nondet':
        return f"(nondet {e[1]} = {rexp(e[2])}.oneOf() {render(e[3])})"
    if t == 'any':
        return "any { " + ", ".join(render(b) for b in e[1]) + " }"
    if t in ('or', 'and'):
        return t + " { " + ", ".join(render(b) for b in e[1]) + " }"
    if t == 'ite':
        return f"(if ({rexp(e[1])}) {render(e[2])} else {render(e[3])})"
    if t == 'then':
        return f"{render(e[1])}.then({render(e[2])})"
    if t == 'reps':
        return f"{e[1]}.reps(i => {render(e[2])})"
    if t == 'expect':
        return f"{render(e[1])}.expect({rexp(e[2])})"
    if t == 'fail':
        return f"{render(e[1])}.fail()"
    raise ValueError(e)


def main():
    seed = int(sys.argv[1])
    out = sys.argv[2]
    g = Gen(seed)
    init = ('all', [('assign', 'x', ('lit', 0)), ('assign', 'y', ('lit', 0))])
    runs = [g.top() for _ in range(2)]
    lines = [f"module h{seed} {{", "  var x: int", "  var y: int",
             f"  action init = {render(init)}"]
    for key, body in g.calls.items():
        lines.append(f"  run {key} = {render(body)}")
    oracle = Oracle({key: frozen(body) for key, body in {'init': init, **g.calls}.items()})
    expected = []
    for k, run in enumerate(runs):
        name = f"r{k}Test"
        lines.append(f"  run {name} = {render(run)}")
        found = oracle.run(frozen(run))
        fails = any(o[0] in ('F', 'E') for o in found)
        kinds = sorted({o[0] if o[0] != 'E' else o[1] for o in found})
        expected.append(f"{name} {'broken' if fails else 'holds'} {kinds}")
    lines.append("}")
    with open(f"{out}/h{seed}.qnt", "w") as written:
        written.write("/// @check\n" + "\n".join(lines) + "\n")
    with open(f"{out}/h{seed}.expect", "w") as written:
        written.write("\n".join(expected) + "\n")


if __name__ == '__main__':
    main()
