// Every data operator the reader takes, in invariants that hold, so that the TLA+ export of
// each can be run through TLC (`earnest confirm --tlc`).
module everything {
  type Shape = Circle(int) | Square(int) | Dot
  var shapes: List[Shape]
  var seen: Set[int]
  var sizes: int -> int
  var pair: (int, str)

  action init = all {
    shapes' = [Dot],
    seen' = Set(),
    sizes' = Map(),
    pair' = (0, "a"),
  }

  pure def area(s: Shape): int = match s {
    | Circle(r) => 3 * r ^ 2
    | Square(w) => w * w
    | Dot => 0
  }

  action grow = {
    nondet r = 1.to(2).oneOf()
    all {
      shapes.length() < 3,
      shapes' = shapes.append(if (r == 1) Circle(r) else Square(r)),
      seen' = seen.union(Set(r)),
      sizes' = sizes.put(r, area(Square(r))),
      pair' = ((pair._1 + 1) % 3, pair._2),
    }
  }

  action trim = all {
    shapes.length() > 1,
    shapes' = shapes.tail(),
    seen' = seen,
    sizes' = if (sizes.keys().contains(1)) sizes.set(1, 0) else sizes,
    pair' = pair,
  }

  action step = any { grow, trim }

  val areas = shapes.foldl(0, (total, s) => total + area(s)) >= 0
  val images = seen.map(x => x * 2).forall(y => y % 2 == 0)
  val byKey = seen.mapBy(x => x).keys() == seen
  val pairs = tuples(seen, Set("a")).setToMap().keys() == seen
  val flat = Set(seen, Set(0)).flatten().contains(0)
  val subsets = Set(1, 2).powerset().size() == 4
  val listed = range(0, shapes.length()) == shapes.indices().fold([], (l, i) => l.append(i))
  val parts = shapes.slice(0, shapes.length()) == shapes and shapes.concat([]) == shapes
  val first = shapes.head() == shapes.nth(0)
  val replaced = shapes.replaceAt(0, Dot).length() == shapes.length()
  val chosen = shapes.select(s => area(s) == 0).length() <= shapes.length()
  val gotten = sizes.keys().forall(k => sizes.get(k) >= 0)
  val updated = sizes.keys().forall(k => sizes.setBy(k, v => v + 1).get(k) > 0)
  val text = pair._2 == "a" and { ...{ n: pair._1 }, n: 1 }.n == 1
  val finite = seen.isFinite() and seen.subseteq(1.to(2)) and seen.exclude(seen) == Set()
  val crossing = seen.intersect(Set(1)).size() <= 1 and 1.in(Set(1)) and 2 ^ 3 == 8
}
