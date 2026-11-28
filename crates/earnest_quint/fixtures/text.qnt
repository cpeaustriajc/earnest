module text {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  val named = if ("a" == "a") x >= 0 else false
}
