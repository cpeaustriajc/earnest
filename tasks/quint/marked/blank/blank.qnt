// A mark with no space after the slashes, each with a blank doc line beside it (#72).
/// @check
module M {
  var x: int
  action init = x' = 0
  action step = x' = x
  ///@check broken
  ///
  val ok = x == 0
  ///
  ///@check broken
  val two = x == 0
}
