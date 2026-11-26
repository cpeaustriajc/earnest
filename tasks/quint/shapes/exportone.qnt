// The import shape `exportone`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module Base {
  var b: int
  val bOk = b >= 0
  val bTwo = b != 2
}
module Mid {
  import Base.*
  export Base.*
  val midOwn = b > 1
}
module U {
  import Mid.bOk
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
