/// @check
module chains {
  var x: int
  var s: Set[int]
  action init = all { x' = 0, s' = 0.to(149) }
  action initSmall = all { x' = 0, s' = 0.to(49) }
  action step = all { x' = x + 1, s' = s }
  action pickSeven = nondet n = 0.to(199).oneOf() all { n == 7, x' = n, s' = s }
  action pickAny = nondet n = 0.to(199).oneOf() all { n >= 0, x' = n, s' = s }
  run twoSteps = init.then(pickAny)
  run pickBy(k: int): bool = step.then(nondet n = 0.to(199).oneOf() all { n == k, x' = n, s' = s })
  run stateSetTest = init.then(nondet n = s.oneOf() all { n == 7, x' = n, s' = s }).then(step)
  run stateSetSmallTest = initSmall.then(nondet n = s.oneOf() all { n == 7, x' = n, s' = s }).then(step)
  run repsTest = init.then(3.reps(i => pickSeven))
  run repsOkTest = init.then(3.reps(i => pickAny))
  run failSometimesTest = init.then(pickSeven).fail()
  run failNeverTest = init.then(nondet n = 0.to(199).oneOf() all { n == 700, x' = n, s' = s }).fail()
  run helperTest = twoSteps.then(pickSeven)
  run helperOkTest = twoSteps.then(step)
  run helperArgTest = init.then(pickBy(7)).then(step)
  run expectTest = init.then(nondet n = 0.to(199).oneOf() all { n < 150, x' = n, s' = s }).expect(x < 150)
  run retriedTopTest = nondet m = 0.to(2).oneOf() init.then(nondet n = 0.to(199).oneOf() all { n == 7, x' = n + m, s' = s })
  run retriedTopOkTest = nondet m = 0.to(2).oneOf() init.then(nondet n = 0.to(199).oneOf() all { m == 1 or n == 7, x' = n, s' = s })
  run retriedTopMidTest = nondet m = 0.to(2).oneOf() init.then(nondet n = 0.to(199).oneOf() all { m == 1 or n == 7, x' = n, s' = s }).then(step)
}
