module fair2 {
  var t: int
  var h: bool
  action init = all { t' = 0, h' = false }
  action tick = all { t < 3, t' = t + 1, h' = h }
  action hear = all { not(h), h' = true, t' = t }
  action step = any { tick, hear }
  temporal hearFairOnly = weakFair(hear, (t, h)) implies eventually(t == 3)
  temporal tickFairOnly = weakFair(tick, (t, h)) implies eventually(t == 3)
  temporal bothFair = (weakFair(tick, (t, h)) and weakFair(hear, (t, h))) implies (eventually(t == 3) and eventually(h))
  temporal stepFairH = weakFair(step, (t, h)) implies eventually(h)
}
