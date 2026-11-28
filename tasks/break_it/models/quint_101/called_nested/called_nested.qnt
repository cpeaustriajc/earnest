/// @check
module calledNested {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run inner = nondet b = 0.to(3).oneOf() init.then(all { b == 2, x' = b })
  run outerMiddle = nondet a = 0.to(3).oneOf() init.then(inner).then(all { a == 1, x' = x + a })
  run lastTest = init.then(outerMiddle).then(all { x == 3, x' = x })
  run middleFalseTest = init.then(outerMiddle).then(all { x == 4, x' = x })
  run twoTest = nondet a = 0.to(3).oneOf() nondet c = 0.to(3).oneOf() init.then(all { a + c == 5, x' = a + c })
  run twoCalledTest = init.then(twoTest).then(all { x == 5, x' = x })
}
