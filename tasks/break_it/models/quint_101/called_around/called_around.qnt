/// @check
module calledAround {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run fresh = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = x + n })
  run expectTest = init.then(fresh).expect(x == 3)
  run expectFalseTest = init.then(fresh).expect(x == 4)
  run expectInsideTest = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n }).expect(x == 3)
  run repsTest = init.then(3.reps(_ => fresh))
  run repsThenTest = init.then(2.reps(_ => fresh)).then(all { x == 3, x' = x })
  run failOneTest = init.then((nondet n = 0.to(4).oneOf() all { n == 7, x' = n }).fail())
  run assertTest = init.then(nondet n = 0.to(4).oneOf() init.then(all { assert(n < 9), n == 3, x' = n })).then(step)
  run assertFalseTest = init.then(nondet n = 0.to(4).oneOf() init.then(all { assert(n < 4), n == 4, x' = n })).then(step)
  run threeStart = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n })
  run never = nondet n = 0.to(4).oneOf() init.then(all { n == 9, x' = n })
  run failCallTest = init.then(threeStart).fail()
  run failNeverTest = init.then(never).fail()
  run failAnyTest = any { init.then(all { x == 1, x' = x }), init.then(all { x == 2, x' = x }) }.fail()
  run failAnyTrueTest = any { init.then(all { x == 1, x' = x }), init.then(all { x == 0, x' = x }) }.fail()
}
