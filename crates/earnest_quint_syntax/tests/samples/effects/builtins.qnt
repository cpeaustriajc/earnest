module builtins {
  var x: int
  var s: Set[int]
  var l: List[int]
  var f: int -> int
  var r: { a: int, b: int }
  var t: (int, str)
  type V = A(int) | B
  var v: V
  val sets = Set(x).union(s).intersect(s).exclude(Set(1)).subseteq(s) and s.contains(x) and x.in(s)
  val setOps = s.powerset().flatten().size() + s.map(n => n + x).filter(n => n > x).fold(0, (a, b) => a + b)
  val quantifiers = s.exists(n => n == x) or s.forall(n => n != x)
  val choices = s.chooseSome() + s.getOnlyElement() + Set(s).flatten().size()
  val lists = s.allLists() == Set(l) and s.allListsUpTo(2) == Set(l) and s.isFinite()
  val maps = f.get(x) + f.keys().size() + s.mapBy(n => n + x).get(1) + Set((1, x)).setToMap().get(1)
  val mapsMore = setOfMaps(s, s) == Set(f) and f.set(1, x) == f and f.put(x, 1) == f and f.setBy(1, n => n + x) == f
  val records = r.a + r.with("b", x).b + r.fieldNames().size()
  val tupled = t._1 + x + tuples(s, s).size()
  val listsOps = l.append(x).concat(l).head() + l.tail().length() + l.nth(0) + l.indices().size() + l.replaceAt(0, x).slice(0, 1).length() + range(0, x).length() + l.select(n => n > x).foldl(0, (a, b) => a + b) + l.foldr(0, (a, b) => a + b)
  val ints = (x + 1 - 2) * 3 / 4 % 5 ^ 2 + -x
  val comparisons = x < 1 and x > 1 and x <= 1 and x >= 1 and x == 1 and x != 1 and not(x == 2) and (x == 1 iff x == 2) and (x == 1 implies x == 2)
  val intervals = 1.to(x).size()
  val conditionals = if (x > 0) x else -x
  val variants = match v { | A(n) => n + x | B => 0 }
  val constructed = A(x)
  val literals = Nat.contains(x) and Int.contains(x) and Bool.contains(x > 0)
  val structures = List(x, 1) == l and Rec("a", x, "b", 1) == r and Tup(x, "a") == t and Map(1 -> x) == f
  val connectives = and(x > 0, x < 2) or or(x == 0, x == 1)
  action assigns = x' = x + 1
  action generated = { nondet n = apalache::generate(3) s' = n }
  action anyOf = any { x' = 1, x' = 2 }
  action allOf = all { x' = 1, s' = s }
}
