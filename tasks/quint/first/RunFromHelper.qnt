module RunFromHelper {
  var n: int
  action initWith(start: int): bool = n' = start
  action init = initWith(0)
  action inc = n' = n + 1
  action step = inc
  val Small = n <= 3
  run fromHelperTest = initWith(2).then(inc).expect(n == 3)
}
