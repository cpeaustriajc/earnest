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

  val below = n <= Max + Start
  val small = n < Max
  temporal reaches = weakFair(step, n) implies eventually(n == Max + Start)
}

module Low { import Spec(Start = 0, Max = 3).* }
module High { import Spec(Start = 2, Max = 5).* }
