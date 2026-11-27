// A comma that ends a doc with no name after it.
/// @check p1,
module M {
  var x: int
  action init = x' = 0
  action step = x' = x
  val p1 = x == 0
}
