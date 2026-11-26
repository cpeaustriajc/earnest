// Int and bool vars, pure defs, any/all, if, nondet over a range.
module counter {
  pure val Limit = 5
  pure def clamp(n: int, hi: int): int = if (n > hi) hi else n

  var count: int
  var on: bool

  action init = all {
    count' = 0,
    on' = false,
  }

  action bump = {
    nondet by = 1.to(3).oneOf()
    all {
      on,
      count' = clamp(count + by, Limit),
      on' = on,
    }
  }

  action toggle = all {
    on' = not(on),
    count' = count,
  }

  action reset = all {
    count == Limit,
    count' = 0,
    on' = false,
  }

  action step = any { bump, toggle, reset }

  val inRange = count >= 0 and count <= Limit
  val tooBig = count < 4
}
