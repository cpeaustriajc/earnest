use earnest_quint_syntax::{Target, read};
use std::env;
use std::process::Command;
use std::thread;

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
        .find_map(|line| line.strip_prefix("refused "))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse().ok())
        .unwrap_or(JOBS)
}

#[test]
#[cfg(target_os = "linux")]
fn eight_threads_at_once_fit_in_two_gibibytes_of_address_space() {
    assert_eq!(
        refused_under(2 * 1024 * 1024),
        0,
        "nothing is refused under 2 GiB"
    );
}
