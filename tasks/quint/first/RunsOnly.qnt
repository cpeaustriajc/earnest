// The owner's sample: a module of runs only on a plain import of a module.
module CounterSpec {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
}
module RunsOnly {
  import CounterSpec as Main
  run stepsOnceTest = Main::init.then(Main::step).expect(Main::n == 1)
}
