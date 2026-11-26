module problems {
  var x: int

  action init = x' = 0
  action grow = x' = x + 1
  action step = grow

  val small = x < 5
  val chosen = Set(x).chooseSome() >= 0

  run fromNowhereTest = grow.then(grow)
  run growTest = init.then(grow)
  run lateFailTest = init.then(grow).fail().then(grow)
}

module repeated {
  var x: int

  action init = x' = 0
  action grow = x' = x + 1
  action pick = x' = Set(1, 2).chooseSome()
  action step = any { grow, pick }

  val small = x < 5
  val listed = allListsUpTo(Set(1), 2).size() > 0
  temporal strong = strongFair(grow, x) implies eventually(x > 2)
  temporal fairPick = weakFair(pick, x) implies eventually(x > 2)

  run pickTest = init.then(pick)
  run growTest = init.then(grow)
}
