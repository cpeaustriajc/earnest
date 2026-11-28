module FairnessVal {
  type State = { n: int }
  var st: State
  action init = st' = { n: 0 }
  action tick = all { st.n < 2, st' = { n: st.n + 1 } }
  action stay = st' = st
  action step = any { tick, stay }
  temporal fair = and {
    tick.weakFair(Set(st)),
  }
  temporal ReachesTwo = fair.implies(eventually(st.n == 2))
}
