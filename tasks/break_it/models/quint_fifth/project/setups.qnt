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

/// @check V::mutex, V::holderKnown, V::bounded, V::gRequests, V::gHolds, V::gDrains, V::gServed, V::gIdle
module threeG {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 1, CAPACITY = 3) as V from "./lock"
  run oneEachTest = V::init.then(3.reps(_ => V::request)).expect(V::net.length() == 3)
  action init = V::init
  action step = V::stepGuarded
}

/// @check V::mutex, V::holderKnown, V::bounded, V::grantsBounded, V::gRequests, V::gHolds, V::gDrains, V::gServed, V::gIdle
module twoG {
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as V from "./lock"
  action init = V::init
  action step = V::stepGuarded
}
