// A shape Quint 0.33.1's flattener stops on: an instance of an instance through `export`.
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
