module bare {
  var x: int
  action init = x' = 0
  action step = any { all { x < 2, x' = x + 1 }, x' = x }
  temporal fairAlone = weakFair(step, x)
  temporal onceThen = eventually(x == 2) implies always(x <= 2)
}
