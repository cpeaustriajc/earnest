// Modules of runs only on a plain import of a spec with variables: `quint test` plays the
// spec's runs too, as `Main::specTest` and `specTest`.
module Spec {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
  run specTest = init.then(step).expect(n == 1)
  run specOverTest = init.then(4.reps(_ => step))
}

module PlainAs {
  import Spec as Main
  run ownTest = Main::init.then(Main::step).expect(Main::n == 1)
}

module PlainStar {
  import Spec.*
  run ownTest = init.then(step).expect(n == 1)
}

module PlainFrom {
  import Other as O from "./Other"
  import Spec as S
  run bothTest = all { O::init, S::init }.then(all { O::step, S::step }).expect(O::m + S::n == 2)
}

module Lib {
  pure def twice(x: int): int = 2 * x
}

module Helped {
  import Lib.*
  var h: int
  action init = h' = 0
  action step = h' = twice(h) + 1
}

module PlainNested {
  import Helped as H
  run nestedTest = H::init.then(H::step).expect(H::h == 1)
}
