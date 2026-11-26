// The import shape `instanceofinstance`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module Param {
  const N: int
  var p: int
  val pOk = p <= N
}
module Mid {
  const M: int
  import Param(N = M) as P
  val mOk = P::pOk
}
module U {
  import Mid(M = 4) as Q
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
