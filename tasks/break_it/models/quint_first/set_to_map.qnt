module set_to_map {
  var n: int
  action init = n' = 0
  action step = all { n < 1, n' = n + 1 }
  val firstKept = Set((1, 3), (1, 2)).setToMap() == Map(1 -> 2)
}
