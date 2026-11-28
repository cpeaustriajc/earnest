module FairnessInAnd {
  type State = { n: int }
  var st: State
  action init = st' = { n: 0 }
  action tick = all { st.n < 2, st' = { n: st.n + 1 } }
  action stay = st' = st
  action step = any { tick, stay }
  temporal fair = and {
    tick.weakFair(st),
  }
  temporal ReachesTwo = and {
    always(st.n <= 2),
    fair.implies(eventually(st.n == 2)),
  }
}
