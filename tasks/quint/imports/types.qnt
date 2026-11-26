// The project's types: a node's name, the messages and a door's record.
module types {
  type Node = str
  type Msg = Open(Node) | Close(Node) | Ping
  type Door = { open: bool, count: int }
}
