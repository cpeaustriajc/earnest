// A variable reached through two routes, each updating it: Quint's effect pass refuses it (QNT202) before its flattener runs.
module Spec {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
}
module W0 {
  import Spec.*
  import Spec as S
  action both = all { x' = 1, S::x' = 2 }
}
