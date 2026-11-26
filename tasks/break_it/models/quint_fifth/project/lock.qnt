module lock {
  import types.* from "./types"
  import helpers.* from "./helpers"

  const NODES: Set[Node]
  const MAX_SENT: int
  const CAPACITY: int

  assume roomy = CAPACITY >= 2 and MAX_SENT >= 1
  assume named = NODES.forall(n => n != "")

  var peers: Node -> Peer
  var net: List[Msg]
  var waiting: List[Node]
  var holder: Holder
  var grants: int

  val vars = (peers, net, waiting, holder, grants)

  def send(m: Msg): List[Msg] = net.append(m)
  def isWaiting(n: Node): bool = waiting.indices().exists(i => waiting[i] == n)

  action init = all {
    peers' = NODES.mapBy(_ => idle),
    net' = [],
    waiting' = [],
    holder' = Nobody,
    grants' = NoGrants,
  }

  action request = {
    nondet n = NODES.filter(n => peers.get(n).status == "idle" and peers.get(n).sent < MAX_SENT).oneOf()
    all {
      fits(net.length(), CAPACITY),
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
    fits(net.length(), CAPACITY),
    peers' = peers,
    net' = send(Grant(waiting.head())),
    waiting' = waiting.tail(),
    holder' = Held(waiting.head()),
    grants' = grants + 1,
  }

  action release = {
    nondet n = NODES.filter(n => peers.get(n).holds).oneOf()
    all {
      fits(net.length(), CAPACITY),
      peers' = peers.setBy(n, p => { ...p, status: "idle", holds: false }),
      net' = send(Release(n)),
      waiting' = waiting,
      holder' = holder,
      grants' = grants,
    }
  }

  action step = any { request, deliver, grant, release }

  val mutex = NODES.filter(n => peers.get(n).holds).size() <= 1
  val holderKnown = NODES.forall(n => peers.get(n).holds implies holderIs(holder, n))
  val bounded = net.length() <= CAPACITY
  val grantsBounded = grants <= NODES.size() * MAX_SENT
  val neverTwoGrants = grants <= 1

  temporal served = weakFair(step, vars) implies always(waiting.length() > 0 implies eventually(holder != Nobody))
  temporal everyoneIdle = weakFair(step, vars) implies always(eventually(NODES.forall(n => peers.get(n).status == "idle")))
  temporal drains = weakFair(step, vars) implies always(eventually(net.length() == 0))
  temporal someGrant = weakFair(step, vars) implies eventually(grants > 0)

  action requestBy(n: Node): bool = all {
    peers.get(n).status == "idle",
    peers.get(n).sent < MAX_SENT,
    fits(net.length(), CAPACITY),
    peers' = peers.setBy(n, p => { ...p, status: "waiting", sent: p.sent + 1 }),
    net' = send(Request(n)),
    waiting' = waiting,
    holder' = holder,
    grants' = grants,
  }

  action releaseBy(n: Node): bool = all {
    peers.get(n).holds,
    fits(net.length(), CAPACITY),
    peers' = peers.setBy(n, p => { ...p, status: "idle", holds: false }),
    net' = send(Release(n)),
    waiting' = waiting,
    holder' = holder,
    grants' = grants,
  }

  action stepEach = any {
    nondet n = NODES.oneOf()
    requestBy(n),
    deliver,
    grant,
    nondet n = NODES.oneOf()
    releaseBy(n),
  }

  action startWith(n: Node): bool = all {
    peers' = NODES.mapBy(m => if (m == n) { ...idle, status: "waiting", sent: 1 } else idle),
    net' = [Request(n)],
    waiting' = [],
    holder' = Nobody,
    grants' = 0,
  }

  val Guard = NODES.size() > 2
  val GuardedBreak = Guard implies neverTwoGrants

  temporal fairEach = and {
    NODES.forall(n => weakFair(requestBy(n), (peers, net))),
    weakFair(deliver, vars),
    weakFair(grant, vars),
    NODES.forall(n => weakFair(releaseBy(n), (peers, net))),
  }
  temporal everyoneRequests = fairEach.implies(NODES.forall(n => eventually(peers.get(n).sent > 0)))
  temporal everyoneHolds = fairEach.implies(NODES.forall(n => eventually(peers.get(n).holds)))
  temporal eachDrains = and {
    always(grants <= NODES.size() * MAX_SENT),
    fairEach.implies(always(eventually(net.length() == 0))),
    weakFair(deliver, vars).implies(eventually(grants > 0)),
  }
  action requestTimes(n: Node, times: int): bool = all {
    peers.get(n).status == "idle",
    peers.get(n).sent < MAX_SENT,
    fits(net.length() + times - 1, CAPACITY),
    peers' = peers.setBy(n, p => { ...p, status: "waiting", sent: p.sent + 1 }),
    net' = if (times == 1) send(Request(n)) else send(Request(n)).append(Request(n)),
    waiting' = waiting,
    holder' = holder,
    grants' = grants,
  }

  action settle = match holder {
    | Nobody => all { waiting.length() == 0, net.length() > 0, net.head() != Grant("a"), peers' = peers, net' = net.tail(), waiting' = waiting, holder' = holder, grants' = grants }
    | Held(n) => all { peers.get(n).status == "idle", net.length() == 0, peers' = peers, net' = net, waiting' = waiting, holder' = Nobody, grants' = grants }
  }

  action stepPick = any {
    nondet p = tuples(NODES, Set(1, 2)).oneOf()
    requestTimes(p._1, p._2),
    deliver,
    grant,
    nondet n = NODES.oneOf()
    releaseBy(n),
    settle,
  }

  temporal fairPick = and {
    NODES.forall(n => weakFair(requestTimes(n, 1), (peers, net))),
    weakFair(deliver, vars),
    weakFair(grant, vars),
    NODES.forall(n => weakFair(releaseBy(n), (peers, net))),
  }
  temporal pickRequests = fairPick.implies(NODES.forall(n => eventually(peers.get(n).sent > 0)))
  temporal pickHolds = fairPick.implies(NODES.forall(n => eventually(peers.get(n).holds)))
  temporal pickSettles = and {
    always(grants <= NODES.size() * MAX_SENT),
    and { fairPick, weakFair(settle, vars) }.implies(always(eventually(holder == Nobody))),
    (grants == 0).implies(weakFair(settle, vars).implies(eventually(net.length() == 0))),
  }

  val open = grants < NODES.size()
  action idleStep = all { peers' = peers, net' = net, waiting' = waiting, holder' = holder, grants' = grants }
  action stepGuarded = any {
    all { open, any {
      nondet p = tuples(NODES, Set(1, 2)).oneOf()
      all { peers.get(p._1).sent < MAX_SENT, requestTimes(p._1, p._2) },
      all { net.length() > 0, deliver },
      all { waiting.length() < 2, grant },
      nondet n = NODES.oneOf()
      all { peers.get(n).holds, releaseBy(n) },
      settle,
    } },
    all { not(open), any { deliver, idleStep } },
  }
  temporal fairG = and {
    NODES.forall(n => weakFair(requestTimes(n, 1), (peers, net))),
    weakFair(deliver, vars),
    weakFair(grant, vars),
    NODES.forall(n => weakFair(releaseBy(n), (peers, net))),
  }
  temporal gRequests = fairG.implies(NODES.forall(n => eventually(peers.get(n).sent > 0)))
  temporal gHolds = fairG.implies(NODES.forall(n => eventually(peers.get(n).holds)))
  temporal gDrains = and { weakFair(stepGuarded, vars), fairG }.implies(always(eventually(net.length() == 0)))
  temporal gServed = and { fairG, weakFair(settle, vars) }.implies(always(waiting.length() > 0 implies eventually(holder != Nobody)))
  temporal gIdle = and { weakFair(stepGuarded, vars), fairG }.implies(eventually(NODES.forall(n => peers.get(n).status != "waiting")))
}
