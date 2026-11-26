module three {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 1, CAPACITY = 3).* from "./lock"
  run oneEachTest = init.then(3.reps(_ => request)).expect(net.length() == 3)
}

module two {
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3).* from "./lock"
}

/// @check bothMutex, fewGrants
module pair {
  import types.* from "./types"
  import lock(NODES = Set("a"), MAX_SENT = 1, CAPACITY = 2) as L from "./lock"
  import lock(NODES = Set("a", "b"), MAX_SENT = 1, CAPACITY = 2) as R from "./lock"
  action init = all { L::init, R::init }
  action keepR = all { R::peers' = R::peers, R::net' = R::net, R::waiting' = R::waiting, R::holder' = R::holder, R::grants' = R::grants }
  action keepL = all { L::peers' = L::peers, L::net' = L::net, L::waiting' = L::waiting, L::holder' = L::holder, L::grants' = L::grants }
  action step = any { all { L::stepEach, keepR }, all { R::stepEach, keepL } }
  val bothMutex = L::mutex and R::mutex
  val fewGrants = L::grants + R::grants <= 2
  run bothTest = init.then(all { L::requestBy("a"), keepR }).then(all { R::requestBy("b"), keepL }).expect(L::net.length() + R::net.length() == 2)
}

/// @check
module witness {
  import types.* from "./types"
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as W from "./lock"
  action serveFrom(n: Node): bool = W::startWith(n).then(W::deliver).then(W::grant).then(W::deliver)
  action requestTwice(n: Node): bool = W::requestBy(n).then(W::deliver).then(W::grant).then(W::deliver).then(W::releaseBy(n))
  run fromATest = serveFrom("a").expect(W::holder == Held("a"))
  run eachTest = { nondet n = Set("a", "b").oneOf() serveFrom(n).expect(W::peers.get(n).holds) }
  run thenRequestTest = W::init.then(requestTwice("b")).then(W::deliver).expect(W::holder == Nobody)
  run repsTest = W::startWith("b").then(2.reps(_ => W::stepEach)).expect(W::grants <= 1)
  run wrongHolderTest = serveFrom("a").expect(W::holder == Held("b"))
  run cannotTest = W::startWith("a").then(W::releaseBy("a")).fail()
  run tooManyTest = W::init.then(3.reps(_ => W::requestBy("a")))
}

/// @check V::mutex, V::holderKnown, V::bounded, V::everyoneRequests, V::everyoneHolds, V::eachDrains
/// @check broken V::GuardedBreak
module threeChecked {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 1, CAPACITY = 3) as V from "./lock"
  run oneEachTest = V::init.then(3.reps(_ => V::request)).expect(V::net.length() == 3)
  action init = V::init
  action step = V::stepEach
}

/// @check broken V::GuardedBreak
module threeKnown {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 1, CAPACITY = 3) as V from "./lock"
  run oneEachTest = V::init.then(3.reps(_ => V::request)).expect(V::net.length() == 3)
  action init = V::init
  action step = V::step
}

/// @check V::mutex, V::holderKnown, V::bounded, V::grantsBounded, V::everyoneRequests, V::everyoneHolds, V::eachDrains
module twoChecked {
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as V from "./lock"
  action init = V::init
  action step = V::stepEach
}

/// @check broken V::GuardedBreak
module twoGuard {
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as V from "./lock"
  action init = V::init
  action step = V::stepEach
}
