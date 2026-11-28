module widening {
  var seen: Set[int]
  var last: int
  action init = all { seen' = Set(0), last' = 0 }
  action extend = {
    nondet x = seen.oneOf()
    all { x < 39, seen' = seen.union(Set(x + 1)), last' = x }
  }
  action step = extend
  val lastSeen = seen.contains(last)
  val belowForty = seen.size() < 40
}
