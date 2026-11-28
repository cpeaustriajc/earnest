module Spec {
  const N: int

  var c: int

  action init = c' = 0
  action step = c' = if (c < N) c + 1 else 0

  /// @check
  val inRange = c >= 0 and c <= N

  /// @view state
  val specView = c

  /// @view group
  val specGroup = if (c > 1) "late" else "early"

  run tickTest = init.then(step).expect(c == 1)
}

/// @check
module Inherits {
  import Spec(N = 2).*
}

/// @check
module Overrides {
  import Spec(N = 3).*

  /// @view state
  val mine = if (c == 0) "zero" else "more"
}

/// @check
module Two {
  import Spec(N = 1) as S
  import Spec(N = 2) as T

  action init = all { S::init, T::init }
  action step = any { all { S::step, T::c' = T::c }, all { T::step, S::c' = S::c } }

  /// @check
  val bothInRange = S::inRange and T::inRange
}

/// @check
module Chooses {
  import Spec(N = 1) as S
  import Spec(N = 2) as T

  action init = all { S::init, T::init }
  action step = any { all { S::step, T::c' = T::c }, all { T::step, S::c' = S::c } }

  /// @check
  val bothInRange = S::inRange and T::inRange

  /// @view state
  val pair = (S::c, T::c)

  /// @view group
  val side = if (S::c == T::c) "level" else "apart"
}

/// @check
module TwoOwn {
  import Spec(N = 2).*

  /// @view state
  val first = c

  /// @view state
  val second = c + 1
}

module Lib {
  var d: int

  /// @view state
  val shownD = if (d == 0) "none" else "some"
}

/// @check
module Once {
  import Lib.*
  import Lib as P
  export P.*

  action init = d' = 0
  action step = d' = if (d < 2) d + 1 else 0

  /// @check
  val small = d <= 2
}
