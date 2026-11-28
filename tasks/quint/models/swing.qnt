// A run that swings between 0 and 1 forever is fair to `a` and `b`, but not to `out`.
module swing {
  var n: int
  var spare: bool
  action init = all { n' = 0, spare' = false }
  action a = all { n == 0, n' = 1, spare' = spare }
  action b = all { n == 1, n' = 0, spare' = spare }
  action out = all { n < 2, n' = 2, spare' = spare }
  action step = any { a, b, out }

  temporal swingFair = (weakFair(a, (n, spare)) and weakFair(b, (n, spare))) implies eventually(n == 2)
  temporal outFair = weakFair(out, (n, spare)) implies eventually(n == 2)
  temporal stepFair = weakFair(step, (n, spare)) implies eventually(n == 2)
  temporal safe = always(n <= 2)
}
