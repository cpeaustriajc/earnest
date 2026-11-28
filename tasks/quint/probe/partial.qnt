module partial {
  var a: int
  var b: int
  action init = all { a' = 0, b' = 0 }
  action step = a' = a + 1
  val ok = a < 3
}
