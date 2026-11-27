/// @check
/// A pure run that fails by an `assert`, as `quint test` reports it.
module tallyfails {
  pure def total(xs: List[int]): int = xs.foldl(0, (sum, x) => sum + x)

  /// @check broken
  run wrongTest = all { assert(total([1, 2]) == 4) }
}
