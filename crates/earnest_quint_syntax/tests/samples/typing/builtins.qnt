module builtins {
  var x: int
  var log: List[int]
  val naturals = Nat.filter(n => n < 3)
  val integers = Int.exists(n => n == 0)
  val booleans = Bool.forall(b => b or not(b))
  val equal = 1 == 1 and 1 != 2
  val logic = (true iff false) implies true
  val member = 1.in(Set(1)) and Set(1).contains(1)
  val setOps = Set(1).union(Set(2)).intersect(Set(1)).exclude(Set(3))
  val subset = Set(1).subseteq(Set(1, 2))
  val mapped = Set(1).map(n => n + 1)
  val folded = Set(1, 2).fold(0, (acc, n) => acc + n)
  val subsets = Set(1).powerset().flatten()
  val lists = Set(1).allLists()
  val listsUpTo = Set(1).allListsUpTo(2)
  val only = Set(1).getOnlyElement()
  val some = Set(1).chooseSome()
  val finite = Set(1).isFinite() and Set(1).size() == 1
  val table = Set(1).mapBy(n => n + 1)
  val fromSet = Set((1, "a")).setToMap()
  val maps = Set(1).setOfMaps(Set("a"))
  val got = table.get(1) + table.keys().size()
  val changed = table.set(1, 3).setBy(1, n => n + 1).put(2, 2)
  val listOps = List(1).append(2).concat(List(3))
  val ends = List(1).head() + List(1, 2).tail().length()
  val indexed = List(1).nth(0) + List(1)[0] + List(1).indices().size()
  val replaced = List(1).replaceAt(0, 2).slice(0, 1)
  val ranged = range(0, 3).select(n => n > 0)
  val leftFold = List(1).foldl(0, (acc, n) => acc + n)
  val rightFold = List(1).foldr(0, (n, acc) => acc + n)
  val arithmetic = 1 + 2 - 3 * 4 / 5 % 6 ^ 2
  val comparisons = 1 < 2 and 2 > 1 and 1 <= 1 and 1 >= 1
  val negated = -x
  val interval = 1.to(3)
  action choose = { nondet c = Set(1).oneOf()
    all { x' = c, log' = log } }
  val tupled = tuples(Set(1), Set("a"))
  val record = { a: 1 }
  val names = record.fieldNames()
  val third = (1, 2, 3)._3
  val debugged = q::debug("x", 1)
  action generate = { nondet g: int = apalache::generate(1)
    all { x' = g, log' = log } }
  action init = all { x' = 0, log' = List() }
  action step = all {
    any {
      x' = x + 1,
      x' = if (x > 3) 0 else x,
    },
    log' = log.append(x),
  }
  temporal both = step and step
  temporal either = step or step
  temporal always_ = always(x >= 0)
  temporal eventually_ = eventually(x > 2)
  temporal nextValue = next(x) >= 0
  temporal kept = always(step.orKeep(x))
  temporal changes = always(step.mustChange(x))
  temporal possible = enabled(step)
  temporal weak = weakFair(step, x)
  temporal strong = strongFair(step, x)
  temporal leads = (x == 0) leadsTo (x == 1)
  run sequence = init.then(step).expect(x == 1)
  run repeated = init.then(2.reps(i => step))
  run failing = init.then(step.fail())
  action asserted = all { assert(x >= 0), x' = x, log' = log }
}
