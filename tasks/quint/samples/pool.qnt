/// @check
/// Records wait in a set until a nondeterministic pick finishes them.
module pool {
  type Job = { id: int, cost: int }

  val JOBS = Set({ id: 1, cost: 2 }, { id: 2, cost: 3 })

  var waiting: Set[Job]
  var done: Set[int]

  action init = {
    nondet start = JOBS.powerset().oneOf()
    all {
      waiting' = start,
      done' = Set(),
    }
  }

  action submit(job: Job): bool = all {
    not(waiting.contains(job)),
    not(done.contains(job.id)),
    waiting' = waiting.union(Set(job)),
    done' = done,
  }

  action finish(job: Job): bool = all {
    waiting.contains(job),
    waiting' = waiting.exclude(Set(job)),
    done' = done.union(Set(job.id)),
  }

  action step = any {
    { nondet job = JOBS.oneOf()
      submit(job) },
    { nondet job = waiting.oneOf()
      finish(job) },
  }

  /// @check
  val notBoth = waiting.forall(job => not(done.contains(job.id)))

  /// @check reachable
  val allDone = done == Set(1, 2)

  run drainsTest = init.then(step).then(step).expect(waiting.size() <= 2)
}
