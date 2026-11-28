module fair_part {
  var x: int
  var t: int
  action init = all { x' = 0, t' = 0 }
  action move = all { x < 3, x' = x + 1 }
  action rest = x' = x
  action tick = all { t < 1, t' = t + 1 }
  action step = all { any { move, rest }, tick }
  temporal reachesThree = weakFair(move, (x, t)) implies eventually(x == 3)
}
