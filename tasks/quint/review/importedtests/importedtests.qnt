module Counter {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 4
  val inRange = n < 4
  run upTest = init.then(step).expect(n == 1)
  run badTest = init.then(step).expect(n == 2)
}

/// @check inRange
module Plain {
  import Counter.*
}
