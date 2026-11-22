import sys, os
root = sys.argv[1]
C = os.path.join(root, "crates/earnest_check/src")


def sub(path, old, new, count=1):
    p = os.path.join(root, path) if not path.startswith("/") else path
    s = open(p).read()
    assert s.count(old) == count, (path, old[:80], s.count(old))
    open(p, "w").write(s.replace(old, new))


E = "crates/earnest_check/src/explore.rs"
sub(E, "    judged: usize,\n    last_spread_merge", "    pub(crate) judged: usize,\n    last_spread_merge")
sub(E, """            let found_before = self.store.len();
            let level = self.level(""", """            let found_before = self.store.len();
            eprintln!("INST level {}..{}", frontier.start, frontier.end);
            let level = self.level(""")
sub(E, """        let mut helpers = self.limits.helpers.as_ref();
        let mut judged = Judged::default();
        let mut rest = frontier.clone();
""", """        let mut helpers = self.limits.helpers.as_ref();
        let mut judged = Judged::default();
        let mut rest = frontier.clone();
        let mut deadline = deadline;
        let force_level: Option<usize> = std::env::var("FORCE_LEVEL").ok().and_then(|v| v.parse().ok());
        let force_round: usize = std::env::var("FORCE_ROUND").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
        let force_delay: u64 = std::env::var("FORCE_DELAY_US").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        let forced_here = asked == Asked::Everything && force_level.is_none_or(|l| l == frontier.start);
        let mut round = 1usize;
        if forced_here && force_round <= 1 {
            eprintln!("FORCE stop at round 1 level {:?}", frontier);
            deadline = Some(Instant::now() + std::time::Duration::from_micros(force_delay));
        }
""")
sub(E, """                Some(expansion) if expansion.crowded => helpers = None,""",
    """                Some(expansion) if expansion.crowded => { eprintln!("INST crowded {:?}", frontier); helpers = None }""")
sub(E, """                    if expansion.stopped && asked == Asked::Everything {
                        self.undo_level(&frontier);""", """                    if expansion.stopped && asked == Asked::Everything {
                        eprintln!("INST stopped spread round {round} level {:?} at {:.3}s", frontier, crate::explore::since());
                        self.undo_level(&frontier);""")
sub(E, """                        Some(left) if self.reason.is_none() => {
                            let taken = left.start.saturating_sub(rest.start);""", """                        Some(left) if self.reason.is_none() => {
                            round += 1;
                            eprintln!("INST round {round} level {:?} left {:?} store {} at {:.3}s", frontier, left, self.store.len(), crate::explore::since());
                            if forced_here && round >= force_round {
                                eprintln!("FORCE stop at round {round} level {:?}", frontier);
                                deadline = Some(Instant::now() + std::time::Duration::from_micros(force_delay));
                            }
                            let taken = left.start.saturating_sub(rest.start);""")
sub(E, """            if here.stopped && asked == Asked::Everything {
                self.undo_level(&frontier);""", """            if here.stopped && asked == Asked::Everything {
                eprintln!("INST stopped here round {round} level {:?} at {:.3}s", frontier, crate::explore::since());
                self.undo_level(&frontier);""")
sub(E, """            if here.handed {
                spread =""", """            if here.handed {
                eprintln!("INST handed {} level {:?}", here.through, frontier);
                spread =""")

R = "crates/earnest_check/src/report.rs"
sub(R, """    pub(crate) fn outcome(&self) -> Outcome {
""", """    pub(crate) fn outcome(&self) -> Outcome {
        let n = self.store.len();
        let mut bad = Vec::new();
        if let Some(id) = self.stuck { if id >= n { bad.push(format!("stuck {id}")); } }
        if let Some((id, _)) = self.outside { if id >= n { bad.push(format!("outside {id}")); } }
        if let Some((id, _, _)) = self.trouble { if id >= n { bad.push(format!("trouble {id}")); } }
        for first in self.first_broken.iter().flatten() { if *first >= n { bad.push(format!("broken {first}")); } }
        for arrow in &self.arrows { if arrow.from >= n || arrow.to >= n { bad.push(format!("arrow {}->{}", arrow.from, arrow.to)); break; } }
        if self.notes.len() > n { bad.push(format!("notes {} > {n}", self.notes.len())); }
        if self.judged > n { bad.push(format!("judged {} > {n}", self.judged)); }
        let through = crate::explore::EXPANDED_THROUGH.load(std::sync::atomic::Ordering::SeqCst);
        if let Some(id) = self.stuck { if id >= through { bad.push(format!("stuck {id} unexpanded from {through}")); } }
        if self.limits.keep_graph {
            let arrows_unexpanded = self.arrows.iter().filter(|a| a.from >= through).count();
            if arrows_unexpanded > 0 { bad.push(format!("arrows from unexpanded {arrows_unexpanded}")); }
            let marked = (through.min(self.notes.len())..self.notes.len()).filter(|id| self.notes.get(*id).is_some_and(|note| note.end || note.stuck)).count();
            if marked > 0 { bad.push(format!("end/stuck marks on unexpanded {marked}")); }
        }
        eprintln!("CHECK through {through} store {n} judged {} notes {} arrows {} bad {:?}", self.judged, self.notes.len(), self.arrows.len(), bad);
""")

W = "apps/earnest/src/run.rs"
sub(W, """        let limits = Limits {
            max_states: bounds.max_states,
            time_limit: bounds.time_limit,
            jobs: 1,
            helpers: Some(helpers.clone()),""", """        let own: Option<usize> = std::env::var("EARNEST_OWN").ok().and_then(|v| v.parse().ok());
        let limits = Limits {
            max_states: bounds.max_states,
            time_limit: bounds.time_limit,
            jobs: own.unwrap_or(1),
            helpers: if own.is_some() { None } else { Some(helpers.clone()) },""")
sub(E, """const STATES_BETWEEN_CLOCK_READS: usize = 64;""", """const STATES_BETWEEN_CLOCK_READS: usize = 64;
pub(crate) static EXPANDED_THROUGH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);
pub(crate) static BEGAN: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
pub(crate) fn since() -> f64 { BEGAN.lock().unwrap().map_or(0.0, |b| b.elapsed().as_secs_f64()) }""")
sub(E, """        let began = Instant::now();
""", """        let began = Instant::now();
        *BEGAN.lock().unwrap() = Some(began);
""")
sub(E, """        let unjudged = self.judged..self.store.len();""", """        EXPANDED_THROUGH.store(self.judged, std::sync::atomic::Ordering::SeqCst);
        let unjudged = self.judged..self.store.len();""")
sub("crates/earnest_check/src/explore/workers.rs", """        if !expansion.stopped {
            expansion.left_over = level""", """        if level.asked == Asked::Everything { eprintln!("INST spread {:?} fit {:?} fewest {} kept {} parts {}/{} last {:?}", frontier, level.kept_rows_that_fit, level.fewest_states, shared.kept_rows.load(Ordering::SeqCst), expansion.parts.len(), parts, (self.last_spread_merge.kept, self.last_spread_merge.new)); }
        if !expansion.stopped {
            expansion.left_over = level""")
print("patched")
