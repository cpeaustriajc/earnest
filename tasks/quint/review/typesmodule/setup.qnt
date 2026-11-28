/// @check
module Setup {
  import light.* from "./light"
  import types.* from "./types"
  /// @check reachable
  val greenAtThree = color == Green and ticks == 3
}
