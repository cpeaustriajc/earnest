// The state, the steps and the marked property that `lamp.qnt` imports.
module wiring {
  var lit: bool
  pure def flipped(on: bool): bool = not(on)
  action init = lit' = false
  action step = lit' = flipped(lit)
  /// @check
  /// The lamp is lit or dark.
  val litOrDark = lit or not(lit)
}
