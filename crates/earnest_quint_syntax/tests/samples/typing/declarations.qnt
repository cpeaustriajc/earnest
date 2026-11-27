module constants {
  const N: int
  const Names: Set[str]
  const Picked: str -> int
  var counter: int
  val bound = N + 1
  def identity(x) = x
  val idInt = identity(1)
  val idSet = identity(Set("a"))
  assume positive = N > 0
  assume named = Names.size() >= 0
  action init = counter' = 0
  action step = counter' = counter + 1
  temporal grows = always(counter >= 0)
  temporal fair = weakFair(step, counter)
  run twoSteps = init.then(step).then(step).expect(counter == 2)
  run repeated = init.then(3.reps(_ => step))
}

module instances {
  import constants(N = 3, Names = Set("a"), Picked = Map("a" -> 1)) as C
  import constants(N = 5, Names = Set(), Picked = Map()).* 
  val fromC = C::bound
  val fromAll = bound
}
