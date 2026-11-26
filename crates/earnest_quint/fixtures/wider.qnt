module wider {
  var x: int
  var flag: bool

  pure def small(v: int): bool = all { v >= 0, v <= 4 }
  val Sane = all {
    small(x),
    any { flag, not(flag) },
    0.to(4).forall(i => all { i >= 0, any { i <= 4, i > 9 } }),
  }
  val Few = 0.to(4).filter(i => all { i <= x, any { i == 0, i > 2 } }).size() <= 2
  val Low = any { x < 4, flag }

  action between(changes: bool): bool = all { x < 4, x' = x + 1, changes }
  action outer(changes: bool): bool = between(changes)
  action either(changes: bool): bool = if (flag) all { x' = x, changes } else between(changes)
  action plus(k: int, changes: bool): bool = all { x + k <= 4, x' = x + k, changes }
  action caller(k: int): bool = plus(k, all { flag' = k > 1 })
  action each(f: int => bool): bool = all { x < 4, x' = x + 1, f(x) }

  action init = all { x' = 0, flag' = false }
  action step = any {
    outer(all { flag' = true }),
    either(flag' = not(flag)),
    caller(2),
    outer({ nondet v = Set(false, true).oneOf() flag' = v }),
    each(v => all { flag' = v >= 2 }),
    all { x' = 0, flag' = false },
  }

  run moveTest = init
    .then(between(all { flag' = true }))
    .then(outer(flag' = false))
    .expect(all { x == 2, not(flag) })
  run wrongTest = init
    .then(caller(2))
    .expect(any { x == 1, not(flag) })
}
