/// @check
module noStart {
  var x: int
  action init = all { 1 > 2, x' = 0 }
  action never = all { 1 > 2, x' = 0 }
  action start = x' = 0
  action step = x' = x + 1
  /// @check
  val ok = x >= 0
  run initFailsTest = init.fail()
  run initThenFailsTest = init.then(step).fail()
  run neverFailsTest = never.fail()
  run neverThenFailsTest = never.then(step).fail()
  run initThenRefusedTest = init.then(step.fail())
  run neverThenRefusedTest = never.then(step.fail())
  run pickedThenTest = { nondet n = 0.to(9).oneOf() init.then(x' = n) }
  run pickedFailsTest = { nondet n = 0.to(9).oneOf() init.fail() }
  run expectTest = init.expect(x == 0)
  run laterInitTest = start.then(init)
  run laterInitRefusedTest = start.then(init.fail())
  run laterInitFailsTest = start.then(init).fail()
  run repeatedTest = 2.reps(_ => init)
  run guardedTest = { nondet n = 0.to(9).oneOf() all { n > 3, init } }
  run ownStartTest = start.then(step)
  run falseTest = false
  run initAloneTest = init
  run bigPickInitTest = { nondet n = 0.to(119).oneOf() init }
  run bigPickConstantTest = { nondet n = 0.to(119).oneOf() n != 5 }
  run smallPickConstantTest = { nondet n = 0.to(9).oneOf() n != 5 }
  run smallPickNeverTest = { nondet n = 0.to(9).oneOf() n > 50 }
}
