module mixedpick {
  var bag: Set[int]
  action init = bag' = Set()
  action add = {
    nondet n = 1.to(3).oneOf()
    bag' = bag.union(Set(n))
  }
  action move = {
    nondet by = Set(1, 2).oneOf()
    nondet a = bag.oneOf()
    all { a + by <= 4, bag' = bag.exclude(Set(a)).union(Set(a + by)) }
  }
  action step = any { add, move }
  val inRange = bag.forall(v => v >= 1 and v <= 4)
  val noFour = not(bag.contains(4))
}
