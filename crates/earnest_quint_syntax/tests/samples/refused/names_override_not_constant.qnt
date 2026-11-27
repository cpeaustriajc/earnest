module Lib {
  const C: int
  val notConstant = 1
}

module OverrideNotConstant {
  import Lib(C = 1, notConstant = 2) as L
}
