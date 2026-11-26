module broken_eventually {
  var done: bool
  action init = done' = false
  action step = done' = true
  temporal finish = eventually(done)
}
