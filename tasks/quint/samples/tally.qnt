/// @check
/// No variable and no step: pure definitions played by runs alone.
module tally {
  pure def total(xs: List[int]): int = xs.foldl(0, (sum, x) => sum + x)

  pure def clamp(x: int, low: int, high: int): int =
    if (x < low) low else if (x > high) high else x

  run clampsTest = all {
    assert(clamp(9, 0, 5) == 5),
    assert(clamp(-1, 0, 5) == 0),
    assert(clamp(3, 0, 5) == 3),
  }

  run sumsTest = {
    val xs = [1, 2, 3]
    all { assert(total(xs) == 6) }
  }
}
