// One name twice in one list.
/// @check p1, p1
module M {
  var x: int
  action init = x' = 0
  action step = x' = x
  val p1 = x == 0
}
