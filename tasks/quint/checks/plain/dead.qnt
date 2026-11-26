// A model that stops: no action is possible at a = 3.
module dead {
  var a: int

  action init = a' = 0

  action step = all {
    a < 3,
    a' = a + 1,
  }

  val ok = a <= 3
}
