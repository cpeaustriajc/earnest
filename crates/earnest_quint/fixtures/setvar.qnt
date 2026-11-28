module setvar {
  var s: Set[int]
  action init = s' = Set()
  action step = s' = s.union(Set(1))
}
