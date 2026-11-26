/// @check
module N {
  var x: int
  action init = x' = 0
  action advance = if (x < 5) x' = x + 1 else x' = x
  /// @check
  val small = x < 3
  run goTest = init.then(advance)
}
