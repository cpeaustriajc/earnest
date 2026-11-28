module guarded_step {
  var paid: bool
  var open: bool
  action init = all { paid' = false, open' = false }
  action pay = all { not(paid), paid' = true, open' = open }
  action toggle = all { open' = not(open), paid' = paid }
  action step = all { open or paid, any { pay, toggle } }
  action guardedBranch = any { all { open, pay }, toggle }
  temporal pays = weakFair(pay, (paid, open)) implies eventually(paid)
}
