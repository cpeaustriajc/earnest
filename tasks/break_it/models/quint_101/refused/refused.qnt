/// @check
module refusedShapes {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run toThree = nondet n = 0.to(4).oneOf() step.then(all { n == 3, x' = n })
  run readsTest = init.then(toThree).then(step)
  run threeStart = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n })
  run failCallTest = init.then(threeStart).fail()
  run ifReadsTest = init.then(if (x > 0) init.then(step) else init.then(step).then(step))
}
