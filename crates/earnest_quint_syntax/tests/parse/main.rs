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

thread_local! {
    static ON_A_LARGE_STACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

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
