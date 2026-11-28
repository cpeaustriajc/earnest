/// @check ok, later, sooner
module dead {
  var a: int

  action init = a' = 0

  action step = all {
    a < 3,
    a' = a + 1,
  }

  val ok = a <= 3
  temporal later = eventually(a == 3)
  temporal sooner = eventually(a >= 1)
}
