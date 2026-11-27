module startfault {
  var x: int
  action init = x' = (-7) / 2
  action step = x' = x
}
