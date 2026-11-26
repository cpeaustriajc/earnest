// `getOnlyElement` of a set of two is a fault, as Quint's TypeScript evaluator stops on it.
module onlyfault {
  var x: int
  var b: bool
  action init = all { x' = 0, b' = false }
  action step = all {
    nondet c = Bool.oneOf()
    all {
      x' = Set(x, x * 2).getOnlyElement() + 1,
      b' = q::debug("c", c),
    }
  }
  val inv = b.in(Bool)
  run boolTest = {
    nondet c = Bool.oneOf()
    init.then(step).expect(x == 1 and c.in(Bool))
  }
}
