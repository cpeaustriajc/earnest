/// @check
module setups {
  import types.* from "./types"
  import lock(NODES = Set("a", "b"), MAX_SENT = 2, CAPACITY = 3) as Two from "./lock"
  action init = Two::init
  action step = Two::step
  /// @check
  val mutex = Two::mutex
  /// @check
  val holderKnown = Two::holderKnown
  /// @check
  val bounded = Two::bounded
  /// @check
  temporal drains = Two::drains
  /// @check
  temporal someGrant = Two::someGrant
  /// @check
  temporal served = Two::served
  run grantTest = init.then(Two::request).then(Two::deliver).then(Two::grant).then(Two::deliver).expect(Two::grants == 1)
  run releaseTest = grantTest.then(Two::release).then(Two::deliver).expect(Two::holder == Nobody)
}

/// @check
module three {
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 2, CAPACITY = 4) as L from "./lock"
  action init = L::init
  action step = L::step
  /// @check
  val mutex = L::mutex
  /// @check
  val holderKnown = L::holderKnown
  /// @check
  val bounded = L::bounded
  /// @check
  val grantsBounded = L::grantsBounded
  /// @check
  val neverTwoGrants = L::neverTwoGrants
  /// @check
  temporal drains = L::drains
  /// @check
  temporal someGrant = L::someGrant
  /// @check
  temporal served = L::served
  /// @check
  temporal everyoneIdle = L::everyoneIdle
  run oneEachTest = init.then(3.reps(_ => L::request)).expect(L::net.length() == 3)
}

/// @check
module tight {
  pure val Cap = 2
  import lock(NODES = Set("a", "b", "c"), MAX_SENT = 2, CAPACITY = Cap) as L from "./lock"
  action init = L::init
  action step = L::step
  /// @check
  val mutex = L::mutex
  /// @check
  val holderKnown = L::holderKnown
  /// @check
  val bounded = L::bounded
  /// @check
  val grantsBounded = L::grantsBounded
  /// @check
  temporal drains = L::drains
  /// @check
  temporal someGrant = L::someGrant
  /// @check
  temporal served = L::served
}

/// @check
module big {
  import lock(NODES = Set("a", "b", "c", "d"), MAX_SENT = 2, CAPACITY = 4) as L from "./lock"
  action init = L::init
  action step = L::step
  /// @check
  val mutex = L::mutex
  /// @check
  val holderKnown = L::holderKnown
  /// @check
  val bounded = L::bounded
  /// @check
  val grantsBounded = L::grantsBounded
  /// @check
  temporal drains = L::drains
  /// @check
  temporal someGrant = L::someGrant
  /// @check
  temporal served = L::served
}

/// @check
module huge {
  import lock(NODES = Set("a", "b", "c", "d"), MAX_SENT = 3, CAPACITY = 5) as L from "./lock"
  action init = L::init
  action step = L::step
  /// @check
  val mutex = L::mutex
  /// @check
  val holderKnown = L::holderKnown
  /// @check
  val bounded = L::bounded
  /// @check
  val grantsBounded = L::grantsBounded
  /// @check
  temporal drains = L::drains
  /// @check
  temporal someGrant = L::someGrant
  /// @check
  temporal served = L::served
}
