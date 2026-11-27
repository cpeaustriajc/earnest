use earnest_quint_syntax::{Target, read};
use std::env;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

const SMALL: &str = "module small {\n  var x: int\n  action init = x' = 0\n  action step = x' = x + 1\n  val ok = x >= 0\n}\n";
const JOBS: usize = 8;
const CHILD: &str = "EARNEST_QUINT_SYNTAX_THREADS_CHILD";
const CHILD_JOBS: &str = "EARNEST_QUINT_SYNTAX_THREADS_JOBS";

fn jobs() -> usize {
    env::var(CHILD_JOBS)
        .ok()
        .and_then(|jobs| jobs.parse().ok())
        .unwrap_or(JOBS)
}

fn checked() -> Result<(), String> {
    let parsed = read("small.qnt", |path| {
        (path == "small.qnt").then(|| SMALL.to_owned())
    })
    .map_err(|problem| problem.to_string())?;
    parsed.typecheck().map_err(|problem| problem.to_string())?;
    let target = Target {
        main: "small".to_owned(),
        init: "init".to_owned(),
        step: "step".to_owned(),
        invariant: Some("ok".to_owned()),
        temporal: None,
    };
    parsed
        .compile(&target)
        .map(drop)
        .map_err(|problem| problem.to_string())
}

#[test]
fn eight_threads_read_typecheck_and_compile_at_once() {
    let outcomes: Vec<Result<(), String>> = thread::scope(|scope| {
        let workers: Vec<_> = (0..jobs()).map(|_| scope.spawn(checked)).collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap_or_else(|_| Err("panicked".to_owned())))
            .collect()
    });
    let refused: Vec<&String> = outcomes
        .iter()
        .filter_map(|outcome| outcome.as_ref().err())
        .collect();
    if env::var(CHILD).is_ok() {
        println!("refused {} of {}", refused.len(), jobs());
        for reason in &refused {
            println!("reason {reason}");
        }
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status
                .lines()
                .filter(|line| line.starts_with("VmHWM") || line.starts_with("VmPeak"))
            {
                println!("{line}");
            }
        }
        return;
    }
    assert!(refused.is_empty(), "{refused:?}");
}

fn refused_under(kib: u64) -> usize {
    let binary = env::current_exe().expect("the test binary");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -v {kib} && exec \"$0\" --exact threads::eight_threads_read_typecheck_and_compile_at_once --nocapture --test-threads 1"
        ))
        .arg(binary)
        .env(CHILD, "1")
        .output()
        .expect("the test runs under a limit");
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.split_once("refused ").map(|(_, rest)| rest))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse().ok())
        .unwrap_or(JOBS)
}

#[test]
#[cfg(target_os = "linux")]
fn eight_threads_at_once_fit_in_four_gibibytes_of_address_space() {
    assert_eq!(
        refused_under(4 * 1024 * 1024),
        0,
        "nothing is refused under 4 GiB"
    );
}

const LARGE: usize = 256 * 1024 * 1024;
const ALONE: &str = "EARNEST_QUINT_SYNTAX_THREADS_ALONE";

fn alone(test: &str) -> bool {
    if env::var(ALONE).is_ok() {
        return true;
    }
    let output = Command::new(env::current_exe().expect("the test binary"))
        .args(["--exact", test, "--nocapture", "--test-threads", "1"])
        .env(ALONE, "1")
        .output()
        .expect("the test runs alone");
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn threads_now() -> usize {
    std::fs::read_dir("/proc/self/task").map_or(0, Iterator::count)
}

fn longer() -> String {
    let items: Vec<String> = (0..300).map(|item| item.to_string()).collect();
    SMALL.replace(
        "  val ok",
        &format!("  val s = Set({})\n  val ok", items.join(", ")),
    )
}

fn most_threads_during<T>(work: impl FnOnce() -> T) -> (usize, T) {
    let done = AtomicBool::new(false);
    let most = AtomicUsize::new(0);
    let before = threads_now();
    let result = thread::scope(|scope| {
        let watcher = scope.spawn(|| {
            while !done.load(Ordering::SeqCst) {
                most.fetch_max(threads_now(), Ordering::SeqCst);
                thread::sleep(Duration::from_millis(1));
            }
        });
        thread::sleep(Duration::from_millis(20));
        let result = work();
        done.store(true, Ordering::SeqCst);
        watcher.join().expect("the watcher ends");
        result
    });
    (most.load(Ordering::SeqCst) - before - 1, result)
}

fn started_by_each_entry(bytes: usize) -> [usize; 3] {
    let text = longer();
    earnest_quint_syntax::on_a_stack_of(bytes, || {
        let (reading, parsed) = most_threads_during(|| {
            read("small.qnt", |path| {
                (path == "small.qnt").then(|| text.clone())
            })
            .expect("read")
        });
        let (checking, checked) = most_threads_during(|| parsed.typecheck().map(drop));
        assert!(checked.is_ok(), "{checked:?}");
        let target = Target {
            main: "small".to_owned(),
            init: "init".to_owned(),
            step: "step".to_owned(),
            invariant: Some("ok".to_owned()),
            temporal: None,
        };
        let (compiling, compiled) = most_threads_during(|| parsed.compile(&target).map(drop));
        assert!(compiled.is_ok(), "{compiled:?}");
        [reading, checking, compiling]
    })
}

fn on_a_large_thread<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    thread::scope(|scope| {
        earnest_threads::start(scope, Some(LARGE), work)
            .expect("a thread of 256 MiB starts")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    })
}

#[test]
fn on_a_stack_as_large_as_the_crate_needs_read_typecheck_and_compile_start_no_thread() {
    if !alone(
        "threads::on_a_stack_as_large_as_the_crate_needs_read_typecheck_and_compile_start_no_thread",
    ) {
        return;
    }
    on_a_large_thread(|| {
        assert_eq!(
            started_by_each_entry(LARGE),
            [0, 0, 0],
            "all three run inline"
        );
        assert_eq!(
            started_by_each_entry(earnest_quint_syntax::STACK_NEEDED - 1),
            [1, 1, 1],
            "a smaller stack gets the crate's own thread for each"
        );
        assert_eq!(
            started_by_each_entry(0),
            [1, 1, 1],
            "so does a stack the caller does not know"
        );
    });
}

#[test]
fn nested_statements_of_the_stack_end_where_they_began() {
    if !alone("threads::nested_statements_of_the_stack_end_where_they_began") {
        return;
    }
    on_a_large_thread(|| {
        earnest_quint_syntax::on_a_stack_of(LARGE, || {
            earnest_quint_syntax::on_a_stack_of(LARGE, || ());
            assert_eq!(
                started_by_each_entry(0),
                [0, 0, 0],
                "the outer statement still holds"
            );
        });
        assert_eq!(
            started_by_each_entry(0),
            [1, 1, 1],
            "and ends with the outer call"
        );
    });
}

#[test]
fn after_a_panic_inside_the_work_the_next_entry_starts_its_own_thread() {
    if !alone("threads::after_a_panic_inside_the_work_the_next_entry_starts_its_own_thread") {
        return;
    }
    on_a_large_thread(|| {
        let panicked = std::panic::catch_unwind(|| {
            earnest_quint_syntax::on_a_stack_of(LARGE, || panic!("the work panics"));
        });
        assert!(panicked.is_err(), "the panic is passed on");
        assert_eq!(
            started_by_each_entry(0),
            [1, 1, 1],
            "the statement ended with the panic"
        );
    });
}
