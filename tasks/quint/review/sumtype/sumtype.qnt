module Light {
  type Color = Red | Green
  var color: Color
  var ticks: int
  action init = all { color' = Red, ticks' = 0 }
  action step = all {
    ticks < 3,
    color' = match color { | Red => Green | Green => Red },
    ticks' = ticks + 1,
  }
  run twiceTest = init.then(step).then(step).expect(color == Green)
}

/// @check
module Setup {
  import Light.*
  /// @check
  val bounded = ticks <= 3
}
