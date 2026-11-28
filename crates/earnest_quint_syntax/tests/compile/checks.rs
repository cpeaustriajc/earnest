use crate::compared::{Outcome, compared, fixtures, listed, root};
use std::env;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

struct Tally {
    equal: usize,
    refused: usize,
    differences: Vec<String>,
}

fn spawn_large<'scope, T: Send + 'scope>(
    scope: &'scope thread::Scope<'scope, '_>,
    work: impl FnOnce() -> T + Send + 'scope,
) -> thread::ScopedJoinHandle<'scope, T> {
    thread::Builder::new()
        .stack_size(earnest_quint_syntax::STACK_NEEDED)
        .spawn_scoped(scope, work)
        .expect("a comparing thread starts")
}

fn every(folder: &Path, base: &Path) -> Tally {
    let recorded = listed(folder).expect("the recordings are read");
    let mut tally = Tally {
        equal: 0,
        refused: 0,
        differences: Vec::new(),
    };
    let next = AtomicUsize::new(0);
    let threads = thread::available_parallelism().map_or(4, NonZeroUsize::get);
    let mut outcomes: Vec<(usize, Result<Outcome, String>)> = thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                spawn_large(scope, || {
                    let mut found = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(each) = recorded.get(at) else {
                            break found;
                        };
                        found.push((at, compared(each, base)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a comparing thread ends"))
            .collect()
    });
    outcomes.sort_by_key(|(at, _)| *at);
    for (at, outcome) in outcomes {
        let each = &recorded[at];
        match outcome {
            Ok(Outcome::Equal) => tally.equal += 1,
            Ok(Outcome::BothRefuse) => tally.refused += 1,
            Err(found) => tally
                .differences
                .push(format!("{}: {found}", each.check.args)),
        }
    }
    tally
}

fn said(tally: &Tally) -> String {
    format!(
        "{} equal, {} refused by both, {} differ",
        tally.equal,
        tally.refused,
        tally.differences.len()
    )
}

#[test]
fn every_check_of_the_repository_flattens_as_quint_compile_writes_it() {
    if crate::ran_on_a_stack_of(
        earnest_quint_syntax::STACK_NEEDED,
        every_check_of_the_repository_flattens_as_quint_compile_writes_it,
    ) {
        return;
    }
    let tally = every(&fixtures(), &root());
    println!("{}", said(&tally));
    assert!(
        tally.differences.is_empty(),
        "{}:\n{}",
        said(&tally),
        tally.differences.join("\n")
    );
    assert!(tally.equal >= 200, "{}", said(&tally));
}

#[test]
#[ignore = "needs a folder of recordings made outside the repository"]
fn every_check_of_a_folder_of_recordings_flattens_as_quint_compile_writes_it() {
    let folder =
        PathBuf::from(env::var("QUINT_COMPILED").expect("QUINT_COMPILED names the recordings"));
    let base = env::var("QUINT_COMPILED_BASE").map_or_else(|_| root(), PathBuf::from);
    let tally = every(&folder, &base);
    println!("{}", said(&tally));
    assert!(
        tally.differences.is_empty(),
        "{}",
        tally.differences.join("\n")
    );
}
