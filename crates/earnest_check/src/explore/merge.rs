use super::part::{Entry, Part, Target, row_at};
use super::workers::Crew;
use super::{Arrow, Search};
use crate::expand::Mark;
use crate::store::{Found, Fresh, Placed};

const ROWS_WORTH_A_MERGE_THREAD: usize = 4096;

#[derive(Debug, PartialEq, Eq)]
struct MergeThreads {
    own: usize,
    borrowed: usize,
}

fn merge_threads(rows: usize, jobs: usize) -> MergeThreads {
    let worth = (rows / ROWS_WORTH_A_MERGE_THREAD).max(1);
    let own = jobs.clamp(1, worth);
    MergeThreads {
        own,
        borrowed: worth - own,
    }
}

#[derive(Default)]
pub(super) struct Judged {
    pub(super) marks: Vec<(usize, Mark)>,
    pub(super) through: usize,
}

impl Judged {
    pub(super) fn add(&mut self, marks: impl IntoIterator<Item = (usize, Mark)>, through: usize) {
        self.marks.extend(marks);
        self.through = self.through.max(through);
    }
}

impl Search<'_> {
    pub(super) fn merge_any(&mut self, parts: Vec<Part>) -> Judged {
        let simple = !self.limits.keep_graph && parts.iter().all(|part| part.raws.is_empty());
        if simple && parts.len() > 1 && self.merge_spread(&parts) {
            let mut judged = Judged::default();
            for part in parts {
                judged.add(part.marks, part.first.saturating_add(part.expanded));
            }
            return judged;
        }
        self.merge(parts)
    }

    fn merge_spread(&mut self, parts: &[Part]) -> bool {
        let fresh: Vec<Fresh<'_>> = parts
            .iter()
            .map(|part| Fresh {
                rows: &part.rows,
                hashes: &part.hashes,
                froms: &part.froms,
                by_shard: &part.by_shard,
                shard_starts: &part.shard_starts,
            })
            .collect();
        let rows = parts.iter().map(|part| part.hashes.len()).sum();
        let threads = merge_threads(rows, self.limits.jobs);
        let crew = Crew {
            own: threads.own,
            helpers: self.limits.helpers.as_ref(),
            most_borrowed: threads.borrowed,
        };
        let most = self.limits.max_states;
        self.store.place_level(&fresh, &crew, most).is_some()
    }

    fn merge(&mut self, parts: Vec<Part>) -> Judged {
        let mut judged = Judged::default();
        let most = self.limits.max_states;
        let words = self.store.layout().words();
        let graph = self.limits.keep_graph;
        for part in parts {
            let mut marks = part.marks.into_iter().peekable();
            let mut raws = part.raws.into_iter();
            let mut listed = part.entries.iter().copied().peekable();
            let mut fresh = 0..part.hashes.len();
            let froms = &part.froms;
            let mut next = || {
                if graph {
                    return listed.next();
                }
                let due = |entry: &Entry| matches!(entry.target, Target::Raw { fresh_before } if fresh_before <= fresh.start);
                listed.next_if(due).or_else(|| {
                    let at = fresh.next()?;
                    Some(Entry {
                        from: froms.get(at).copied().unwrap_or(0),
                        action: 0,
                        target: Target::Fresh(at),
                    })
                })
            };
            while let Some(entry) = next() {
                judged.add(
                    std::iter::from_fn(|| marks.next_if(|(id, _)| *id <= entry.from)),
                    entry.from.saturating_add(1),
                );
                let parent = Some(entry.from);
                let placed = match entry.target {
                    Target::Known(to) => Placed::Known(Found { id: Some(to) }),
                    Target::Fresh(at) => {
                        let hash = part.hashes.get(at).copied().unwrap_or(0);
                        self.store
                            .place(row_at(&part.rows, words, at), hash, parent, most)
                    }
                    Target::Raw { .. } => {
                        let Some(values) = raws.next() else {
                            continue;
                        };
                        self.store.place_values(values, parent, most)
                    }
                };
                let to = match placed {
                    Placed::Known(found) => found.id,
                    Placed::New(to) => Some(to),
                    Placed::Full => {
                        self.reason = Some(self.limit_reason());
                        return judged;
                    }
                };
                if graph && let Some(to) = to {
                    self.arrows.push(Arrow {
                        from: entry.from,
                        to,
                        action: entry.action,
                    });
                }
            }
            judged.add(marks, part.first.saturating_add(part.expanded));
        }
        judged
    }
}

#[cfg(test)]
mod tests {
    use super::{MergeThreads, ROWS_WORTH_A_MERGE_THREAD, merge_threads};

    const ROWS: usize = ROWS_WORTH_A_MERGE_THREAD;

    fn threads(own: usize, borrowed: usize) -> MergeThreads {
        MergeThreads { own, borrowed }
    }

    #[test]
    fn a_level_of_fewer_rows_than_one_thread_is_worth_merges_on_one_thread() {
        assert_eq!(merge_threads(0, 4), threads(1, 0), "no rows");
        assert_eq!(merge_threads(600, 10), threads(1, 0), "a narrow level");
        assert_eq!(
            merge_threads(ROWS - 1, 10),
            threads(1, 0),
            "just short of one thread"
        );
        assert_eq!(
            merge_threads(2 * ROWS - 1, 10),
            threads(1, 0),
            "just short of two threads"
        );
    }

    #[test]
    fn a_merge_takes_a_thread_for_every_full_share_of_rows_and_its_own_threads_first() {
        assert_eq!(merge_threads(2 * ROWS, 10), threads(2, 0), "two shares");
        assert_eq!(
            merge_threads(10 * ROWS, 4),
            threads(4, 6),
            "more shares than workers"
        );
        assert_eq!(
            merge_threads(10 * ROWS, 10),
            threads(10, 0),
            "a share for every worker"
        );
        assert_eq!(
            merge_threads(40 * ROWS, 10),
            threads(10, 30),
            "four shares a worker"
        );
    }

    #[test]
    fn no_worker_threads_merge_as_one() {
        assert_eq!(merge_threads(ROWS, 0), threads(1, 0), "one share");
        assert_eq!(
            merge_threads(3 * ROWS, 0),
            threads(1, 2),
            "the rest is borrowed"
        );
    }
}
