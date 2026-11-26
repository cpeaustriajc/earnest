module initpick {
  var x: int
  action init = {
    nondet a = 1.to(2).oneOf()
    nondet b = 0.to(a).oneOf()
    x' = b
  }
  action step = x' = x
  val small = x <= 1
}
