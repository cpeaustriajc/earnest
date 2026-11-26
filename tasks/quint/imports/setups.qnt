// Three setups of the spec and a module of runs only, all checked from one typecheck.
/// @check
module two {
  import types.* from "./types"
  import doors(NODES = Set("a", "b"), LIMIT = 2).* from "./doors"
  run bothOpenTest = init.then(all {
    queue' = [Open("a"), Open("b")], sent' = 2, doors' = doors,
  }).then(deliver).then(deliver).expect(doors.get("b") == { open: true, count: 1 })
}

/// @check
module three {
  import doors(NODES = Set("a", "b", "c"), LIMIT = 3).* from "./doors"
}

/// @check
module named {
  import types.* from "./types"
  import lib.* from "./lib"
  import doors(NODES = Set("a"), LIMIT = 2) as D from "./doors"
  action init = D::init
  action step = D::step
  /// @check
  val fresh = D::doors.get("a") == closed or D::sent > 0
  run ownTest = init.then(D::ping).expect(D::queue == [Ping])
  run pingTest = init.then(D::ping).then(D::ping).expect(D::sent == 2)
}

/// @check
module scenarios {
  import types.* from "./types"
  import doors(NODES = Set("x"), LIMIT = 1) as S from "./doors"
  run sendTest = S::init.then(S::send).expect(S::sent == 1)
  run closeTest = S::init.then(S::send).then(S::deliver).expect(S::doors.get("x").count <= 1)
}
