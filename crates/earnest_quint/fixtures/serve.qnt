module serve {
  var pending: Set[str]
  var served: Set[str]
  action init = all { pending' = Set("ann", "bob", "cy"), served' = Set() }
  action serve = {
    nondet who = pending.oneOf()
    all { pending' = pending.exclude(Set(who)), served' = served.union(Set(who)) }
  }
  action step = serve
  val everyoneOnce = pending.size() + served.size() == 3
  val annNotFirst = served != Set("ann")
}
