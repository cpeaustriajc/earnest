/// @check small
module starts {
  var a: int
  var b: bool

  action init = {
    nondet v = 0.to(2).oneOf()
    nondet w = Set(true, false).oneOf()
    all {
      a' = v * 2,
      b' = w,
    }
  }

  action step = all {
    a < 6,
    a' = a + 1,
    b' = b,
  }

  val small = a <= 6
}
