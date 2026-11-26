// Names after a mark in a module's doc are judged in that check only (#72).
module Lamp {
  const Top: int
  var level: int
  action init = level' = 0
  action step = level' = if (level < Top) level + 1 else level
  val low = level <= Top
  val high = level >= Top
  temporal bounded = always(level <= Top)
  temporal stays = always(level == 0)
  run upTest = init.then(step).expect(level == 1)
  run upTwice = init.then(step).then(step).expect(level == 2)
  run tooFar = init.then(step).expect(level == 5)
}

/// @check low, bounded, upTwice
/// The names that hold, each judged here only.
module Holds {
  import Lamp(Top = 3).*
}

/// @check broken high, stays, tooFar
module Broken {
  import Lamp(Top = 3).*
}

/// The lamp reaches its top, on the second line of the doc.
/// @check reachable high
module Reached {
  import Lamp(Top = 3).*
}

/// @check
module Plain {
  import Lamp(Top = 3).*
}
