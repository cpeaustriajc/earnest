// `assume` read the same way in a module checked directly and in a setup: a false one stops the
// check, as `quint verify --backend tlc` stops on it; a true one changes nothing.
/// @check
module Direct {
  var n: int
  assume Positive = 1 > 2
  action init = n' = 0
  action step = n' = n
  /// @check
  val ok = n >= 0
}

module Spec {
  const N: int
  var n: int
  assume Positive = N > 0
  action init = n' = N
  action step = all { n < 3, n' = n + 1 }
  /// @check
  val ok = n >= 0
  run upTest = init.then(step).expect(n == N + 1)
}

/// @check
module Bad {
  import Spec(N = 0).*
}

/// @check
module Good {
  import Spec(N = 1).*
}
