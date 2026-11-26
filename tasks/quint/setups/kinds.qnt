// Constants of every kind the reader reads: bool, text, a set, a record, a tuple, a map, a list.
module Spec {
  const On: bool
  const Name: str
  const Names: Set[str]
  const Limits: { low: int, high: int }
  const Pair: (int, str)
  const Weights: str -> int
  const Steps: List[int]

  var who: str
  var level: int
  var lit: bool

  action init = all { who' = Name, level' = Limits.low, lit' = On }
  action step = {
    nondet nextWho = Names.oneOf()
    nondet by = Steps.indices().oneOf()
    all {
      level + Steps[by] <= Limits.high,
      who' = nextWho,
      level' = level + Steps[by] + Weights.get(nextWho) - Weights.get(nextWho),
      lit' = not(lit),
    }
  }

  /// @check

  val inRange = level >= Limits.low and level <= Limits.high
  /// @check
  val paired = Pair._1 >= 0 and Pair._2 != ""
  /// @check
  val known = Names.contains(who)
}

/// @check
module Setup {
  import Spec(
    On = true,
    Name = "a",
    Names = Set("a", "b"),
    Limits = { low: 1, high: 4 },
    Pair = (1, "x"),
    Weights = Map("a" -> 1, "b" -> 2),
    Steps = [1, 2],
  ).*
}
