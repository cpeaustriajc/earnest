module div {
  var x: int
  action init = x' = 0
  action step = x' = x
  run floorTest = (-7) / 2 == -4
  run truncTest = (-7) / 2 == -3
  run modPosTest = (-7) % 2 == 1
  run modNegTest = (-7) % 2 == -1
  run modNeg2Test = 7 % (-2) == 1
  run divNeg2Test = 7 / (-2) == -3
  run setEqTest = Set(1,2) == Set(2,1)
  run mapEqTest = Map(1 -> 2, 3 -> 4) == Map(3 -> 4, 1 -> 2)
  run recEqTest = { a: 1, b: 2 } == { b: 2, a: 1 }
  run divZeroTest = 1 / 0 == 0
}
