/// @check ok
module choose {
  var x: int

  action init = x' = 0

  action step = x' = Set(1, 2).chooseSome()

  val ok = x >= 0
}
