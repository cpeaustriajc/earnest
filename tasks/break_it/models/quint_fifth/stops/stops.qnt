module stops {
  var paid: bool
  var sent: bool
  var closed: bool
  var n: int
  val vars = (paid, sent, closed, n)
  action init = all { paid' = false, sent' = false, closed' = false, n' = 0 }
  action pay = all { not(paid), paid' = true, sent' = sent, closed' = closed, n' = n }
  action send = all { paid, not(sent), sent' = true, paid' = paid, closed' = closed, n' = n }
  action close = all { closed' = true, paid' = paid, sent' = sent, n' = n }
  action touch = all { n' = n, paid' = paid, sent' = sent, closed' = closed }
  action bump = all { n < 1, n' = n + 1, paid' = paid, sent' = sent, closed' = closed }
  action step = any {
    all { not(closed), any { pay, send, close, bump } },
    all { closed, touch },
  }
  temporal both = (weakFair(step, vars) and weakFair(pay, vars) and weakFair(send, vars)) implies eventually(paid)
  temporal sendOnly = (weakFair(step, vars) and weakFair(send, vars)) implies eventually(paid)
  temporal stepOnly = weakFair(step, vars) implies eventually(paid)
  temporal stutter = (weakFair(step, vars) and weakFair(touch, vars)) implies eventually(paid)
  temporal leads = (weakFair(step, vars) and weakFair(pay, vars)) implies leadsTo(n == 1, paid)
  temporal parts = and {
    weakFair(step, vars).implies(eventually(closed or paid)),
    (weakFair(step, vars) and weakFair(pay, vars)).implies(always(eventually(paid))),
    weakFair(pay, vars).implies(leadsTo(closed, paid)),
  }
  temporal sendsAfter = (weakFair(step, vars) and weakFair(pay, vars) and weakFair(send, vars)) implies eventually(sent)
  temporal bumpNarrow = (weakFair(step, vars) and weakFair(bump, n)) implies eventually(n == 1)
}
