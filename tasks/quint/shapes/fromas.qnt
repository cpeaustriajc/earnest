// The import shape `fromas`: the names `U` sees are the ones `quint compile --main U --invariant` accepts.
module U {
  import Far as F from "./far"
  var u: int
  action uInit = u' = 0
  action uStep = u' = u
}
