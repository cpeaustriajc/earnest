module start_implies {
  var x: int
  action init = x' = 0
  action step = any { all { x < 3, x' = x + 1 }, x' = x }
  temporal fromZero = (x == 0).implies(eventually(x == 3))
  temporal fromOne = (x == 1).implies(eventually(x == 3))
  temporal fairFromZero = weakFair(step, x).implies((x == 0).implies(eventually(x == 3)))
}
