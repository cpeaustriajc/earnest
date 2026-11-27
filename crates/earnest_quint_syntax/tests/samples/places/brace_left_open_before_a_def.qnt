module A {
  var x: int
  val small = and {
    x >= 0,
  def at(v) = x == v
  action init = x' = 0
  action step = x' = x
  /// @check
  val ok = x >= 0
}
