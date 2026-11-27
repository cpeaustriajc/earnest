//! Flattening `.qnt` files as `quint compile --target json` does, compared with what Quint
//! 0.33.1 wrote for each check of the repository, and read by earnest's reader both ways.

mod checks;
mod compared;
mod labels;
mod reader;
mod samples;

thread_local! {
    static ON_A_LARGE_STACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `test` again on a thread of `bytes` of stack, unless this thread is that one already:
/// true when it did, and the caller returns, false when the caller is on it and goes on. A test
/// of the whole repository reads, compares and drops what it reads as deep as a sample is
/// nested, and the samples of `tasks/quint/deep` are nested 500 deep; the default stack of a
/// test thread holds far less in a debug build.
fn ran_on_a_stack_of(bytes: usize, test: fn()) -> bool {
    if ON_A_LARGE_STACK.with(std::cell::Cell::get) {
        return false;
    }
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(bytes)
            .spawn_scoped(scope, || {
                ON_A_LARGE_STACK.with(|large| large.set(true));
                test();
            })
            .expect("the test's thread starts")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    });
    true
}
