/// @check
module assertFail {
  var x: int
  run assertFailTest = { nondet n = Set(0, 1, 2, 3).oneOf() all { assert(n < 2), x' = n }.fail() }
  run assertOnlyFailTest = all { assert(false), x' = 0 }.fail()
  run falseFailTest = all { 1 > 2, x' = 0 }.fail()
  run assertHoldsFailTest = all { assert(true), x' = 0 }.fail()
}
