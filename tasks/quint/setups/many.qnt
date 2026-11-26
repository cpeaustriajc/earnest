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
  val darkWhenOff = not(on) implies level <= Max
}

module S1 {
  import Lamp(Max = 1).*
}

module S2 {
  import Lamp(Max = 2).*
}

module S3 {
  import Lamp(Max = 3).*
}

module S4 {
  import Lamp(Max = 4).*
}

module S5 {
  import Lamp(Max = 5).*
}

module S6 {
  import Lamp(Max = 6).*
}

module S7 {
  import Lamp(Max = 7).*
}

module S8 {
  import Lamp(Max = 8).*
}

module S9 {
  import Lamp(Max = 9).*
}

module S10 {
  import Lamp(Max = 10).*
}

module S11 {
  import Lamp(Max = 11).*
}

module S12 {
  import Lamp(Max = 12).*
}

module S13 {
  import Lamp(Max = 13).*
}

module S14 {
  import Lamp(Max = 14).*
}

module S15 {
  import Lamp(Max = 15).*
}

module S16 {
  import Lamp(Max = 16).*
}

module S17 {
  import Lamp(Max = 17).*
}
