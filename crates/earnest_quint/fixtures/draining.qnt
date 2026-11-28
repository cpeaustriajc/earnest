module draining {
  var pending: Set[int]
  var done: bool
  action init = all { pending' = Set(1, 2, 3), done' = false }
  action serve = {
    nondet who = pending.oneOf()
    all { pending' = pending.exclude(Set(who)), done' = done }
  }
  action finish = all { pending == Set(), not(done), done' = true, pending' = pending }
  action step = any { serve, finish }
  temporal drained = eventually(pending == Set())
  temporal fairDrained = weakFair(serve, (pending, done)) implies eventually(pending == Set())
  temporal fairDone = weakFair(serve, (pending, done)) implies eventually(done)
  temporal bothFair = (weakFair(serve, (pending, done)) and weakFair(finish, (pending, done)))
    implies eventually(done)
}
