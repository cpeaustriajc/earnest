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
