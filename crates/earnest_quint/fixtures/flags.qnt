module flags {
  var on: bool
  action init = on' = false
  action step = { nondet flip = Bool.oneOf() on' = flip }
}
