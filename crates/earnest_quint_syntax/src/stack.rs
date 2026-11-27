use crate::problem::Problem;
use earnest_threads::start;
use std::cell::Cell;
use std::panic;
use std::sync::mpsc;
use std::thread;

/// The stack of the thread the crate reads and checks a file on. Every recursive pass refuses a
/// file nested deeper than its bound before it can spend this.
pub(crate) const STACK: usize = 256 * 1024 * 1024;

thread_local! {
    static ON_LARGE_STACK: Cell<bool> = const { Cell::new(false) };
}

fn not_started(error: &std::io::Error) -> Problem {
    Problem {
        code: "unreadable".to_owned(),
        message: format!("earnest could not start the thread it reads Quint files on: {error}"),
        source: String::new(),
        line: 0,
        column: 0,
    }
}

/// Runs `work` on a thread with a stack of [`STACK`] bytes, or on this thread when it is one
/// already. A panic of `work` is passed on as it is.
pub(crate) fn on_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> Result<T, Problem> {
    if ON_LARGE_STACK.with(Cell::get) {
        return Ok(work());
    }
    thread::scope(|scope| {
        let worker = start(scope, Some(STACK), || {
            ON_LARGE_STACK.with(|large| large.set(true));
            work()
        })
        .map_err(|error| not_started(&error))?;
        worker
            .join()
            .map_err(|payload| panic::resume_unwind(payload))
    })
}

/// Runs `work` on a large stack as [`on_large_stack`] does, with `load` kept on this thread:
/// `work` is given a loader that asks this thread for each file and waits for its text.
pub(crate) fn reading_on_large_stack<T: Send>(
    mut load: impl FnMut(&str) -> Option<String>,
    work: impl FnOnce(&mut dyn FnMut(&str) -> Option<String>) -> T + Send,
) -> Result<T, Problem> {
    if ON_LARGE_STACK.with(Cell::get) {
        return Ok(work(&mut load));
    }
    let (asking, asked) = mpsc::channel::<String>();
    let (giving, given) = mpsc::channel::<Option<String>>();
    thread::scope(|scope| {
        let worker = start(scope, Some(STACK), move || {
            ON_LARGE_STACK.with(|large| large.set(true));
            let mut remote = |path: &str| {
                asking.send(path.to_owned()).ok()?;
                given.recv().ok().flatten()
            };
            work(&mut remote)
        })
        .map_err(|error| not_started(&error))?;
        for path in asked {
            if giving.send(load(&path)).is_err() {
                break;
            }
        }
        worker
            .join()
            .map_err(|payload| panic::resume_unwind(payload))
    })
}
