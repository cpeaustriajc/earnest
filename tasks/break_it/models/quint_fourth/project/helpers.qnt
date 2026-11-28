module helpers {
  import types.* from "./types"
  pure def idle: Peer = { status: "idle", holds: false, sent: 0 }
  pure def fits(used: int, room: int): bool = used < room
  pure def holderIs(h: Holder, n: Node): bool = match h { | Held(m) => m == n | Nobody => false }
  pure val NoGrants = 0
}
