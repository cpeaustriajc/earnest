//! The one place in the workspace that starts threads, so that every thread is joined.
//!
//! A standard handle that is dropped detaches its thread, and glibc's `pthread_detach` reads the
//! thread's own block, at the top of its stack, after it marks the thread as detached. A thread
//! that exits in between frees that stack; one larger than glibc's cache of stacks (40 MiB) is
//! unmapped at once, and the read crashes the process. So the workspace starts threads only
//! through [`start`], and its handle joins the thread whatever the caller does with it.
//! `just lint-library` refuses the standard calls that start a thread everywhere else.

use std::any::Any;
use std::io;
use std::panic;
use std::thread::{self, Scope, ScopedJoinHandle};

/// A thread started by [`start`]: join it with [`Joined::join`] to get what it returned or the
/// payload of its panic.
///
/// Dropping it also waits for the thread to end, and passes its panic on to the thread that drops
/// it, unless that thread is unwinding already.
#[derive(Debug)]
pub struct Joined<'scope, T> {
    handle: Option<ScopedJoinHandle<'scope, T>>,
}

impl<T> Joined<'_, T> {
    /// Waits for the thread to end and returns what it returned, or the payload of its panic.
    ///
    /// # Errors
    ///
    /// The payload of the thread's panic, as [`std::thread::JoinHandle::join`] gives it.
    pub fn join(mut self) -> thread::Result<T> {
        self.handle
            .take()
            .map_or_else(|| Err(joined_already()), ScopedJoinHandle::join)
    }

    /// Whether the thread has ended, so that [`Joined::join`] would not wait.
    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .is_none_or(ScopedJoinHandle::is_finished)
    }
}

fn joined_already() -> Box<dyn Any + Send> {
    Box::new("the thread was joined already")
}

impl<T> Drop for Joined<'_, T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take()
            && let Err(panic) = handle.join()
            && !thread::panicking()
        {
            panic::resume_unwind(panic);
        }
    }
}

/// Starts `work` on a new thread of `scope`, with a stack of `stack` bytes, or of the standard
/// size where `stack` is `None`.
///
/// # Errors
///
/// When the system cannot start a thread, as [`std::thread::Builder::spawn_scoped`] reports it.
pub fn start<'scope, T, F>(
    scope: &'scope Scope<'scope, '_>,
    stack: Option<usize>,
    work: F,
) -> io::Result<Joined<'scope, T>>
where
    F: FnOnce() -> T + Send + 'scope,
    T: Send + 'scope,
{
    let builder = stack.map_or_else(thread::Builder::new, |bytes| {
        thread::Builder::new().stack_size(bytes)
    });
    builder.spawn_scoped(scope, work).map(|handle| Joined {
        handle: Some(handle),
    })
}

#[cfg(test)]
mod tests {
    use super::start;
    use std::fs;
    use std::panic::{self, AssertUnwindSafe};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

    const PAUSE: Duration = Duration::from_millis(20);
    const LARGE_STACK: usize = 256 * 1024 * 1024;

    #[test]
    fn join_gives_what_the_thread_returned() {
        let answer = thread::scope(|scope| {
            start(scope, None, || 6 * 7)
                .expect("a thread starts")
                .join()
                .expect("the thread does not panic")
        });
        assert_eq!(answer, 42, "the thread's result");
    }

    #[test]
    fn join_gives_the_payload_of_a_panic() {
        let payload = thread::scope(|scope| {
            start(scope, None, || panic::panic_any(7_u8))
                .expect("a thread starts")
                .join()
                .expect_err("the thread panics")
        });
        assert_eq!(payload.downcast_ref::<u8>(), Some(&7), "its payload");
    }

    #[test]
    fn dropping_the_handle_waits_for_the_thread() {
        let ended = AtomicBool::new(false);
        thread::scope(|scope| {
            let running = start(scope, Some(LARGE_STACK), || {
                thread::sleep(PAUSE);
                ended.store(true, Ordering::SeqCst);
            })
            .expect("a thread starts");
            drop(running);
            assert!(
                ended.load(Ordering::SeqCst),
                "the thread ended before the drop returned"
            );
        });
    }

    #[test]
    fn dropping_the_handle_passes_the_panic_on() {
        let caught = panic::catch_unwind(AssertUnwindSafe(|| {
            thread::scope(|scope| {
                drop(start(scope, None, || panic::panic_any(9_u8)).expect("a thread starts"));
            });
        }))
        .expect_err("the panic reaches the thread that dropped the handle");
        assert_eq!(
            caught.downcast_ref::<u8>(),
            Some(&9),
            "the thread's payload"
        );
    }

    #[test]
    fn a_thread_reports_when_it_has_finished() {
        thread::scope(|scope| {
            let running = start(scope, None, || thread::sleep(PAUSE)).expect("a thread starts");
            while !running.is_finished() {
                thread::sleep(PAUSE);
            }
            assert!(running.join().is_ok(), "a finished thread joins at once");
        });
    }

    #[test]
    fn the_settings_for_library_code_repeat_the_workspace_settings() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |file: &str| {
            fs::read_to_string(root.join(file)).unwrap_or_else(|_| panic!("{file} exists"))
        };
        assert!(
            read("lints/threads/clippy.toml").starts_with(&read("clippy.toml")),
            "lints/threads/clippy.toml starts with every line of clippy.toml"
        );
    }
}
