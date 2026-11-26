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

/// @check
module Named {
  import Base(K = 3) as B
  action init = B::init
  action step = B::step
  /// @check
  val ok = B::ok
  /// @check
  val below = B::below
  /// @check
  temporal grows = B::grows
}

/// @check
module Reexported {
  import Base(K = 3) as B
  action init = B::init
  action step = B::step
  /// @check
  val ok = B::ok
  /// @check
  val small = B::v < 3
  run ownTest = B::init.then(B::step).expect(B::v == 1)
}

/// @check
module ByOwn {
  pure val Own = 3
  import Base(K = Own) as B
  action init = B::init
  action step = B::step
  /// @check
  val ok = B::ok
}

/// @check
module ByName {
  import Limit.*
  import Base(K = twice(L)) as B
  action init = B::init
  action step = B::step
  /// @check
  val ok = B::ok
}

/// @check
/// Two specs, searched with a start and a step over both.
module Two {
  import Base(K = 2) as B
  import Other(M = 3) as O
  action init = all { B::init, O::start }
  action step = any { all { B::step, O::w' = O::w }, all { O::go, B::v' = B::v } }
  /// @check
  val ok = B::ok
  /// @check
  val fine = O::fine
}

/// @check
module Pair {
  import Base(K = 2) as P
  import Base(K = 3) as R
  action init = all { P::init, R::init }
  action step = any { all { P::step, R::v' = R::v }, all { R::step, P::v' = P::v } }
  /// @check
  val ok = P::ok and R::ok
  /// @check
  val apart = P::v != R::v or P::v == 0
}

/// @check
/// `Base` with `K = 1`, through an export; its properties named as `Plain` names them.
module Exported {
  import Base(K = 1) as B
  export B.*
  /// @check
  val okOne = ok
  /// @check
  val belowOne = below
  /// @check
  temporal growsOne = grows
}

/// @check
module Plain {
  import Base(K = 1) as B
  action init = B::init
  action step = B::step
  /// @check
  val okOne = B::ok
  /// @check
  val belowOne = B::below
  /// @check
  temporal growsOne = B::grows
}
