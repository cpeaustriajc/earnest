module runs {
  var x: int
  var y: int
  action init = all { x' = 0, y' = 0 }
  action step = all { x' = x + 1, y' = y }
  run thenRun = init.then(step).then(step)
  run expected = init.then(step).expect(x == 1)
  run repeated = init.then(3.reps(_ => step))
  run repeatedIndex = init.then(3.reps(i => all { x' = i, y' = y }))
  run failed = init.then(step.fail())
  run debugged = init.then(all { x' = q::debug("x", x), y' = y })
  run asserted = init.then(all { assert(x == 0), x' = x, y' = y })
  run chained = init.then(step).then(step).expect(x == 2).then(step)
  run lastTrace = init.then(step).expect(q::lastTrace.length() > 0)
}

module tests {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  val inv = x >= 0
  run tested = q::test(3, 2, 1, init, step, inv)
  run testedOnce = q::testOnce(3, 2, init, step, inv)
}
