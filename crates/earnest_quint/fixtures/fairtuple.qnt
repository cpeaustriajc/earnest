module fairtuple {
  var x: int
  var done: bool

  val vars = (x, done)
  val allVars = vars
  val backwards = (done, x)
  val onlyX = (x)

  action init = all { x' = 0, done' = false }
  action tick = all { x < 3, x' = x + 1, done' = x + 1 == 3 }
  action rest = all { x' = x, done' = done }
  action step = any { tick, rest }

  temporal finishes = weakFair(tick, vars) implies eventually(done)
  temporal finishesThroughVals = weakFair(tick, allVars) implies eventually(done)
  temporal finishesBackwards = weakFair(tick, backwards) implies eventually(x == 3)
  temporal unfairFinishes = eventually(done)
  temporal neverFour = not(eventually(x == 4))
  temporal neverTwo = not(eventually(x == 2))
  temporal leavesZero = weakFair(tick, vars) implies not(always(x == 0))
  temporal mayStayAtZero = not(always(x == 0))
  temporal halfListed = weakFair(tick, onlyX) implies eventually(done)
  temporal doubleNot = not(not(eventually(done)))
}
