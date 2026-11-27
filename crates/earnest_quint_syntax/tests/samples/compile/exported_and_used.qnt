module Lib {
  pure val c = 3
}

module Mid {
  import Lib.*
  export Lib.*
}

module Other {
  import Lib.*
  pure val top = c
}

module Main {
  import Mid.*
  var s: int
  action init = s' = c
  action step = s' = s
}
