module lock {
  import types.* from "./types"

  const NODES: Set[Node]
  const MAX_SENT: int
  const CAPACITY: int

  var peers: Node -> Peer
  var net: List[Msg]
  var waiting: List[Node]
  var holder: Holder
  var grants: int

  val vars = (peers, net, waiting, holder, grants)

  pure def idle: Peer = { status: "idle", holds: false, sent: 0 }
  def send(m: Msg): List[Msg] = net.append(m)
  def isWaiting(n: Node): bool = waiting.indices().exists(i => waiting[i] == n)

  action init = all {
    peers' = NODES.mapBy(_ => idle),
    net' = [],
    waiting' = [],
    holder' = Nobody,
    grants' = 0,
  }

  action request = {
    nondet n = NODES.filter(n => peers.get(n).status == "idle" and peers.get(n).sent < MAX_SENT).oneOf()
    all {
      net.length() < CAPACITY,
      peers' = peers.setBy(n, p => { ...p, status: "waiting", sent: p.sent + 1 }),
      net' = send(Request(n)),
      waiting' = waiting,
      holder' = holder,
      grants' = grants,
    }
  }

  action deliver = all {
    net.length() > 0,
    match net.head() {
      | Request(n) => all {
          peers' = peers,
          net' = net.tail(),
          waiting' = if (isWaiting(n)) waiting else waiting.append(n),
          holder' = holder,
          grants' = grants,
        }
      | Grant(n) => all {
          peers' = peers.setBy(n, p => { ...p, status: "critical", holds: true }),
          net' = net.tail(),
          waiting' = waiting,
          holder' = holder,
          grants' = grants,
        }
      | Release(n) => all {
          peers' = peers,
          net' = net.tail(),
          waiting' = waiting,
          holder' = if (holder == Held(n)) Nobody else holder,
          grants' = grants,
        }
    }
  }

  action grant = all {
    holder == Nobody,
    waiting.length() > 0,
    net.length() < CAPACITY,
    peers' = peers,
    net' = send(Grant(waiting.head())),
    waiting' = waiting.tail(),
    holder' = Held(waiting.head()),
    grants' = grants + 1,
  }

  action release = {
    nondet n = NODES.filter(n => peers.get(n).holds).oneOf()
    all {
      net.length() < CAPACITY,
      peers' = peers.setBy(n, p => { ...p, status: "idle", holds: false }),
      net' = send(Release(n)),
      waiting' = waiting,
      holder' = holder,
      grants' = grants,
    }
  }

  action step = any { request, deliver, grant, release }

  val mutex = NODES.filter(n => peers.get(n).holds).size() <= 1
  val holderKnown = NODES.forall(n => peers.get(n).holds implies holder == Held(n))
  val bounded = net.length() <= CAPACITY
  val grantsBounded = grants <= NODES.size() * MAX_SENT
  val neverTwoGrants = grants <= 1

  temporal served = weakFair(step, vars) implies always(waiting.length() > 0 implies eventually(holder != Nobody))
  temporal everyoneIdle = weakFair(step, vars) implies always(eventually(NODES.forall(n => peers.get(n).status == "idle")))
  temporal drains = weakFair(step, vars) implies always(eventually(net.length() == 0))
  temporal someGrant = weakFair(step, vars) implies eventually(grants > 0)
}
