module variant_compare {
  type Holder = Nobody | Held(str)
  var holder: Holder
  var m: int -> int
  action init = all { holder' = Nobody, m' = Map() }
  action step = all { holder' = Held("a"), m' = m.put(1, 1) }
  val known = holder == Nobody or holder == Held("a")
}
