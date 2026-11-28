/// @check
module zeroReps {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run zeroTest = init.then(0.reps(_ => step))
  run zeroAloneTest = 0.reps(_ => step)
  run zeroThenInitTest = init.then(0.reps(_ => step)).then(init).then(step)
  run zeroExpectConstTest = init.then(0.reps(_ => step)).expect(1 > 0)
  run zeroExpectFalseTest = init.then(0.reps(_ => step)).expect(1 > 2)
  run zeroFailTest = init.then(0.reps(_ => step)).fail()
  run zeroFirstTest = 0.reps(_ => step).then(init).then(step)
  run negTest = init.then((-2).reps(_ => step)).then(init)
  run zeroNondetTest = nondet n = 0.to(2).oneOf() init.then(n.reps(_ => step)).then(init).then(all { n == 2, x' = x })
  run zeroReadsTest = init.then(0.reps(_ => step)).then(step)
  run zeroExpectTest = init.then(0.reps(_ => step)).expect(x == 0)
  run zeroCalledTest = init.then(nondet n = 0.to(2).oneOf() init.then(0.reps(_ => step)).then(all { n == 1, x' = x })).then(step)
}
