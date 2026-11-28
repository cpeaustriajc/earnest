/// @check
module Cased {
  var n: int
  action init = n' = 0
  action step = n' = n
  /// @Check
  val upper = n < 9
  /// @CHECK broken
  val shout = n < 9
  // @check
  val commented = n < 9
  /* @check */
  val blocked = n < 9
  // see @check in the docs
  val later = n < 9
  // @check
  temporal tmp = always(n >= 0)
  // @check
  run aTest = init.then(step).expect(n == 0)
}
// @check
module Plain {
  var m: int
  action init = m' = 0
  action step = m' = m
  val ok = m == 0
}
