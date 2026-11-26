// A `nondet` that shadows a variable, an operator with parameters, a type alias and a sum type.
module Spec {
  type Flag = bool
  type Light = Red | Amber | Green
  const N: int

  var x: int
  var f: Flag
  var l: Light

  pure def twice(n: int): int = n * 2
  def cycle(light: Light): Light = match light { | Red => Green | Green => Amber | Amber => Red }

  action init = all { x' = 0, f' = false, l' = Red }
  action move(by: int): bool = all { x' = (x + by) % (N + 1), f' = not(f), l' = cycle(l) }
  action step = {
    nondet x = 0.to(N).oneOf()
    move(twice(x))
  }

  /// @check

  val ok = x <= N
  /// @check
  val notAmberTwice = not(f and l == Amber)
}

/// @check
module S { import Spec(N = 3).* }
