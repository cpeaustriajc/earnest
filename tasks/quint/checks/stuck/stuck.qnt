/// @check ok
module stuck {
  var y: int
  action init = y' = 0
  action step = all { y < 2, y' = y + 1 }
  val ok = y <= 2
  run toTwoTest = init.then(step).then(step).expect(y == 2)
  run thirdTest = init.then(3.reps(_ => step)).fail()
  run withNondetTest = {
    nondet k = 1.to(2).oneOf()
    init.then(k.reps(_ => step)).expect(y == k)
  }
  run assertedTest = init.then(all { assert(y == 0), step }).expect(y == 1)
  run wrongExpectTest = init.then(step).expect(y == 5)
}
