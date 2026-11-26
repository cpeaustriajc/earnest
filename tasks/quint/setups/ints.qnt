// An int constant in `init`, in a `nondet` set, in an invariant and in a temporal property.
module Spec {
  const Start: int
  const Max: int

  var n: int

  action init = n' = Start
  action step = {
    nondet d = 1.to(Max).oneOf()
    all { n + d <= Max + Start, n' = n + d }
  }

  /// @check

  val below = n <= Max + Start
  /// @check
  val small = n < Max
  /// @check
  temporal reaches = weakFair(step, n) implies eventually(n == Max + Start)
}

/// @check
module Low { import Spec(Start = 0, Max = 3).* }
/// @check
module High { import Spec(Start = 2, Max = 5).* }
