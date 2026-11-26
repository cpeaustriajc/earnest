// The `Lamp` module and 17 setups of it, one Quint call for all of them.
module Lamp {
  const Max: int

  var level: int
  var on: bool

  action init = all { level' = 0, on' = false }

  action toggle = all { on' = not(on), level' = level }
  action brighter = all { on, level < Max, level' = level + 1, on' = on }
  action off = all { on, level == Max, level' = 0, on' = false }
  action step = any { toggle, brighter, off }

  val levelOk = level <= 6
  /// @check
  val darkWhenOff = not(on) implies level <= Max
}

/// @check
module S1 {
  import Lamp(Max = 1).*
}

/// @check
module S2 {
  import Lamp(Max = 2).*
}

/// @check
module S3 {
  import Lamp(Max = 3).*
}

/// @check
module S4 {
  import Lamp(Max = 4).*
}

/// @check
module S5 {
  import Lamp(Max = 5).*
}

/// @check
module S6 {
  import Lamp(Max = 6).*
}

/// @check
module S7 {
  import Lamp(Max = 7).*
}

/// @check
module S8 {
  import Lamp(Max = 8).*
}

/// @check
module S9 {
  import Lamp(Max = 9).*
}

/// @check
module S10 {
  import Lamp(Max = 10).*
}

/// @check
module S11 {
  import Lamp(Max = 11).*
}

/// @check
module S12 {
  import Lamp(Max = 12).*
}

/// @check
module S13 {
  import Lamp(Max = 13).*
}

/// @check
module S14 {
  import Lamp(Max = 14).*
}

/// @check
module S15 {
  import Lamp(Max = 15).*
}

/// @check
module S16 {
  import Lamp(Max = 16).*
}

/// @check
module S17 {
  import Lamp(Max = 17).*
}
