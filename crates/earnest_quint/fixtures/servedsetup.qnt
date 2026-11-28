module Desk {
  const People: Set[str]
  var pending: Set[str]
  action init = pending' = People
  action serve = {
    nondet who = pending.oneOf()
    pending' = pending.exclude(Set(who))
  }
  action stay = all { pending == Set(), pending' = pending }
  action step = any { serve, stay }
  val annWaits = pending.contains("ann")
}

module TwoAtTheDesk {
  import Desk(People = Set("ann", "bob")) as D
  action init = D::init
  action step = D::step
  val annWaits = D::annWaits
}
