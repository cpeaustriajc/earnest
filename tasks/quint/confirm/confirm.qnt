// The samples of `earnest confirm --quint` (issue #73).
module lift {
  var floor: int

  action init = floor' = 0
  action up = all { floor < 3, floor' = floor + 1 }
  action down = all { floor > 0, floor' = floor - 1 }
  action pick = {
    nondet level = 0.to(1).oneOf()
    floor' = level
  }
  action step = any { up, down }

  val inside = floor >= 0 and floor <= 3
  val low = floor < 2
  temporal reachesTop = eventually(floor == 3)

  run upTest = init.then(up).expect(floor == 1)
  run pickTest = init.then(pick).expect(floor == 1)
}

module big {
  pure val Big = 4294967296
  var n: int

  action init = n' = 0
  action step = all { n < 2, n' = n + 1 }

  val belowBig = n < Big
}

module halves {
  var n: int

  action init = n' = -3
  action step = all { n < 0, n' = n + 1 }

  val halfAbove = n / 2 >= -2
}

module ranges {
  var n: int

  action init = n' = 0
  action step = n' = n

  run emptyTest = init.expect(range(3, 1).length() == 0)
}

module words {
  var n: int

  action init = n' = 0
  action step = all { n < 2, n' = n + 1 }

  val counted = Set("up", "down").fold(n, (total, word) => total + 1) <= 4
}
