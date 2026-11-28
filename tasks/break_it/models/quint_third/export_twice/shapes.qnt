module TwiceImported {
  import Spec as A from "./Lib"
  import Spec as B from "./Lib"
  run ownTest = A::init.then(A::step).expect(A::v == 1)
}
module Transitive {
  import Deep as D from "./Lib"
  import Helped as H from "./Lib"
  run ownTest = H::init.then(H::step).expect(H::w == 1)
}
module OneName {
  import Spec.specTest from "./Lib"
  import Spec.init from "./Lib"
  import Spec.step from "./Lib"
  import Spec.v from "./Lib"
  run ownTest = init.then(step).expect(v == 1)
}
module Inst {
  import Param(K = 3) as C from "./Lib"
  run ownTest = C::init.then(C::step).expect(C::c == 4)
}
module InstStar {
  import Param(K = 3).* from "./Lib"
  run ownTest = init.then(step).expect(c == 4)
}
module Exported {
  import Spec as S from "./Lib"
  export S.*
  run ownTest = init.then(step).expect(v == 1)
}
module Mixed {
  import Spec as S from "./Lib"
  import Param(K = 1) as P from "./Lib"
  run mixTest = all { S::init, P::init }.then(all { S::step, P::step }).expect(S::v + P::c == 3)
  run mixWrongTest = all { S::init, P::init }.then(all { S::step, P::step }).expect(S::v + P::c == 4)
}
