module constant_false {
  var n: int
  action init = n' = 0
  action step = n' = n
  val never = Set(1, 2).size() == 3
}
