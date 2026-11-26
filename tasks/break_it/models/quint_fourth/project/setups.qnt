module three {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 1, CAPACITY = 3).* from "./lock"
  run oneEachTest = init.then(3.reps(_ => request)).expect(net.length() == 3)
}

module two {
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3).* from "./lock"
}

module witness {
  import types.* from "./types"
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as W from "./lock"
  action serveFrom(n: Node): bool = W::startWith(n).then(W::deliver).then(W::grant).then(W::deliver)
  run fromATest = serveFrom("a").expect(W::holder == Held("a"))
  run eachTest = { nondet n = Set("a", "b").oneOf() serveFrom(n).expect(W::peers.get(n).holds) }
  run doubleTest = W::init.then(W::requestTimes("a", 2)).then(W::deliver).then(W::deliver).expect(W::waiting == ["a"])
  run settleTest = serveFrom("b").then(W::releaseBy("b")).then(W::deliver).then(W::settle).expect(W::holder == Nobody)
  run wrongTest = serveFrom("a").then(W::settle).expect(W::holder == Nobody)
}
