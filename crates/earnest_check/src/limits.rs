use crate::helpers::Helpers;
use std::time::Duration;

/// How far a search may go and what it keeps.
///
/// The default is the CLI's defaults on one thread: five million states, no time limit, no
/// graph and every rule.
#[derive(Clone, Debug)]
pub struct Limits {
    /// The most distinct states to count. The search stops as unfinished when it finds one
    /// more. The rules of every counted state are judged, also of the states it found last
    /// and tried no action from.
    pub max_states: usize,
    /// How long the search may run before it stops as unfinished. `None` is no limit.
    pub time_limit: Option<Duration>,
    /// Worker threads for a level with work enough for two parts: 2,048 states or more, or
    /// fewer whose states took long to expand in the level before, as when each state tries
    /// many inputs. 0 counts as 1. The outcome is the same for every number. A search left
    /// with one thread, no helper being idle, that keeps no graph takes a level of 16,384
    /// states or more in parts on its own, in rounds of 32 parts, half the most parts it cuts a
    /// level into: it merges what a round found before it takes the next, so it holds the new
    /// states of half of its widest levels at most, and it takes the next round and the next
    /// level with the buffers of the last. Several threads take a level in parts in one round,
    /// short of the state limit, and hold its new states until they merge them.
    pub jobs: usize,
    /// Idle threads the search may borrow, on top of `jobs`, for the parts of each level it
    /// spreads. It borrows when it starts to spread a level and whenever a thread is about to
    /// take a part, while it sees more parts left to take than threads at work. Near the state
    /// limit it may spread a level in rounds, merging what it found before it hands out the
    /// rest, and it borrows no thread to take a part while it holds the rest back. A thread
    /// goes back as soon as it finds no part to take. A search that keeps no graph may also
    /// borrow threads to merge a level. It takes a thread only for a share of new states large
    /// enough to pay for the thread, its `jobs` first, so a level with few new states merges
    /// on one thread and borrows none. Searches that run side by side share one handle, so a
    /// thread one of them leaves idle goes to another. The outcome is the same whatever is
    /// borrowed.
    pub helpers: Option<Helpers>,
    /// Whether [`crate::Outcome::graph`] is filled. It costs memory for every arrow, so leave
    /// it off unless a chart is wanted. With [`Self::outline`] the search fills
    /// [`crate::Outcome::outline`] instead.
    pub keep_graph: bool,
    /// Whether a search that keeps its graph fills [`crate::Outcome::outline`] in place of
    /// [`crate::Outcome::graph`], which it then leaves `None`. An outline holds what a chart
    /// needs in far less memory than the graph. When no outline can be made, as for a search of
    /// 2^32 states or more or a graph of more than 2^32 edges, the search fills
    /// [`crate::Outcome::graph`] instead. Nothing is filled without [`Self::keep_graph`].
    pub outline: bool,
    /// Check only the rule with this id, and the numbered parts of a rule that is several,
    /// `id#2`, `id#3` and so on. The others are not evaluated and are left out of
    /// [`crate::Outcome::rules`]. An id no rule has leaves no rule checked.
    pub only_rule: Option<String>,
}

pub(crate) fn asked(only_rule: Option<&str>, id: &str) -> bool {
    only_rule.is_none_or(|wanted| {
        id == wanted
            || id
                .strip_prefix(wanted)
                .and_then(|rest| rest.strip_prefix('#'))
                .is_some_and(|number| {
                    !number.is_empty() && number.bytes().all(|digit| digit.is_ascii_digit())
                })
    })
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_states: 5_000_000,
            time_limit: None,
            jobs: 1,
            helpers: None,
            keep_graph: false,
            outline: false,
            only_rule: None,
        }
    }
}

/// The stack, in bytes, a thread that evaluates a model needs: an expression a file nests a
/// thousand levels deep is evaluated one frame per level, so a thread started for the
/// search or for a model should be given this much.
pub const WORKER_STACK: usize = 256 * 1024 * 1024;

pub(crate) const TIME_LIMIT_REASON: &str = "time limit of";

/// Whether `reason`, the [`crate::Outcome::reason`] of a search, says the time limit stopped it.
///
/// A search that a state limit stopped is the same on every run, one that a time limit stopped
/// is not: it ends wherever the clock caught it.
#[must_use]
pub fn stopped_by_time_limit(reason: &str) -> bool {
    reason.starts_with(TIME_LIMIT_REASON)
}
