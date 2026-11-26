module servedRuns {
  var pending: Set[str]
  var served: Set[str]
  action init = all { pending' = Set(), served' = Set() }
  action add(who: str): bool = all {
    pending' = pending.union(Set(who)),
    served' = served,
  }
  action serve = {
    nondet who = pending.oneOf()
    all { pending' = pending.exclude(Set(who)), served' = served.union(Set(who)) }
  }
  action step = any {
    nondet who = Set("ann", "bob").oneOf()
    add(who),
    serve,
  }
  val everyoneOnce = pending.intersect(served) == Set()
  run serveOneTest = init.then(add("ann")).then(add("bob")).then(serve)
    .expect(served.size() == 1 and pending.size() == 1)
  run serveAnnTest = init.then(add("ann")).then(add("bob")).then(serve)
    .expect(served == Set("ann"))
  run serveNobodyTest = init.then(serve)
  run serveNobodyFailsTest = init.then(serve).fail()
}
