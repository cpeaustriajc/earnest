/// A module's doc.
/// Over two lines.
module Docs {
  ///@check
  val checked = true
  /// @check
  ///
  val blankEnds = true
  // not a doc
  val lineComment = true
  /* not a doc either */
  val blockComment = true
  ///
  val onlyBlank = true
  /// on a const
  const C: int
  /// on a var
  var v: int
  /// on a sum type: it lands on the last constructor
  type Answer = Yes | No
  /// on a run
  run aRun = v' = 1
  ///trimmed? no: the first four characters go
  val fourth = 1
  //// four slashes
  val slashes = 1
}

module Other {
  /// on an instance
  import Docs(C = 1) as D
  /// on a destructuring: it lands on the last name
  val (a, b) = (1, 2)
}
