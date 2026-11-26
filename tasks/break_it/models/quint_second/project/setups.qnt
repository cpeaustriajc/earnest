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

module three {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 2, CAPACITY = 4).* from "./lock"
  run oneEachTest = init.then(3.reps(_ => request)).expect(net.length() == 3)
}

module tight {
  pure val Cap = 2
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 2, CAPACITY = Cap).* from "./lock"
}

module big {
  import lock(NODES = Set("a", "b", "c", "d"), MAX_SENT = 2, CAPACITY = 4).* from "./lock"
}

module huge {
  import lock(NODES = Set("a", "b", "c", "d"), MAX_SENT = 3, CAPACITY = 5).* from "./lock"
}
