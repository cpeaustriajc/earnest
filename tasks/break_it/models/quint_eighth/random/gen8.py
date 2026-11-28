#!/usr/bin/env python3
"""Random Quint runs and an exact oracle of `quint test`'s possible outcomes.

The oracle transcribes the rules of Quint 0.33.1's evaluator
(runtime/impl/nondet.js and builtins.js): a `nondet` over fewer than 100
values retries while its body is false, over 100 or more it keeps its pick;
`any` tries its branches in a random order until one is true; an error stops
everything; `then` errors (QNT513) on a false first part; `reps` errors on a
false round; `expect` errors on a false action or condition; `fail` negates;
`assert` errors when false. Each evaluation returns the SET of outcomes some
sample can reach: ('T', state) | ('F',) | ('E', code). A test fails when
some sample ends in F or E.

usage: gen8.py SEED OUTDIR   -> writes OUTDIR/gSEED.qnt and OUTDIR/gSEED.expect
"""
import random
import sys

BIG = 100
F = ('F',)


def E(code):
    return ('E', code)


class Gen:
    def __init__(self, seed):
        self.r = random.Random(seed)
        self.n = 0
        self.nostate = False

    def fresh(self):
        self.n += 1
        return f"v{self.n}"

    def term(self, names):
        r = self.r
        opts = [('lit', r.randint(0, 3))]
        if not self.nostate:
            opts += [('var', 'x'), ('var', 'y')]
        opts += [('var', n) for n in names]
        return r.choice(opts)

    def arith(self, names, depth=0):
        r = self.r
        if depth > 1 or r.random() < 0.5:
            return self.term(names)
        op = r.choice(['+', '-', '+', '%'])
        if op == '%':
            return ('bin', '%', self.arith(names, depth + 1), ('lit', r.choice([2, 3])))
        return ('bin', op, self.arith(names, depth + 1), self.arith(names, depth + 1))

    def cond(self, names):
        r = self.r
        op = r.choice(['<', '<=', '==', '!=', '>', '>='])
        c = ('bin', op, self.arith(names), self.arith(names))
        if r.random() < 0.15:
            c = ('bin', r.choice(['and', 'or']), c, self.cond(names))
        return c

    def theset(self, names):
        r = self.r
        k = r.choice([1, 2, 3, 4, 2, 3, 98, 99, 100, 120, 149])
        lo = r.choice([('lit', 0), ('lit', 0)] + ([] if self.nostate else [('var', 'x')]) + [('var', n) for n in names[-1:]])
        if r.random() < 0.3:
            return ('filter', lo, r.choice([0, 1, 2, 3, 120]), self.r.choice([2, 3]), r.randint(0, 1))
        return ('range', lo, ('bin', '+', lo, ('lit', k)))

    def action(self, names, depth=0, first=False):
        r = self.r
        p = r.random()
        if depth < 3 and p < 0.3:
            v = self.fresh()
            return ('nondet', v, self.theset(names), self.action(names + [v], depth + 1, first))
        if depth < 3 and p < 0.42:
            return ('any', [self.action(names, depth + 1, first) for _ in range(r.randint(2, 3))])
        if depth < 3 and p < 0.52:
            return ('ite', self.cond(names), self.action(names, depth + 1, first),
                    self.action(names, depth + 1, first))
        parts = []
        if r.random() < 0.12:
            parts.append(('assert', self.cond(names)))
        if r.random() < 0.6:
            parts.append(('guard', self.cond(names)))
        parts.append(('assign', 'x', ('lit', r.randint(0, 3)) if first else self.arith(names)))
        parts.append(('assign', 'y', ('lit', r.randint(0, 2)) if first else self.arith(names)))
        return ('all', parts)

    def chain(self, names):
        r = self.r
        self.nostate = True
        steps = [('first', self.action(names, first=True) if r.random() < 0.5 else ('init',))]
        self.nostate = False
        for _ in range(r.randint(0, 3)):
            k = r.random()
            if k < 0.15:
                steps.append(('reps', r.randint(1, 3), self.action(names + ['i'])))
            else:
                steps.append(('then', self.action(names)))
            if r.random() < 0.2:
                steps.append(('expect', self.cond(names)))
        if r.random() < 0.15 and steps[-1][0] == 'then':
            steps.append(('fail',))
        return steps

    def run(self):
        r = self.r
        tops = []
        names = []
        for _ in range(r.choice([0, 0, 1, 1, 2])):
            v = self.fresh()
            k = r.choice([1, 2, 3, 4, 120, 99, 100])
            tops.append((v, k))
            names.append(v)
        if len(tops) == 2 and tops[0][1] * tops[1][1] > 900:
            tops = tops[:1]
            names = names[:1]
        return (tops, self.chain(names))

    def init(self):
        if self.r.random() < 0.3:
            k = self.r.choice([2, 3, 120])
            return ('nondet', 'a0', ('range', ('lit', 0), ('lit', k)),
                    ('all', [('assign', 'x', ('var', 'a0')), ('assign', 'y', ('lit', 0))]))
        return ('all', [('assign', 'x', ('lit', 0)), ('assign', 'y', ('lit', 0))])


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
    if t == 'filter':
        return f"{rexp(e[1])}.to({rexp(e[1])} + {e[2]}).filter(w => w % {e[3]} == {e[4]})"
    raise ValueError(e)


