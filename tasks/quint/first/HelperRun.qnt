// The owner's sample: a run that calls a helper built with `then`.
/// @check
module HelperRun {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
  action TwoSteps: bool = init.then(step).then(step)
  run twoStepsTest = TwoSteps.expect(n == 2)
}
