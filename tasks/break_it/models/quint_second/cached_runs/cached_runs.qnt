module cached_runs {
  var v: int
  action init = v' = 0
  action step = all { v < 2, v' = v + 1 }
  run upTest = init.then(step).expect(v == 1)
  run downTest = init.then(step).expect(v == 0)
}
