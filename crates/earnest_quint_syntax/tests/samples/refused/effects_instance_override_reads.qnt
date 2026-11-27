module inner {
  const N: int
  var y: int
  action step = y' = N
}

module effects_instance_override_reads {
  var x: int
  import inner(N = x) as I
}
