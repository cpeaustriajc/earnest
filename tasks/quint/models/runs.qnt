// How quint test judges runs.
module runs {
  var y: int
  var picked: int
  action init = all { y' = 0, picked' = 0 }
  action inc = all { y < 2, y' = y + 1, picked' = picked }
  action pick = {
    nondet v = 1.to(3).oneOf()
    all { y' = y, picked' = v }
  }
  action step = any { inc, pick }

  // fail applied to a disabled action directly
  run failDirectTest = init.then(inc).then(inc).then(inc.fail())
  // fail on a whole chain whose last step is disabled
  run failChainTest = init.then(inc).then(inc).then(inc).fail()
  // fail on a reps chain
  run failRepsTest = init.then(3.reps(_ => inc)).fail()
  // then after a disabled action, no fail
  run disabledMidTest = init.then(inc).then(inc).then(inc).then(inc)
  // expect after nondet pick: true only for some choices
  run pickSomeTest = init.then(pick).expect(picked == 2)
  // expect true for all choices
  run pickAllTest = init.then(pick).expect(picked >= 1 and picked <= 3)
  // a run that ends without expect: passes if every step was enabled
  run plainTest = init.then(inc)
  // assert inside a run
  run assertFalseTest = init.then(all { assert(y == 1), inc })
  // run that is a plain boolean
  run boolTest = 1 + 1 == 2
  // init.then(step) where step's any picks a branch
  run stepTest = init.then(step).expect(y == 1)
  // nondet in the run itself
  run nondetRunTest = { nondet n = 1.to(2).oneOf()
    init.then(n.reps(_ => inc)).expect(y == n) }
  // reps index
  run repsIndexTest = init.then(2.reps(i => all { y == i, inc })).expect(y == 2)
  // empty oneOf in a run
  run emptyTest = { nondet n = Set().filter(x => x > 0).oneOf()
    init.then(all { y' = n, picked' = 0 }) }
}
