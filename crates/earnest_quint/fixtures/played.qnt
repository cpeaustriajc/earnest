// What quint test does with run operators beyond the spike's runs.qnt: each run's verdict is in
// tasks/quint/README.md, "Slice 6: what is checked".
module played {
  var y: int
  var z: int
  action init = all { y' = 0, z' = 0 }
  action inc = all { y < 2, y' = y + 1, z' = z }
  action add(k: int): bool = all { y' = y + k, z' = z }
  action gated = { nondet v = 1.to(3).oneOf() all { v == 2, y' = v, z' = z } }
  action eitherGated = any { gated, all { y' = 7, z' = z } }
  action step = any { inc, gated }
  run expectAfterFalseTest = init.then(inc).then(inc).then(inc).expect(y == 2)
  run failExpectFalseTest = init.then(inc).then(inc).then(inc).expect(y == 2).fail()
  run failExpectTest = init.then(inc).expect(y == 5).fail()
  run failExpectTrueTest = init.then(inc).expect(y == 1).fail()
  run midExpectTest = init.then(inc).expect(y == 1).then(inc).expect(y == 2)
  run gatedTest = init.then(gated)
  run eitherTest = init.then(eitherGated)
  run assertFirstTest = init.then(all { assert(false), y > 5, inc }).fail()
  run argStateTest = init.then(inc).then(add(y)).expect(y == 2)
  run initMidTest = init.then(inc).then(init).expect(y == 0)
  run nondetArgTest = { nondet k = 1.to(2).oneOf() init.then(add(k)).expect(y == k) }
  run failCanTest = init.then(inc.fail())
  run repsThenTest = init.then(2.reps(_ => inc.then(inc))).expect(y == 2)
  run initExpectTest = init.expect(y == 0).then(inc)
  run nestedExpectTest = init.then(inc.expect(y == 1)).then(inc)
  run falseBoolTest = 1 + 1 == 3
  run topBoolTest = { nondet n = 1.to(3).oneOf() n == 2 }
  run topErrorTest = { nondet n = 1.to(2).oneOf() init.then(inc).expect(y == n) }
  run failOnlyTest = init.then(inc).then(inc).fail()
  run anyRunsTest = init.then(any { inc, all { y > 10, y' = 0, z' = z } }).expect(y == 1)
  run valTest = { val two = 2  init.then(two.reps(_ => inc)).expect(y == two) }
  run calledTest = midExpectTest.then(inc.fail())
  run notTested = init.then(inc).then(inc).then(inc)
}
