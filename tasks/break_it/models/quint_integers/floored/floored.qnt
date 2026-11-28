// `/` floors and `%` follows it, a == b * (a / b) + a % b, for all four signs, inside and past 64 bits.
/// @check rebuilt, between, literals, wideLiterals
module floored {
  var i: int
  var q: int
  var r: int

  pure val Wide = 18446744073709551617

  pure val Pairs = [
    (-7, 2), (7, -2), (-7, -2), (7, 2),
    (-Wide, 2), (Wide, -2), (-Wide, -2), (Wide, 2),
  ]

  val a = Pairs[i]._1
  val b = Pairs[i]._2

  action init = all {
    i' = 0,
    q' = Pairs[0]._1 / Pairs[0]._2,
    r' = Pairs[0]._1 % Pairs[0]._2,
  }

  action step = any {
    all {
      i < 7,
      i' = i + 1,
      q' = Pairs[i + 1]._1 / Pairs[i + 1]._2,
      r' = Pairs[i + 1]._1 % Pairs[i + 1]._2,
    },
    all { i == 7, i' = i, q' = q, r' = r },
  }

  val rebuilt = a == b * q + r and q == a / b and r == a % b
  val between = if (b > 0) 0 <= r and r < b else b < r and r <= 0
  val literals = all {
    (-7) / 2 == -4, 7 / (-2) == -4, (-7) / (-2) == 3, 7 / 2 == 3,
    (-7) % 2 == 1, 7 % (-2) == -1, (-7) % (-2) == -1, 7 % 2 == 1,
  }
  val wideLiterals = all {
    (-Wide) / 2 == -9223372036854775809, Wide / (-2) == -9223372036854775809,
    (-Wide) % 2 == 1, Wide % (-2) == -1, (-Wide) % (-2) == -1,
    (-7) / 18446744073709551616 == -1, (-7) % 18446744073709551616 == 18446744073709551609,
  }

  run flooredTest = init.then(step).expect(q == -4 and r == -1)
  run truncatedTest = init.expect(q == -3 and r == -1)
  run wideFlooredTest = init.then(4.reps(_ => step)).expect(q == -9223372036854775809 and r == 1)
  run positiveTest = init.then(3.reps(_ => step)).expect(q == 3 and r == 1)
}
