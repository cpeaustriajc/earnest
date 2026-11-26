// A dimmer: a level that goes up to a constant and back to zero.
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
  val darkWhenOff = not(on) implies level <= Max
}

/// @check levelOk, darkWhenOff
module Dim {
  import Lamp(Max = 2).*
}

/// @check levelOk
module Bright {
  import Lamp(Max = 5).*
}

/// @check levelOk
module Blinding {
  import Lamp(Max = 9).*
}
