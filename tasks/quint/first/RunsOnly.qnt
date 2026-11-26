// The owner's sample: a module of runs only on a plain import of a module.
module CounterSpec {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
}
/// @check
module RunsOnly {
  import CounterSpec as Main
  run stepsOnceTest = Main::init.then(Main::step).expect(Main::n == 1)
}

/// @check
/// `RunsOnly` with `init` and `step` named, as a check could name them.
module RunsOnlyNamed {
  import CounterSpec as Main
  action init = Main::init
  action step = Main::step
  run stepsOnceTest = Main::init.then(Main::step).expect(Main::n == 1)
}
