// Every way a module of runs only brings in what its runs use.
module Local {
  var k: int
  action init = k' = 0
  action step = k' = k + 1
}

/// @check
module StarImport {
  import Local.*
  run starTest = init.then(step).expect(k == 1)
}

/// @check
module AsImport {
  import Local as L
  run asTest = L::init.then(L::step).expect(L::k == 1)
}

/// @check
module StarInstance {
  import Param(START = 2).* from "./Spec"
  run starInstanceTest = init.then(step).expect(m == 3)
}

/// @check
module AsInstance {
  import Param(START = 4) as P from "./Spec"
  run asInstanceTest = P::InitOn(1).then(P::step).expect(P::m == 2)
}

/// @check
module FromFile {
  import Plain as F from "./Spec"
  run fromFileTest = F::init.then(F::step).expect(F::n == 1)
}

/// @check
module Several {
  import Plain as F from "./Spec"
  import Param(START = 0) as A from "./Spec"
  import Param(START = 7) as B from "./Spec"
  run aTest = A::init.then(A::step).expect(A::m == 1)
  run bTest = B::init.then(B::step).expect(B::m == 8)
  run fTest = F::init.then(F::step).expect(F::n == 1)
}

/// @check
module OwnVariables {
  import Param(START = 0) as A from "./Spec"
  var seen: bool
  action start = all { A::init, seen' = false }
  action look = all { A::step, seen' = true }
  run ownTest = start.then(look).expect(seen and A::m == 1)
  run onlyOwnTest = all { seen' = true }.then(all { seen' = not(seen) }).expect(not(seen))
}

/// @check
module Mixed {
  import Local.*
  import Plain as F from "./Spec"
  import Param(START = 1) as A from "./Spec"
  var t: int
  run mixTest = all { k' = 0, F::n' = 0, A::m' = 2, t' = 0 }
    .then(all { step, F::step, A::step, t' = t + 1 })
    .expect(k + F::n + A::m + t == 6)
}
