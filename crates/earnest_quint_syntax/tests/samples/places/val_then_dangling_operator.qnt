module A {
  var x: int
  action init = x' = 0
  action step = { val v = 0.to(2).oneOf() x' = v + }
}
