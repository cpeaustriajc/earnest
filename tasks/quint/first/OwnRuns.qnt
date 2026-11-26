// The orchestrator's sample: a module of runs only with variables and actions of its own,
// no `init`, no `step` and no import.
module OwnRuns {
  var open: bool
  var count: int
  action seed(o: bool): bool = all { open' = o, count' = 0 }
  action bump = all { open, count' = count + 1, open' = open }
  run seededTest = seed(true).then(bump).then(bump).expect(count == 2)
  run closedTest = seed(false).then(bump).fail()
  run wrongTest = seed(true).then(bump).expect(count == 5)
}
