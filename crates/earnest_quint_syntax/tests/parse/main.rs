//! Reading `.qnt` files and comparing what is read with what Quint 0.33.1 recorded for them.

mod compare;
mod corpus;
mod deep;
mod depth;
mod docs;
mod effects;
mod names;
mod places;
mod readings;
mod refusals;
mod review;
mod threads;
mod typecheck;
mod types;
mod typing_review;

/// `work` on a thread of the crate's own stack. A corpus test walks what it reads and what Quint
/// recorded as deep as a sample is nested, and the samples of `tasks/quint/deep` are nested 500
/// deep; the default stack of a test thread holds far less in a debug build.
fn on_a_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(earnest_quint_syntax::STACK_NEEDED)
            .spawn_scoped(scope, work)
            .expect("the test's thread starts")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}
