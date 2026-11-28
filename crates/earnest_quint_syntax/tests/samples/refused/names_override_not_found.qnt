module Lib {
  const C: int
}

module OverrideNotFound {
  import Lib(C = 1, D = 2) as L
}
