// Records, sets, maps, str fields, a constant set, fold/filter/exists/forall.
module bank {
  type Entry = { owner: str, balance: int, tags: Set[str] }

  pure val Names = Set("alice", "bob")
  pure val Amounts = Set(10, 20)

  var byName: str -> Entry
  var log: List[str]
  var total: int

  action init = all {
    byName' = Map(),
    log' = [],
    total' = 0,
  }

  action open(name: str): bool = all {
    not(byName.keys().contains(name)),
    byName' = byName.put(name, { owner: name, balance: 0, tags: Set() }),
    log' = log.append(name),
    total' = total,
  }

  action deposit(name: str, amount: int): bool = all {
    byName.keys().contains(name),
    total < 40,
    byName' = byName.setBy(name, e => { ...e, balance: e.balance + amount, tags: e.tags.union(Set("paid")) }),
    log' = log,
    total' = total + amount,
  }

  action step = {
    nondet name = Names.oneOf()
    nondet amount = Amounts.oneOf()
    any { open(name), deposit(name, amount) }
  }

  val sumMatches = byName.keys().fold(0, (acc, k) => acc + byName.get(k).balance) == total
  val ownersMatch = byName.keys().forall(k => byName.get(k).owner == k)
  val someRich = byName.keys().filter(k => byName.get(k).balance > 0).size() >= 0
  val noOverdraft = not(byName.keys().exists(k => byName.get(k).balance < 0))
}
