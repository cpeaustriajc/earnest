// Checks of runs only: a marked module with no `init` or `step` plays its runs.
module Counter {
  var n: int
  action begin = n' = 0
  action up = n' = n + 1
}

/// @check
/// The runs of the counter, with no search.
module Plays {
  import Counter.*
  run upTest = begin.then(up).expect(n == 1)
  /// @check
  /// Two steps up from zero reach two.
  run twice = begin.then(up).then(up).expect(n == 2)
  /// @check broken
  /// One step up does not reach two.
  run tooFar = begin.then(up).expect(n == 2)
}

// Not marked, so not judged, though its run fails.
module Ignored {
  import Counter.*
  run failsTest = begin.then(up).expect(n == 5)
}
