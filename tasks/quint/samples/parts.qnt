module lamp {
  var on: bool
  action init = on' = false
  action toggle = on' = not(on)
}

module timer {
  var ticks: int
  action init = ticks' = 0
  action tick = all { ticks < 3, ticks' = ticks + 1 }
}

/// @check
/// Two modules with their own variables, put together under names.
module house {
  import lamp as L
  import timer as T

  action init = all { L::init, T::init }

  action flip = all { L::toggle, T::ticks' = T::ticks }
  action count = all { T::tick, L::on' = L::on }
  action step = any { flip, count }

  /// @check
  val tickBound = T::ticks <= 3

  /// @check broken
  val neverLit = not(L::on)

  run flips(n: int): bool = init.then(n.reps(_ => flip))
  run twiceTest = flips(2).expect(not(L::on))
  run thriceTest = flips(3).expect(L::on)
}
