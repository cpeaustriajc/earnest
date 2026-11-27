/// @check
module bigPick {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run midPickTest = init.then(nondet n = 0.to(199).oneOf() all { n == 7, x' = n }).then(step)
  run lastBigTest = init.then(nondet n = 0.to(199).oneOf() all { n == 7, x' = n })
  run firstBigTest = (nondet n = 0.to(199).oneOf() all { n == 7, x' = n }).then(step)
  run smallPickTest = init.then(nondet n = 0.to(9).oneOf() all { n == 7, x' = n }).then(step)
}
