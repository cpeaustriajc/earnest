module pass {
  var has: Set[str]
  action init = has' = Set("a")
  action give = {
    nondet giver = has.oneOf()
    nondet taker = Set("a", "b", "c").exclude(Set(giver)).oneOf()
    has' = has.exclude(Set(giver)).union(Set(taker))
  }
  action step = give
  val oneHolder = has.size() == 1
  val neverC = not(has.contains("c"))
}
