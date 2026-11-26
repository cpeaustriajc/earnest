/// @check
module marks {
  /// @view state
  var n: int

  action init = n' = 0
  action step = n' = if (n < 2) n + 1 else 0

  /// @check
  val small = n <= 2

  /// @view title
  val titled = n

  /// The bare word.
  /// @view
  val bare = n

  /// @view state, n
  val named = n

  /// @view group
  action moved = n' = n

  /// @view state
  def twice(k) = k * 2

  /// @view state
  temporal staysSmall = always(small)

  /// @view group
  run playTest = init.then(step)

  ///@view state
  val unspaced = n

  /// @view state
  /// @view group
  val both = n
}

/// @check
/// @view state
module moduled {
  var m: int

  action init = m' = 0
  action step = m' = 1 - m

  /// @check
  val bit = m <= 1
}

module unchecked {
  /// @view group
  const K: int

  var k: int

  action init = k' = 0
  action step = k' = k

  /// @view state
  val label = k
}
