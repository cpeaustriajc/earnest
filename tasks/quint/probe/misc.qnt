module misc {
  var m: str -> int
  var t: int
  action init = all { m' = Map("a" -> 1), t' = 0 }
  action look = all { t < 2, m' = m, t' = t + m.get("b") }
  action tick = all { t < 3, m' = m, t' = t + 1 }
  action step = any { tick, look }
  val ok = t >= 0
  temporal onlyLookFair = weakFair(look, (m, t)) implies eventually(t == 3)
  temporal tickFair = weakFair(tick, (m, t)) implies eventually(t == 3)
  temporal safe = always(t <= 3)
  temporal evAlways = weakFair(step, (m, t)) implies eventually(always(t == 3))
  run getTest = init.then(look)
  val divs = (-7) / 2 == -4 and (-7) % 2 == 1
  val divs2 = (-7) / 2 == -3 and (-7) % 2 == -1
}
