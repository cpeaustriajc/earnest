module unstarted {
  var y: int
  action init = y' = 0
  action step = all { y < 2, y' = y + 1 }
  run noInitTest = step.then(step)
  run zeroTest = init.then(0.reps(_ => step))
  run failMidTest = init.then(step.fail()).then(step)
  run lateAssertTest = init.then(all { y < 5, assert(y == 0), step })
}
