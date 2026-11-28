module drain {
  var left: int
  action init = left' = 4
  action take = {
    nondet n = 1.to(left).oneOf()
    left' = left - n
  }
  action step = take
  val notNegative = left >= 0
  val notEmpty = left != 0
}
