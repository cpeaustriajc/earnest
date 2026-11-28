///@check
module A {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run upTest = init.then(step).then(all { assert(x == 5), step })
}
/// @check
module B {
  var y: int
  action init = y' = 0
  action step = if (y < 3) y' = y + 1 else y' = y
  ///@check broken
  val wraps = y < 100
  /// @check
  val nonneg = y >= 0
}
