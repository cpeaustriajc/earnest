module Expressions {
  var x: int
  var b: bool
  val ifs = if (x > 0) 1 else if (x < 0) -1 else 0
  val elsePair = if (b) 1 else 2 -> 3
  val record = { a: 1, b: "two" }
  val recordTrailing = { a: 1, }
  val spread = { ...record, a: 2 }
  val spreadLast = { b: "three", ...record }
  val spreadAlone = { ...record }
  val spreadTwo = { ...record, a: 3, b: "four" }
  val fieldA = record.a
  val withField = record.with("a", 5)
  val tuple = (1, "two", false)
  val tupleTrailing = (1, 2,)
  val unit = ()
  val pair = 1 -> 2
  val pairs = 1 -> 2 -> 3
  val itemOne = tuple._1
  val itemTwo = tuple._2
  val notItem = { _0: 1 }._0
  val list = [1, 2, 3]
  val listTrailing = [1, 2,]
  val emptyList = []
  val commaList = [,]
  val nthOne = list[0]
  val nthNested = [[1]][0][0]
  val allOf = all { x > 0, b, }
  val anyOf = any { x > 0, b }
  val andOf = and { b, b }
  val orOf = or { b, not(b), }
  val calls = Set(1, 2).union(Set(3)).size()
  val infixCalls = b.and(b).or(b).implies(b).iff(b)
  val leads = b.leadsTo(b)
  val namedCall = and(b, b)
  val orCall = or(b, b)
  val setCall = Set()
  val listCall = List(1, 2)
  val emptyArgs = Set(,)
  val braces = { x + 1 }
  val parens = (x + 1)
  action assignOne = x' = 1
  action assignAnd = x' = 1 and b' = true
  action assignOr = all { x' = x, b' = b }
}
