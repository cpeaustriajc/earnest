// Setups the typecheck build leaves to `quint compile`: one name imported alone into the spec, a
// spec that imports a module with variables, and a module imported by the setup both whole and
// under a name.
module Lib {
  pure def twice(x: int): int = 2 * x
  pure def thrice(x: int): int = 3 * x
}

module Counter {
  var c: int
  action tick = c' = c + 1
}

module OneName {
  import Lib.twice
  const K: int
  var n: int
  action init = n' = twice(K)
  action step = n' = n
  val ok = n >= 0
}

module WithState {
  import Counter.*
  const K: int
  var n: int
  action init = all { n' = K, c' = 0 }
  action step = all { n' = n, tick }
  val ok = n >= 0
}

module Single {
  import OneName(K = 1).*
}

module Stateful {
  import WithState(K = 1).*
}

module Twice {
  import Lib.*
  import Lib as L
  import OneName(K = 1).*
}
