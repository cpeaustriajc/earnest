module temporal_lambda {
  var s: Set[int]
  action init = s' = Set()
  action step = all { s.size() < 1, s' = s.union(Set(5)) }
  temporal small = always(s.forall(e => e < 9))
}
