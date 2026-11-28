/// @check
module emptyPick {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run assignedTest = {
    nondet n = 0.to(119).oneOf()
    nondet m = n.to(n + 1).filter(w => w % 3 == 1).oneOf()
    x' = m
  }
  run chainedTest = {
    nondet n = 0.to(119).oneOf()
    nondet m = n.to(n).filter(w => w != 5).oneOf()
    init.then(x' = m)
  }
  run initTest = {
    nondet n = 0.to(119).oneOf()
    nondet m = n.to(n).filter(w => w != 5).oneOf()
    init
  }
  run constantTest = {
    nondet n = 0.to(119).oneOf()
    nondet m = n.to(n).filter(w => w != 5).oneOf()
    m >= 0
  }
  run smallTest = {
    nondet n = 0.to(9).oneOf()
    nondet m = n.to(n).filter(w => w != 5).oneOf()
    init
  }
  run everyEmptyTest = {
    nondet n = 0.to(9).oneOf()
    nondet m = n.to(n).filter(w => w > 500).oneOf()
    init
  }
  run topEmptyTest = {
    nondet m = Set(1).filter(w => w > 500).oneOf()
    init
  }
}
