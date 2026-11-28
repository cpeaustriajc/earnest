module asserts {
  var x: int
  var log: List[int]

  action init = all { assert(true), x' = 0, log' = [] }

  action up = all { x < 3, log.length() < 3, assert(x >= 0), x' = x + 1, log' = log.append(x) }
  action down = all { x > 0, x' = x - 1, assert(log.length() >= 0), log' = log }

  action step = any { up, down }

  val small = x <= 3
  val short = log.length() <= 2

  run upTest = init.then(up).then(up).expect(x == 2)
}

module broken {
  var x: int

  action init = x' = 0
  action step = all { assert(x < 2), x' = x + 1 }

  val small = x < 10

  run climbTest = init.then(step).then(step).then(step)
}
