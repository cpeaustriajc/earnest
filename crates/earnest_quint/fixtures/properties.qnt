module properties {
  var x: int
  var on: bool
  action init = all { x' = 0, on' = false }
  action up = all { x < 2, x' = x + 1, on' = on }
  action flip = all { on' = not(on), x' = x }
  action move(d: int): bool = all { x + d <= 2, x' = x + d, on' = on }
  action aside = all { x' = 0, on' = on }
  action step = any { up, flip, move(1) }

  temporal reach = eventually(x == 2)
  temporal upFair = weakFair(up, (x, on)) implies eventually(x == 2)
  temporal leads = weakFair(step, (x, on)) implies (x == 0).leadsTo(x == 2)
  temporal spelled = weakFair(step, (x, on)) implies always(x == 0 implies eventually(x == 2))
  temporal often = (weakFair(step, (on, x)) and weakFair(flip, (x, on))) implies always(eventually(on))
  temporal safe = always(x <= 2)
  temporal mixed = weakFair(up, (x, on)) implies (always(x >= 0) and eventually(x == 2))
  temporal named = upFair
  temporal strong = strongFair(up, (x, on)) implies eventually(x == 2)
  temporal stays = eventually(always(x == 2))
  temporal negated = not(eventually(x == 2))
  temporal either = eventually(x == 2) or eventually(on)
  temporal nested = weakFair(step, (x, on)) implies (on implies eventually(x == 2))
  temporal some = weakFair(up, x) implies eventually(x == 2)
  temporal given = weakFair(move(1), (x, on)) implies eventually(x == 2)
  temporal outside = weakFair(aside, (x, on)) implies eventually(x == 2)
  temporal keep = always(step.orKeep((x, on)))
  temporal able = always(enabled(up))
}
