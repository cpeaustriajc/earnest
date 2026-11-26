// The same spec as `SpecStar` of `star.qnt`, with its imports under a name. Quint 0.33.1 can't
// compile a module of this file (`Name 'twice' not found`), so it is compared with `Star`.
module SpecAs {
  import Kinds as T from "./star"
  import Helpers as H from "./star"
  const K: int
  var n: int
  var on: T::Flag
  var last: T::Cmd
  action init = all { n' = H::Start, on' = false, last' = T::Up }
  action up = all { n < K, n' = n + H::Start, on' = not(on), last' = T::Up }
  action down = all { n > 0, n' = n - 1, on' = on, last' = T::Down(n) }
  action step = any { up, down }
  val ok = n <= H::quad(K)
  val noDownAtTop = last != T::Down(K)
  run upTest = init.then(up).expect(n == 2)
  run downTest = init.then(down).expect(last == T::Down(1))
}

module As {
  import Kinds.* from "./star"
  import Helpers.* from "./star"
  import SpecAs(K = quad(1)).*
  val small = n < quad(2) and last != Down(quad(1) + 1)
}
