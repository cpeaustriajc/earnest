module startfault {
  var x: int
  action init = x' = (-7) / 0
  action step = x' = x
}
