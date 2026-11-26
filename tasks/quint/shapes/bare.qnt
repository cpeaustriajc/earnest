// The import shape `bare`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module Base {
  var b: int
  val bOk = b >= 0
  val bTwo = b != 2
}
module U {
  import Base
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
