// A spec used in a setup that imports a types module and helpers with `.*`, helpers that
// import a module themselves, and two specs that import the same helpers.
module Lib {
  pure def twice(x: int): int = 2 * x
}

module Helpers {
  import Lib.*
  pure val Start = 1
  pure def quad(x: int): int = twice(twice(x))
}

module Kinds {
  type Flag = bool
  type Cmd = Up | Down(int)
}

module SpecStar {
  import Kinds.*
  import Helpers.*
  const K: int
  var n: int
  var on: Flag
  var last: Cmd
  action init = all { n' = Start, on' = false, last' = Up }
  action up = all { n < K, n' = n + Start, on' = not(on), last' = Up }
  action down = all { n > 0, n' = n - 1, on' = on, last' = Down(n) }
  action step = any { up, down }
  val ok = n <= quad(K)
  val noDownAtTop = last != Down(K)
  run upTest = init.then(up).expect(n == 2)
  run downTest = init.then(down).expect(last == Down(1))
}

module Star {
  import Kinds.*
  import Helpers.*
  import SpecStar(K = quad(1)).*
  val small = n < quad(2) and last != Down(quad(1) + 1)
}

module SpecOther {
  import Kinds.*
  import Helpers.*
  const K: int
  var n: int
  var last: Cmd
  action init = all { n' = quad(Start), last' = Up }
  action step = all { n < quad(K), n' = n + 1, last' = Down(n) }
  val ok = n <= quad(K)
  run upTest = init.then(step).expect(n == 5)
}

module Both {
  import Kinds.*
  import SpecStar(K = 2) as A
  import SpecOther(K = 2) as B
  action init = all { A::init, B::init }
  action step = any {
    all { A::step, B::n' = B::n, B::last' = B::last },
    all { B::step, A::on' = A::on, A::n' = A::n, A::last' = A::last },
  }
  val apart = A::n <= 2 and B::n <= 8
  val tagged = A::last != Down(2) and B::last != Down(9)
  run bothTest = init.then(all { A::up, B::step }).expect(A::n + B::n == 7)
}
