// What a path prints: a step that changes one key of a large map, a run whose first action
// can't happen, and a fault while working out a run's first action.
module Printing {
  var balances: str -> int
  var seen: Set[int]
  var queue: List[int]

  action init = all {
    balances' = Map("ann" -> 3, "bob" -> 5, "cy" -> 0, "dee" -> 7, "eve" -> 1),
    seen' = Set(1, 3, 5, 7),
    queue' = [1, 2, 3, 4, 5],
  }

  action pay = all {
    balances.get("ann") < 5,
    balances' = balances.set("ann", balances.get("ann") + 1),
    seen' = seen.union(Set(balances.get("ann"))),
    queue' = queue.append(balances.get("ann")),
  }

  action step = pay

  action seed(d: int): bool = all { balances' = Map("ann" -> 10 / d), seen' = Set(), queue' = [] }

  val annBelowFive = balances.get("ann") < 5

  run noStartTest = all { 1 > 2, balances' = Map(), seen' = Set(), queue' = [] }.then(step)
  run faultTest = seed(0).then(step)
}
