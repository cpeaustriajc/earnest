// A shape Quint 0.33.1's flattener stops on: `import X as A` inside a spec that is instantiated.
module Helpers {
  pure def twice(v: int): int = v + v
}
module Spec {
  import Helpers as H
  const K: int
  var s: int
  action init = s' = H::twice(K)
  action step = s' = s
  val sOk = s >= 0
}
module Setup {
  import Spec(K = 1).*
}
