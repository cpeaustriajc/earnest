// The import shape `instanceofinstanceexport`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module Param {
  const N: int
  var p: int
  val pOk = p <= N
}
module Mid {
  const M: int
  import Param(N = M) as B
  export B.*
}
module U {
  import Mid(M = 4).*
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
