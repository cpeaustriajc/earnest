module temporals {
  var x: int
  var y: int
  action step = x' = x + 1
  temporal alwaysPositive = always(x >= 0)
  temporal eventuallyBig = eventually(x > 10)
  temporal leads = (x == 1).leadsTo(x == 2)
  temporal nested = always(eventually(x > 0) and y >= 0)
  temporal grows = always(next(x) >= x)
  temporal kept = always(step.orKeep(x))
  temporal changes = always(step.mustChange(x))
  temporal enabledStep = always(enabled(step))
  temporal weak = weakFair(step, x)
  temporal strong = strongFair(step, Set(x, y))
  temporal fair = weakFair(step, x) and strongFair(step, y) implies eventually(x > 3)
  temporal notStep = always(not(step))
  temporal combined = always(step and x > 0)
  temporal quantified = always(Set(1, 2).exists(n => x' = n))
  temporal definedWithParam(p) = always(p)
  temporal usesParam = definedWithParam(x > 0)
}