def ract(a):
    t = a[0]
    if t == 'nondet':
        return f"nondet {a[1]} = {rexp(a[2])}.oneOf() {ract(a[3])}"
    if t == 'any':
        return "any { " + ", ".join(ract(b) for b in a[1]) + " }"
    if t == 'ite':
        return f"if ({rexp(a[1])}) {ract(a[2])} else {ract(a[3])}"
    if t == 'all':
        out = []
        for p in a[1]:
            if p[0] == 'assert':
                out.append(f"assert({rexp(p[1])})")
            elif p[0] == 'guard':
                out.append(rexp(p[1]))
            else:
                out.append(f"{p[1]}' = {rexp(p[2])}")
        return "all { " + ", ".join(out) + " }"
    if t == 'init':
        return "init"
    raise ValueError(a)


def rrun(run):
    tops, steps = run
    s = ""
    for st in steps:
        if st[0] == 'first':
            s = ract(st[1])
            if st[1][0] in ('nondet', 'ite') and len(steps) > 1:
                s = "{ " + s + " }"

        elif st[0] == 'then':
            s = f"{s}.then({ract(st[1])})"
        elif st[0] == 'reps':
            s = f"{s}.then({st[1]}.reps(i => {ract(st[2])}))"
        elif st[0] == 'expect':
            s = f"{s}.expect({rexp(st[1])})"
        elif st[0] == 'fail':
            s = f"{s}.fail()"
    pre = "".join(f"nondet {v} = 0.to({k - 1}).oneOf() " for v, k in tops)
    return pre + s


def ev(e, env):
    t = e[0]
    if t == 'lit':
        return e[1]
    if t == 'var':
        return env[e[1]]
    if t == 'bin':
        op = e[1]
        if op == 'and':
            return ev(e[2], env) and ev(e[3], env)
        if op == 'or':
            return ev(e[2], env) or ev(e[3], env)
        a, b = ev(e[2], env), ev(e[3], env)
        if op == '%':
            return a % b
        return {'+': a + b, '-': a - b, '<': a < b, '<=': a <= b, '==': a == b,
                '!=': a != b, '>': a > b, '>=': a >= b}[op]
    if t == 'range':
        return list(range(ev(e[1], env), ev(e[2], env) + 1))
    if t == 'filter':
        lo = ev(e[1], env)
        return [w for w in range(lo, lo + e[2] + 1) if w % e[3] == e[4]]
    raise ValueError(e)


def retry(values, body):
    """Outcomes of a nondet over `values` whose body gives the outcome set body(v)."""
    if not values:
        return {F}
    outs = [body(v) for v in values]
    if len(values) >= BIG:
        return set().union(*outs)
    res = set()
    for o in outs:
        res |= {x for x in o if x != F}
    if all(F in o for o in outs):
        res.add(F)
    return res


