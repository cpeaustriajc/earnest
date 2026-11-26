module Counter {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 4
  /// @check
  val inRange = n < 4
  /// @check broken
  run skips = init.then(step).expect(n == 2)
}

/// @check
module Plain {
  import Counter.*
}
