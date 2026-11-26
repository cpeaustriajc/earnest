module div2 {
  var x: int
  action init = x' = -7
  action step = x' = x
  val truncDiv = x / 2 == -3
  val floorDiv = x / 2 == -4
  val modNeg = x % 2 == -1
  val modPos = x % 2 == 1
}
