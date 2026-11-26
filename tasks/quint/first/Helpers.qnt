// Definitions that chain steps with `then`, `reps`, `expect` and `fail`, played where a run
// calls them.
module Helpers {
  var n: int
  var page: str
  action init = all { n' = 0, page' = "home" }
  action step = all { n < 5, n' = n + 1, page' = page }
  action open(p: str): bool = all { n' = n, page' = p }
  action TwoSteps: bool = init.then(step).then(step)
  action SeeMoney(p: str): bool = open(p).then(step).expect(page == p)
  action TwoMore = step.then(step)
  action Fresh(p: str): bool = all { n' = 0, page' = p }.then(step)
  action FourMore = TwoMore.then(TwoMore)
  action Climb(k: int): bool = k.reps(_ => step)
  run Opening(p: str): bool = init.then(open(p))
  run twoStepsTest = TwoSteps.expect(n == 2)
  run argumentsTest = init.then(SeeMoney("money")).expect(n == 1 and page == "money")
  run startTest = Fresh("start").then(step).expect(n == 2 and page == "start")
  run middleTest = init.then(TwoMore).expect(n == 2)
  run nestedTest = init.then(FourMore).expect(n == 4)
  run repsTest = init.then(2.reps(_ => TwoMore)).expect(n == 4)
  run paramRepsTest = init.then(Climb(3)).expect(n == 3)
  run runParamTest = Opening("help").then(TwoMore).expect(page == "help" and n == 2)
  run failTest = init.then(Climb(5)).then(step).fail()
  run overTest = init.then(FourMore).then(TwoMore)
  run nondetArgTest = { nondet p = Set("a", "b").oneOf() init.then(SeeMoney(p)).expect(page == p) }
  run wrongTest = TwoSteps.expect(n == 3)
}

module StepIsChain {
  var n: int
  action init = n' = 0
  action step = n' = n + 1
  action Twice: bool = step.then(step)
  action onward = Twice
}
