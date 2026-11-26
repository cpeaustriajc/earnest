// A setup of a spec that uses `Bool`, `Nat`, `fieldNames`, `foldr`, an operator given as an
// argument and a helper no check reaches, built by earnest from `quint typecheck`.
module ReadSpec {
  type Slot = { at: int, on: bool }
  const Limit: int
  var slot: Slot
  var log: List[int]

  pure def app(f: int => int, v: int): int = f(v)
  pure def unused(s: Set[int]): int = s.chooseSome()

  action init = all {
    nondet on = Bool.oneOf()
    all { slot' = { at: 0, on: on }, log' = [] }
  }
  action step = all {
    slot.at < Limit,
    slot' = slot.with("at", app(v => v + 1, slot.at)),
    log' = log.append(slot.at),
  }

  val named = slot.fieldNames() == Set("at", "on") and slot.at.in(Nat) and slot.on.in(Bool)
  val reversed = log.foldr(0, (d, total) => total * 10 + d) >= 0
}

module ReadSetup {
  import ReadSpec(Limit = 3).*
}
