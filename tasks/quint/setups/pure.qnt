// Pure definitions and values that depend on a constant.
module Spec {
  const Size: int

  pure val Top = Size - 1
  pure def wrap(i: int): int = if (i > Top) 0 else i
  val Cells = 0.to(Top)
  def ahead(i: int): int = wrap(i + 1)

  var at: int
  var visited: Set[int]

  action init = all { at' = 0, visited' = Set(0) }
  action step = all { at' = ahead(at), visited' = visited.union(Set(ahead(at))) }

  /// @check

  val inside = Cells.contains(at)
  /// @check
  val notAll = visited.size() < Size
}

/// @check
module Four { import Spec(Size = 4).* }
