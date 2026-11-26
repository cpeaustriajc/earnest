// Edge semantics: failed assignment in all/any, empty oneOf, deadlock, big ints.
module edge {
  var x: int
  var s: Set[int]

  action init = all { x' = 1, s' = Set() }

  // all: the first guard fails after an assignment was made
  action halfThere = all { x' = x + 1, x > 100, s' = s }
  // any: one branch disabled
  action grow = all { x' = x * 1000000007, s' = s.union(Set(x)) }
  action pickEmpty = {
    nondet v = s.filter(i => i < 0).oneOf()
    all { x' = v, s' = s }
  }
  action step = any { halfThere, grow, pickEmpty }

  val small = x < 1000000007 * 1000000007 * 1000000007
}

module stuck {
  var y: int
  action init = y' = 0
  action step = all { y < 2, y' = y + 1 }
  val ok = y <= 2
  run toTwo = init.then(step).then(step).expect(y == 2)
  run third = init.then(3.reps(_ => step)).fail()
  run withNondet = {
    nondet k = 1.to(2).oneOf()
    init.then(k.reps(_ => step)).expect(y == k)
  }
  run asserted = init.then(all { assert(y == 0), step }).expect(y == 1)
  run wrongExpect = init.then(step).expect(y == 5)
}
