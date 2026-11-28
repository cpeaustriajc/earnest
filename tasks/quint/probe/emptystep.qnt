module emptystep {
  var a: int
  action init = a' = 0
  action step = { nondet v = Set(1,2).filter(x => x > a).oneOf()
    a' = v }
  val ok = a <= 3
}
