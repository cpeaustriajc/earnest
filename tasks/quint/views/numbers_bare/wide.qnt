/// @check
module wide {
  var n: int

  action init = n' = 9007199254740993
  action step = n' = if (n < 9007199254740995) n + 1 else 9007199254740993

  /// @check
  val inRange = n >= -9223372036854775807 and n <= 9223372036854775807

  val shown = if (n > 9007199254740992) { n: n, top: 9223372036854775807 } else { n: n, top: -9223372036854775807 }
}

/// @check
module wider {
  var m: int

  action init = m' = 18446744073709551617
  action step = m' = m

  /// @check
  val below = m < 36893488147419103233 and m < 9223372036854775808 and m > -18446744073709551617 and m != -9223372036854775808

  val shown = { m: m, top: 18446744073709551615 }
}
