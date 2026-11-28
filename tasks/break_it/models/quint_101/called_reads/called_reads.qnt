/// @check
module calledReads {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run toThree = nondet n = 0.to(4).oneOf() step.then(all { n == 3, x' = n })
  run readsTest = init.then(toThree).then(step)
}
