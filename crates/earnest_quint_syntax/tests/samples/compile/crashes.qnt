// The two shapes Quint 0.33.1's flattener stops on: an instance of an instance through `export`, and `import X as A` inside a spec that is instantiated.
module Param {
  const N: int
  var p: int
  action init = p' = N
  action step = p' = p
  val pOk = p <= N
}
module Mid {
  const M: int
  import Param(N = M) as B
  export B.*
}
module Through {
  import Mid(M = 4).*
}
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
