use super::part::{Entry, Part, Target, row_at};
use super::workers::Crew;
use super::{Arrow, Search};
use crate::expand::Mark;
use crate::store::{Found, Fresh, Placed};

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
        let crew = Crew {
            own: self.limits.jobs.max(1),
            helpers: self.limits.helpers.as_ref(),
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
