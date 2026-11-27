/// @check
module falseRuns {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  pure def total(xs: List[int]): int = xs.foldl(0, (sum, x) => sum + x)
  run plainFalseTest = total([1]) == 2
  run plainTrueTest = total([1]) == 1
  run bareAssertTest = assert(total([1]) == 2)
  run firstAssertTest = all { assert(total([1, 2]) == 4), x' = 0 }
  run pickAssertTest = { nondet n = Set(1, 2, 3).oneOf() all { assert(n < 3), x' = n } }
  run passingAssertTest = all { assert(total([1, 2]) == 3), x' = 0 }
}
