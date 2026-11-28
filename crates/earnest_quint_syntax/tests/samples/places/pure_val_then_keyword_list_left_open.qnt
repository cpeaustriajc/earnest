module A {
  var x: int
  var y: int
  action step = {
    pure val p = 3 + 4
    and { x <= (y +), x' = y, y' = y }
  }
}
