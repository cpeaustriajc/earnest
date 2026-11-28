// Sums, products and powers past 2^64: a counter that passes 2^63 and a power that doubles.
/// @check sumIsPositive, squareIsWide, powersAreOrdered, quotientIsExact, belowTwoToThe63Plus2
module bignum {
  var n: int
  var p: int

  pure val Top = 9223372036854775807

  action init = all {
    n' = Top - 1,
    p' = 2^64,
  }

  action step = any {
    all { n < Top + 3, n' = n + 1, p' = p * 2 },
    all { n >= Top + 3, n' = n, p' = p },
  }

  val sumIsPositive = n + n > 0
  val squareIsWide = p * p >= 2^128
  val powersAreOrdered = 2^70 > 2^64 and -(2^70) < -(2^64)
  val quotientIsExact = (p / 3) * 3 + p % 3 == p
  val belowTwoToThe63Plus2 = n < 9223372036854775809

  run passesTwoToThe63Test = init.then(step).then(step).then(step)
    .expect(n == 9223372036854775809 and p == 2^67)
  run wideProductTest = init.then(step)
    .expect(p * n == 2^65 * 9223372036854775807)
  run wrongPowerTest = init.then(step).expect(p == 2^64)
  run literalTest = init.expect(p == 18446744073709551616 and 100000000000000000000 > n)
}
