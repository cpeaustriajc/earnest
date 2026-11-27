/// @check
module assertAfterNondet {
  var x: int

  action init = x' = 0

  run wholeTest = nondet n = 0.to(4).oneOf() assert(n >= 3)

  run andTest = nondet n = 0.to(4).oneOf() and { assert(n >= 3), n >= 0 }

  run valTest = nondet n = 0.to(4).oneOf() val m = n + 1 assert(m >= 4)

  run stepTest = nondet n = 0.to(4).oneOf() all { assert(n >= 3), x' = n }
}
