/// @check
module nondetRuns {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  action guarded(k: int): bool = all { k < 3, x' = x + k }
  action chain2(k: int): bool = step.then(guarded(k))
  action pickSmall = { nondet m = Set(1, 2, 3).oneOf() all { m < 2, x' = x + m } }
  action pickAny = { nondet m = Set(1, 2, 3).oneOf() x' = x + m }
  run expectOneOfThreeTest = { nondet n = Set(1, 2, 3).oneOf() init.then(step).expect(x == n) }
  run thenGuardTest = { nondet n = Set(1, 2, 3).oneOf() init.then(all { n < 3, x' = n }) }
  run firstGuardThenTest = { nondet n = Set(1, 2, 3).oneOf() all { n < 3, x' = n }.then(step) }
  run firstGuardOnlyTest = { nondet n = Set(1, 2, 3).oneOf() all { n < 3, x' = n } }
  run allFalseTest = { nondet n = Set(1, 2, 3).oneOf() all { n > 5, x' = n } }
  run twoNestedTest = { nondet n = Set(1, 2, 3).oneOf() nondet k = Set(1, 2).oneOf() init.then(all { n + k == 4, x' = n }) }
  run twoNestedAllFalseTest = { nondet n = Set(1, 2, 3).oneOf() nondet k = Set(1, 2).oneOf() init.then(all { n + k == 9, x' = n }) }
  run twoNestedErrTest = { nondet n = Set(1, 2, 3).oneOf() nondet k = Set(1, 2).oneOf() init.then(step).expect(x == n * k) }
  run valTest = { nondet n = Set(1, 2, 3).oneOf() val m = n + 1 init.then(all { m < 4, x' = m }) }
  run repsLastTest = { nondet n = Set(1, 2, 3).oneOf() init.then(2.reps(_ => all { n < 3, x' = x + 1 })) }
  run repsThenTest = { nondet n = Set(1, 2, 3).oneOf() init.then(2.reps(_ => step)).then(all { n < 3, x' = n }) }
  run failTest = { nondet n = Set(1, 2, 3).oneOf() init.then(all { n < 3, x' = n }).fail() }
  run failAllTrueTest = { nondet n = Set(1, 2, 3).oneOf() init.then(all { n < 9, x' = n }).fail() }
  run failExpectTest = { nondet n = Set(1, 2, 3).oneOf() init.then(step).expect(x == 1).fail() }
  run helperTest = { nondet n = Set(1, 2, 3).oneOf() init.then(chain2(n)) }
  run helperMidTest = { nondet n = Set(1, 2, 3).oneOf() init.then(chain2(n)).then(step) }
  run calledTest = thenGuardTest
  run innerTopTest = { nondet n = Set(1, 2, 3).oneOf() init.then(pickAny).then(all { x + n == 4, x' = x }) }
  run innerTopFalseTest = { nondet n = Set(1, 2, 3).oneOf() init.then(pickSmall).then(all { n < 2, x' = x }) }
  run assertFirstTest = { nondet n = Set(1, 2, 3).oneOf() all { assert(n < 3), x' = n } }
  run emptyTest = { nondet n = Set().oneOf() init }
  run bigTest = { nondet n = 1.to(150).oneOf() init.then(all { n < 140, x' = n }) }
  run bigNestTest = { nondet n = 1.to(150).oneOf() nondet k = Set(1,2).oneOf() init.then(all { k < 2, x' = n }) }
  action startPick = { nondet m = Set(1, 2, 3).oneOf() x' = m }
  run failOnlyTest = { nondet n = Set(1, 2, 3).oneOf() all { n < 3, x' = n }.fail() }
  run failOnlyAllTest = { nondet n = Set(1, 2, 3).oneOf() all { n < 9, x' = n }.fail() }
  run pureTest = { nondet n = Set(1, 2, 3).oneOf() n > 2 }
  run pureFalseTest = { nondet n = Set(1, 2, 3).oneOf() n > 5 }
  run boolTest = { nondet b = Bool.oneOf() init.then(all { b, x' = 1 }) }
  run startMixTest = { nondet n = Set(1, 2, 3).oneOf() startPick.then(all { x == n, x' = x }) }
  run startMixOneTest = { nondet n = Set(1, 2, 3).oneOf() startPick.then(all { x >= 1, n == 2, x' = x }) }
  run expectLastTest = { nondet n = Set(1, 2, 3).oneOf() init.then(all { n < 3, x' = n }).expect(x < 3) }
  run threeStepTest = { nondet n = Set(1, 2, 3).oneOf() init.then(step).then(step).then(all { n == 2, x' = x }) }
  run repsBodyThenTest = { nondet n = Set(1, 2, 3).oneOf() init.then(2.reps(_ => step.then(all { n < 3, x' = x }))) }
  run nestedEmptyTest = { nondet n = Set(1, 2).oneOf() nondet k = (if (n == 1) Set() else Set(5)).oneOf() init.then(all { k == 5, x' = k }) }
}
