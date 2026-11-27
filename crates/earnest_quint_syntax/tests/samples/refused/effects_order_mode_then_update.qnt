module effects_order_mode_then_update {
  var x: int
  action a = all { x' = 1, x' = 2 }
  val v = x' = 1
}
