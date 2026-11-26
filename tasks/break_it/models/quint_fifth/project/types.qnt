module types {
  type Node = str
  type Msg = Request(Node) | Grant(Node) | Release(Node)
  type Holder = Nobody | Held(Node)
  type Peer = { status: str, holds: bool, sent: int }
}
