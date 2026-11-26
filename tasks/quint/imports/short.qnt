// A refusal inside a setup names the definition by its short name: `picked`, not `Few::C::picked`.
module Spec {
  const NODES: Set[str]
  var on: str -> bool
  action init = on' = NODES.mapBy(_ => false)
  action step = { nondet n = NODES.oneOf() on' = on.set(n, true) }
  val picked = NODES.chooseSome() == "a"
}

module Few {
  import Spec(NODES = Set("a", "b")) as C
  action init = C::init
  action step = C::step
}
