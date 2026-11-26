// Each new way a mark is unreadable, reported at its line (#72).
// `Faults` is no check: `Uses` sets its constant.
module Faults {
  ///@check
  const N: int
  /// @check
  var x: int
  /// @check
  action init = x' = 0
  action step = x' = x
  /// @check
  /// @check broken
  val twice = x == 0
  /// @check ok, fine
  val listed = x == 0
  /// @check reachable
  temporal always0 = always(x == 0)
  /// @check
  def above(n: int): bool = x > n
  /// The level is fine.
  /// @check
  val later = x >= 0
}

/// @check
module Uses {
  /// @check
  import Faults(N = 1).*
}
