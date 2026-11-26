// The import shape `instanceexporttwice`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module Param {
  const N: int
  var p: int
  val pOk = p <= N
}
module Mid {
  import Param(N = 3) as B
  export B.*
}
module Top {
  import Mid.*
  export Mid.*
}
module U {
  import Top.*
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
