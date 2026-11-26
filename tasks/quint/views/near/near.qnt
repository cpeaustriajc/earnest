/// @check
module near {
  var n: int

  action init = n' = 0
  action step = n' = if (n < 2) n + 1 else 0

  /// @check
  val small = n <= 2

  /// @View state
  val a = n

  /// @VIEW group
  val b = n

  // @view state
  val c = n

  /** @view group */
  val d = n

  /**
   * @view state
   */
  val e = n

  /// @vew state
  val f = n

  // @view state, which is not above a declaration

  val g = n
}