def act(a, env):
    """Outcome set of action a from env (holds x, y and bound names)."""
    t = a[0]
    if t == 'nondet':
        vals = ev(a[2], env)
        return retry(vals, lambda v: act(a[3], {**env, a[1]: v}))
    if t == 'any':
        bs = [act(b, env) for b in a[1]]
        res = set()
        import itertools
        for perm in itertools.permutations(range(len(bs))):
            def go(k):
                if k == len(perm):
                    return {F}
                o = bs[perm[k]]
                r = {x for x in o if x != F}
                if F in o:
                    r |= go(k + 1)
                return r
            res |= go(0)
        return res
    if t == 'ite':
        return act(a[2] if ev(a[1], env) else a[3], env)
    if t == 'all':
        nxt = {}
        for p in a[1]:
            if p[0] == 'assert':
                if not ev(p[1], env):
                    return {E('QNT508')}
            elif p[0] == 'guard':
                if not ev(p[1], env):
                    return {F}
            else:
                nxt[p[1]] = ev(p[2], env)
        return {('T', (nxt['x'], nxt['y']))}
    if t == 'init':
        return act(env['__init'], env)
    raise ValueError(a)


def play(steps, env):
    """Outcome set of a left-nested chain, from no state."""
    outs = act(steps[0][1], env)
    for st in steps[1:]:
        nxt = set()
        for o in outs:
            if st[0] == 'fail':
                nxt.add(F if o[0] == 'T' else (('T', None) if o == F else o))
                continue
            if o[0] == 'E':
                nxt.add(o)
                continue
            if o == F:
                nxt.add(E('QNT508') if st[0] == 'expect' else E('QNT513'))
                continue
            s = o[1]
            if st[0] == 'expect':
                nxt.add(o if ev(st[1], at(env, s)) else E('QNT508'))
            elif st[0] == 'then':
                nxt |= act(st[1], at(env, s))
            elif st[0] == 'reps':
                frontier = {s}
                for i in range(st[1]):
                    nf = set()
                    for cs in frontier:
                        for r in act(st[2], {**at(env, cs), 'i': i}):
                            if r[0] == 'T':
                                nf.add(r[1])
                            elif r == F:
                                nxt.add(E('QNT513'))
                            else:
                                nxt.add(r)
                    frontier = nf
                nxt |= {('T', cs) for cs in frontier}
        outs = nxt
    return outs


def at(env, s):
    return {**env, 'x': s[0], 'y': s[1]}


def run_outcomes(run, initact):
    tops, steps = run
    env = {'__init': initact}
    def body(env):
        return play(steps, env)

    def level(k, env):
        if k == len(tops):
            return body(env)
        v, n = tops[k]
        return retry(list(range(n)), lambda val: level(k + 1, {**env, v: val}))
    return level(0, env)


def main():
    seed = int(sys.argv[1])
    out = sys.argv[2]
    g = Gen(seed)
    initact = g.init()
    runs = [g.run() for _ in range(4)]
    lines = [f"module g{seed} {{", "  var x: int", "  var y: int",
             f"  action init = {ract(initact)}",
             "  action step = all { x' = x + 1, y' = y }"]
    exp = []
    for k, run in enumerate(runs):
        name = f"r{k}Test"
        lines.append(f"  run {name} = {rrun(run)}")
        try:
            o = run_outcomes(run, initact)
            fails = any(x[0] in ('F', 'E') for x in o)
            exp.append(f"{name} {'broken' if fails else 'holds'} {sorted(set(x[0] if x[0] != 'E' else x[1] for x in o))}")
        except RecursionError:
            exp.append(f"{name} ?")
    lines.append("}")
    open(f"{out}/g{seed}.qnt", "w").write("/// @check\n" + "\n".join(lines) + "\n")
    open(f"{out}/g{seed}.expect", "w").write("\n".join(exp) + "\n")


if __name__ == '__main__':
    main()
