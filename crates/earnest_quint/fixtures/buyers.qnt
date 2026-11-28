module buyers {
  pure val Buyers = Set("ann", "bob")
  var controls: str -> bool
  var lastBuyer: str
  var previousPeer: str
  var noise: bool
  var log: int

  action init = all {
    controls' = Buyers.mapBy(b => false),
    lastBuyer' = "",
    previousPeer' = "",
    noise' = false,
    log' = 0,
  }
  action reachDeadline(buyer: str): bool = all {
    controls' = controls.set(buyer, true),
    lastBuyer' = buyer,
    previousPeer' = lastBuyer,
  }
  action chatter = all {
    controls' = controls,
    lastBuyer' = lastBuyer,
    previousPeer' = previousPeer,
    noise' = not(noise),
    log' = log,
  }
  action step = any {
    nondet buyer = Buyers.oneOf()
    all { reachDeadline(buyer), noise' = noise, log' = log },
    chatter,
  }

  temporal perBuyer = Buyers.forall(buyer => and {
    weakFair(reachDeadline(buyer), (controls, lastBuyer, previousPeer)),
  })
  temporal allReach = perBuyer.implies(eventually(Buyers.forall(b => controls.get(b))))
  temporal unfairAllReach = eventually(Buyers.forall(b => controls.get(b)))
  temporal noisy = perBuyer.implies(eventually(noise))
  temporal annOnly = weakFair(reachDeadline("ann"), (controls, lastBuyer, previousPeer))
  temporal annReaches = annOnly.implies(eventually(controls.get("ann")))
  temporal bobReaches = annOnly.implies(eventually(controls.get("bob")))
  temporal overPeer = weakFair(reachDeadline("ann"), previousPeer).implies(eventually(controls.get("ann")))
  temporal parts = and {
    always(log == 0),
    perBuyer.implies(leadsTo(controls.get("ann"), Buyers.forall(b => controls.get(b)))),
  }
  temporal allForms = and {
    always(log == 0),
    Buyers.forall(buyer => weakFair(reachDeadline(buyer), { c: controls, peers: (lastBuyer, Set(previousPeer)) }))
      .implies(eventually(Buyers.forall(b => controls.get(b)))),
  }
  temporal stepOverSome = weakFair(step, (controls, lastBuyer, previousPeer)).implies(eventually(Buyers.forall(b => controls.get(b))))

  temporal leavesOpen = weakFair(reachDeadline("ann"), (controls, noise)).implies(eventually(controls.get("ann")))
  temporal givenTheState = weakFair(reachDeadline(lastBuyer), (controls, lastBuyer, previousPeer)).implies(eventually(controls.get("ann")))
  temporal neverPicked = weakFair(reachDeadline("cid"), (controls, lastBuyer, previousPeer)).implies(eventually(controls.get("ann")))
  temporal someBuyer = Buyers.exists(buyer => weakFair(reachDeadline(buyer), (controls, lastBuyer, previousPeer))).implies(eventually(controls.get("ann")))
}
