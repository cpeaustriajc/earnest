module ActionArgument {
  var a: bool
  var b: int
  action between(changes: bool): bool = all {
    b' = b + 1,
    changes,
  }
  action init = all { a' = false, b' = 0 }
  action approve: bool = between(all { a' = true })
  action step = if (b < 3) approve else all { a' = a, b' = b }
  val Bounded = b <= 3
}
