module runplace {
  var x: int

  action init = x' = 0
  action step = x' = x + 1

  val small = x < 5

  run fromNowhereTest = step.then(step)
  run lateFailTest = init.then(step).fail().then(step)
}
