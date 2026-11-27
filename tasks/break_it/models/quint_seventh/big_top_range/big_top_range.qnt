/// @check
module bigTopRange {
  var x: int

  action init = x' = 0

  run pastTest = nondet n = (2 ^ 63 - 1).to(2 ^ 63 + 1).oneOf() init.expect(x <= n)

  run writtenTest = nondet n = (9223372036854775807).to(9223372036854775808).oneOf() init.expect(x <= n)

  run belowTest = nondet n = (2 ^ 62).to(2 ^ 62 + 1).oneOf() init.expect(x <= n)

  run inStepTest = init.then(nondet n = (2 ^ 64).to(2 ^ 64 + 1).oneOf() x' = n).expect(x >= 2 ^ 64)
}
