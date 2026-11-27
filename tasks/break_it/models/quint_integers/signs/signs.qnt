// `/` floors and `%` follows it for all four signs, where TLC refuses `%` by a negative number.
/// @check rebuilt, literals
module signs {
  var a: int
  var b: int

  action init = all { a' = -7, b' = 2 }

  action step = any {
    all { a == -7, b == 2, a' = 7, b' = -2 },
    all { a == 7, b == -2, a' = -7, b' = -2 },
    all { a == -7, b == -2, a' = 7, b' = 2 },
    all { a == 7, b == 2, a' = a, b' = b },
  }

  val rebuilt = a == b * (a / b) + a % b
  val literals = all {
    (-7) / 2 == -4, 7 / (-2) == -4, (-7) / (-2) == 3,
    (-7) % 2 == 1, 7 % (-2) == -1, (-7) % (-2) == -1,
  }
}
