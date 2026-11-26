// Setups beyond one `import Spec(...).*`: an instance under a name, constants given by names,
// several instances, two instances of one spec, and `export`.
module Base {
  const K: int

  var v: int

  action init = v' = 0
  action step = all { v < K, v' = v + 1 }

  val ok = v <= K
  val below = v < K
  temporal grows = weakFair(step, v) implies eventually(v == K)

  run upTest = init.then(step).expect(v == 1)
  run fullTest = init.then(K.reps(_ => step)).expect(v == K)
}

module Other {
  const M: int

  var w: int

  action start = w' = 0
  action go = all { w < M, w' = w + 1 }

  val fine = w <= M
}

module Limit {
  pure val L = 2
  pure def twice(x: int): int = 2 * x
}

module Named {
  import Base(K = 3) as B
}

module Reexported {
  import Base(K = 3) as B
  action init = B::init
  action step = B::step
  val ok = B::ok
  val small = B::v < 3
  run ownTest = B::init.then(B::step).expect(B::v == 1)
}

module ByOwn {
  pure val Own = 3
  import Base(K = Own).*
}

module ByName {
  import Limit.*
  import Base(K = twice(L)).*
}

module Two {
  import Base(K = 2).*
  import Other(M = 3).*
  action both = all { init, start }
  action either = any { all { step, w' = w }, all { go, v' = v } }
}

module Pair {
  import Base(K = 2) as P
  import Base(K = 3) as R
  action init = all { P::init, R::init }
  action step = any { all { P::step, R::v' = R::v }, all { R::step, P::v' = P::v } }
  val ok = P::ok and R::ok
  val apart = P::v != R::v or P::v == 0
}

module Exported {
  import Base(K = 1) as B
  export B.*
}

module Plain {
  import Base(K = 1).*
}
