module Debug {
  val shown = q::debug("value", 1 + 2)
  val one = q::debug(3)
  val dotted = 4.q::debug()
}
