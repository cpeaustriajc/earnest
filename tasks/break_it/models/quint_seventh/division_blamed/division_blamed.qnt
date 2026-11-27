/// @check
module divisionBlamed {
  var x: int

  action init = x' = 0

  action step = x' = x + 1

  run halfTest = (-7) / 2 == -4

  run pickTest = init.then(nondet n = 0.to(199).oneOf() all { n == 7, x' = n }).then(step)
}
