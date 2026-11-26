/// @check mutex, holderKnown, Two::bounded, Two::drains, Two::someGrant, Two::served
module setups {
  import types.* from "./types"
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as Two from "./lock"
  action init = Two::init
  action step = Two::step
  val mutex = Two::mutex
  val holderKnown = Two::holderKnown
  run grantTest = init.then(Two::request).then(Two::deliver).then(Two::grant).then(Two::deliver).expect(Two::grants == 1)
  run releaseTest = grantTest.then(Two::release).then(Two::deliver).expect(Two::holder == Nobody)
}

/// @check mutex, holderKnown, bounded, grantsBounded, neverTwoGrants, drains, someGrant, served, everyoneIdle
module three {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 2, CAPACITY = 4).* from "./lock"
  run oneEachTest = init.then(3.reps(_ => request)).expect(net.length() == 3)
}

/// @check mutex, holderKnown, bounded, grantsBounded, drains, someGrant, served
module tight {
  pure val Cap = 2
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 2, CAPACITY = Cap).* from "./lock"
}

/// @check mutex, holderKnown, bounded, grantsBounded, drains, someGrant, served
module big {
  import lock(NODES = Set("a", "b", "c", "d"), MAX_SENT = 2, CAPACITY = 4).* from "./lock"
}

/// @check mutex, holderKnown, bounded, grantsBounded, drains, someGrant, served
module huge {
  import lock(NODES = Set("a", "b", "c", "d"), MAX_SENT = 3, CAPACITY = 5).* from "./lock"
}
