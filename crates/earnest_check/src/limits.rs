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
    /// Worker threads for a level of 512 states or more. 0 counts as 1. The outcome is the
    /// same for every number. While no thread is idle, one thread goes through every level
    /// state by state, unless the search has `helpers` and keeps no graph: then it takes a
    /// wide level in parts on its own, which holds the level's new states until it merges them.
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
    /// borrowed. With `jobs` at 1, a handle makes a search that keeps no graph spread a wide
    /// level even when no thread is idle, its own thread taking every part, so it needs more
    /// memory than with `None`, which keeps it state by state.
    pub helpers: Option<Helpers>,
    /// Whether [`crate::Outcome::graph`] is filled. It costs memory for every arrow, so leave
    /// it off unless a chart is wanted.
    pub keep_graph: bool,
    /// Check only the rule with this id. The others are not evaluated and are left out of
    /// [`crate::Outcome::rules`]. An id no rule has leaves no rule checked.
    pub only_rule: Option<String>,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_states: 5_000_000,
            time_limit: None,
            jobs: 1,
            helpers: None,
            keep_graph: false,
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
