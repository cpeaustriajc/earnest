// The import shape `instance`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module Param {
  const N: int
  var p: int
  val pOk = p <= N
}
module U {
  import Param(N = 3).*
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
