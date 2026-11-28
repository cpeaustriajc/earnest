/// @check inBounds, downAtZero, agree, neverFourDown
module mixed {
  pure val Top = 4

  var x: int
  var up: bool

  action init = all {
    x' = 0,
    up' = true,
  }

  action move(d: int): bool = {
    val moved = x + d
    all {
      moved >= 0,
      moved <= Top,
      x' = moved,
      up' = d > 0,
    }
  }

  action flip = all {
    if (up) x' = x else x' = 0,
    up' = not(up),
  }

  action wrap = all {
    x == Top,
    if (up) x' = 0 else x' = x,
    up' = up,
  }

  action step = any {
    { nondet d = Set(-1, 1, 2).oneOf() move(d) },
    flip,
    wrap,
  }

  val inBounds = x >= 0 and x <= Top
  val downAtZero = (x == 0 and not(up)) implies x < 1
  val agree = (x > Top) iff false
  val neverFourDown = not(x == 4 and not(up))
}
