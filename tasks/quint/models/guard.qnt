// An invariant that breaks before its guard holds: `Late implies Small` breaks only where `Late` holds.
module guard {
  var n: int

  action init = n' = 0
  action step = any {
    all { n < 4, n' = n + 1 },
    all { n >= 4, n' = n },
  }

  val Small = n < 2
  val Late = n >= 3
}
