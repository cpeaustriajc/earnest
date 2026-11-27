// What `quint compile` adds for a check: `q::init`, `q::step`, `q::inv` of one invariant, of several and of an expression, and `q::temporalProps`.
module Lamp {
  var on: bool
  var n: int
  action init = all { on' = false, n' = 0 }
  action flip = all { on' = not(on), n' = n + 1 }
  action step = flip
  val small = n < 10
  val known = on or not(on)
  temporal flips = always(eventually(on))
  temporal counts = always(n >= 0)
}
module Dim {
  import Lamp as L
  action start = L::init
  action go = L::step
  val dimOk = L::small and L::known
}
