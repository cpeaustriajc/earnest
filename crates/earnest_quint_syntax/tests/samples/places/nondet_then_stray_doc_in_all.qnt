module A {
  var x: int
  var y: int
  action init = all { x' = 0, y' = 0 }
  action step = {
    nondet v = Set(1, 2).oneOf()
    all {
      x' = v,
      /// stray
      y' = y
    }
  }
}
