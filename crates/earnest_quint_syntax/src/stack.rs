use crate::problem::Problem;
use earnest_threads::start;
use std::cell::Cell;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

/// The stack, in bytes, that reading and checking a file needs: what the deepest file the bounds
/// let through spends in a debug build, twice over. Every recursive pass refuses a file nested
/// deeper than its bound before it can spend this.
pub const STACK_NEEDED: usize = 64 * 1024 * 1024;

static THREADS_STARTED: AtomicUsize = AtomicUsize::new(0);

/// How many threads of its own the crate has started in this process. For the crate's tests.
#[doc(hidden)]
pub fn threads_started() -> usize {
    THREADS_STARTED.load(Ordering::Relaxed)
}

fn started() {
    THREADS_STARTED.fetch_add(1, Ordering::Relaxed);
}

thread_local! {
    static ON_LARGE_STACK: Cell<bool> = const { Cell::new(false) };
}

struct Restored(bool);

impl Drop for Restored {
    fn drop(&mut self) {
        ON_LARGE_STACK.with(|large| large.set(self.0));
    }
}

/// Runs `work` on this thread, which the caller states was started with `bytes` of stack. When
/// that is at least [`STACK_NEEDED`], every entry of the crate called inside `work` on this thread
/// runs on it, and no thread is started. When it is less, `work` runs as it would without this
/// call: each entry starts a thread of its own.
pub fn on_a_stack_of<T>(bytes: usize, work: impl FnOnce() -> T) -> T {
    if bytes < STACK_NEEDED {
        return work();
    }
    let _restored = Restored(ON_LARGE_STACK.with(|large| large.replace(true)));
    work()
}

fn not_started(error: &std::io::Error) -> Problem {
    Problem {
        code: "unreadable".to_owned(),
        message: format!(
            "earnest could not start the thread it reads Quint files on ({error}): run fewer jobs at once with --jobs, or raise the limit on address space (ulimit -v)"
        ),
        source: String::new(),
        line: 0,
        column: 0,
    }
}

/// Runs `work`, which recurses as deep as a `.qnt` file is nested but is not one of the crate's
/// entries (a caller's walk of the JSON the crate wrote, say), on a stack of at least
/// [`STACK_NEEDED`] bytes. [`on_a_stack_of`] only declares the stack of the caller's thread and
/// never starts one; this starts one when it must. The caller states, as for [`on_a_stack_of`],
/// that its thread was started with `bytes` of stack: when that is at least [`STACK_NEEDED`], or
/// when the call is inside [`on_a_stack_of`] with enough bytes or inside another such thread,
/// `work` runs inline and no thread is started, so a worker of 256 MiB starts none per file.
/// Otherwise `work` runs on a thread of [`STACK_NEEDED`] bytes started for it. A panic of `work`
/// is passed on as it is.
///
/// # Errors
///
/// When the system refuses the thread, the problem that says what to do (fewer `--jobs`, or a
/// higher `ulimit -v`); never a panic.
pub fn on_large_stack<T: Send>(
    bytes: usize,
    work: impl FnOnce() -> T + Send,
) -> Result<T, Problem> {
    on_a_stack_of(bytes, || on_stack_of(STACK_NEEDED, work))
}

/// [`on_large_stack`] for the crate's own entries, which run inline only inside a large stack.
pub(crate) fn on_own_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> Result<T, Problem> {
    on_stack_of(STACK_NEEDED, work)
}

fn on_stack_of<T: Send>(stack: usize, work: impl FnOnce() -> T + Send) -> Result<T, Problem> {
    if ON_LARGE_STACK.with(Cell::get) {
        return Ok(work());
    }
    thread::scope(|scope| {
        let worker = start(scope, Some(stack), || {
            ON_LARGE_STACK.with(|large| large.set(true));
            work()
        })
        .map_err(|error| not_started(&error))?;
        started();
        worker
            .join()
            .map_err(|payload| panic::resume_unwind(payload))
    })
}

/// Runs `work` on a large stack as [`on_own_large_stack`] does, with `load` kept on this thread:
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
        let worker = start(scope, Some(STACK_NEEDED), move || {
            ON_LARGE_STACK.with(|large| large.set(true));
            let mut remote = |path: &str| {
                asking.send(path.to_owned()).ok()?;
                given.recv().ok().flatten()
            };
            work(&mut remote)
        })
        .map_err(|error| not_started(&error))?;
        started();
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

/// What `work` builds, built on a large stack as [`on_own_large_stack`] runs it, or on this thread
/// when no thread can be started.
pub(crate) fn built_on_large_stack<T: Send>(work: impl Fn() -> T + Sync) -> T {
    on_own_large_stack(&work).unwrap_or_else(|_| work())
}

#[cfg(test)]
mod tests {
    use super::on_stack_of;

    #[test]
    fn a_thread_the_system_cannot_start_is_a_refusal_that_says_what_to_do() {
        let refused = on_stack_of(usize::MAX / 2, || 1).expect_err("no such stack is granted");
        assert_eq!(refused.code, "unreadable", "refused, not a panic");
        assert!(
            refused.message.ends_with(
                "run fewer jobs at once with --jobs, or raise the limit on address space (ulimit -v)"
            ),
            "{}",
            refused.message
        );
    }
}
