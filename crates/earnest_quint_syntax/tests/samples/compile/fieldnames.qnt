// `fieldNames` of a variable, of a call, of a parameter, of a `val` and of a record written out, in a spec and in an instance of it.
module Spec {
  type Slot = { at: int, on: bool }
  const Start: int
  var slot: Slot
  var count: int
  pure def make(n: int): Slot = { at: n, on: true }
  def namesOf(r) = r.fieldNames()
  pure val fixed: { lo: int, hi: int } = { lo: 1, hi: 2 }
  pure val bare = { x: 1 }
  action init = all { slot' = make(Start), count' = 0 }
  action step = all { slot' = make(slot.at + 1), count' = count + 1 }
  val named = slot.fieldNames() == Set("at", "on")
  val called = make(3).fieldNames() == Set("at", "on")
  val viaParam = namesOf(fixed) == Set("lo", "hi")
  val valued = fixed.fieldNames().size() == 2 and bare.fieldNames().size() == 1
  val literal = { a: 1 }.fieldNames().size() == 1
}
module Main {
  import Spec(Start = 0).*
}
