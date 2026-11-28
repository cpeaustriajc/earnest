module inner {
  const P: bool
  var y: int
  action step = all { P, y' = 1 }
}

module effects_instance_override_updates {
  var x: int
  import inner(P = x' = 1) as I
}
