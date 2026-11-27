// Instances with constants, under `.*` and under a name, an instance of a module that itself
// holds an instance, three deep, and the constants of an instance read through its name.
module Counter {
  const START: int
  const STEP: int
  var n: int
  action init = n' = START
  action step = n' = n + STEP
}

module Pair {
  const FIRST: int
  import Counter(START = FIRST, STEP = 1) as C
  val first = C::START + FIRST
}

module Triple {
  const BASE: int
  import Pair(FIRST = BASE) as P
  val base = P::first
}

module Instances {
  import Counter(START = 0, STEP = 2).*
  import Counter(START = 5, STEP = 1) as Five
  import Triple(BASE = 7) as T
  import Library(LIMIT = 3) as Lib from "./library"
  action both = all { init, Five::init }
  val start = START + Five::STEP
  val deep = T::base + Lib::limit
}

module Exported {
  import Counter(START = 1, STEP = 1) as C
  export C.*
  val start = START + C::STEP
}

module ThroughExport {
  import Exported.*
  val again = start + START
}
