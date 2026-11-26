module Spec {
  var v: int
  action init = v' = 0
  action step = v' = v + 1
  run specTest = init.then(step).expect(v == 1)
  run specOverTest = init.then(step).expect(v == 2)
}
module Exporter {
  import Spec.specTest
  import Spec.init
  import Spec.step
  import Spec.v
  export Spec.specTest
}
