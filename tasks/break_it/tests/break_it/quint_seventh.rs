#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

use tempfile::TempDir;

use crate::support::{Report, Sandbox, binary, rule};

const RECORDED_TEST: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
case "$1" in
  --version) echo "0.33.1" ;;
  test) cat "$here/quint_test.out"; exit 1 ;;
  *) echo "the recording has no answer for $1" >&2; exit 2 ;;
esac
"#;

fn verified(file: &str, text: &str, arguments: &[&str]) -> Report {
    Sandbox::with(&[(file, text)]).verify(arguments)
}

fn verdicts<'a>(report: &'a Report, model: &str, ids: &[&str]) -> Vec<(String, &'a str)> {
    let found = report.model(model);
    ids.iter()
        .map(|id| {
            (
                (*id).to_owned(),
                rule(found, id)["verdict"].as_str().unwrap_or("missing"),
            )
        })
        .collect()
}

#[test]
fn a_step_that_picks_from_100_values_or_more_is_not_retried_so_a_false_pick_breaks_the_run() {
    let report = verified(
        "big_step_pick.qnt",
        include_str!("../../models/quint_seventh/big_step_pick/big_step_pick.qnt"),
        &[],
    );
    assert_eq!(
        verdicts(
            &report,
            "bigStepPick",
            &[
                "midPickTest",
                "lastPickTest",
                "firstPickTest",
                "smallPickTest"
            ]
        ),
        vec![
            ("midPickTest".to_owned(), "broken"),
            ("lastPickTest".to_owned(), "broken"),
            ("firstPickTest".to_owned(), "broken"),
            ("smallPickTest".to_owned(), "holds"),
        ],
        "`quint test --backend typescript --main bigStepPick` of Quint 0.33.1 prints \
         `midPickTest failed after 1 test(s)` and `firstPickTest failed after 1 test(s)`, each \
         with `Error [QNT513]: Cannot continue in `then` because the highlighted expression \
         evaluated to false` at the step that picks, `lastPickTest failed after 1 test(s)` with \
         `Error [QNT511]: Test lastPickTest returned false`, and `ok smallPickTest passed 10000 \
         test(s)`: Quint retries a `nondet` only over fewer than 100 values, in a step as at \
         the top of a run, so a step over `0.to(199)` keeps the value it picked and is false \
         for 199 of them\n{report}"
    );
}

#[test]
fn a_run_s_own_nondet_over_a_range_past_64_bits_is_played_not_called_a_set_read_from_the_state() {
    let report = verified(
        "big_top_range.qnt",
        include_str!("../../models/quint_seventh/big_top_range/big_top_range.qnt"),
        &[],
    );
    assert_eq!(
        (
            report.is_unreadable(),
            verdicts(
                &report,
                "bigTopRange",
                &["pastTest", "writtenTest", "belowTest", "inStepTest"]
            )
        ),
        (
            false,
            vec![
                ("pastTest".to_owned(), "holds"),
                ("writtenTest".to_owned(), "holds"),
                ("belowTest".to_owned(), "holds"),
                ("inStepTest".to_owned(), "holds"),
            ]
        ),
        "`quint test --backend typescript` of Quint 0.33.1 prints `ok pastTest passed 10000 \
         test(s)`, and the same for `writtenTest`, `belowTest` and `inStepTest`; the range \
         `(2 ^ 63 - 1).to(2 ^ 63 + 1)` reads no state, its ends are numbers, and the docs say \
         the ends of a range keep every digit. Earnest refuses `pastTest` and `writtenTest` \
         with ``a run's own `nondet` over a set read from the state``, though the same range \
         is read inside a step\n{report}"
    );
}

#[test]
fn confirm_names_the_division_cause_on_the_run_that_divides_and_on_no_other() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let program = folder.path().join("quint");
    fs::write(&program, RECORDED_TEST).expect("the recording program can be written");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("the recording program can be made a program");
    fs::write(
        folder.path().join("quint_test.out"),
        include_str!("../../models/quint_seventh/division_blamed/quint_test.out"),
    )
    .expect("the recorded output can be written");
    let checks = folder.path().join("checks");
    fs::create_dir_all(&checks).expect("the checks folder can be made");
    fs::write(
        checks.join("division_blamed.qnt"),
        include_str!("../../models/quint_seventh/division_blamed/division_blamed.qnt"),
    )
    .expect("the model can be written");
    let output = Command::new(binary())
        .args(["confirm", "--quint", "division_blamed.qnt"])
        .env("EARNEST_QUINT", &program)
        .current_dir(&checks)
        .output()
        .expect("earnest starts");
    let printed = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = printed.lines().collect();
    let half = lines
        .iter()
        .position(|line| line.contains("run halfTest"))
        .and_then(|at| lines.get(at.saturating_add(1)));
    assert!(
        half.is_some_and(|cause| cause.contains("truncates toward zero")),
        "`halfTest` is `(-7) / 2 == -4`: the checker floors and says holds, `quint test` \
         truncates and fails it, and the line under it names that cause\n{printed}"
    );
    assert!(
        !printed.contains("pickTest"),
        "`pickTest` reaches no `/` and no `%`, and since #99 the checker breaks it as `quint \
         test` does (`pickTest failed after 1 test(s)`, `Error [QNT513]: Cannot continue in \
         `then``), so it is not listed. Before #99 it differed, and the division cause of \
         `halfTest` was given to it too, which turned a real `differs` into `cannot`: a cause \
         belongs to the run that reaches it (`confirm_reach.rs` has the tests for that)\n{printed}"
    );
}

#[test]
fn a_false_assert_that_is_the_whole_run_after_a_nondet_is_an_error_not_a_skipped_value() {
    let report = verified(
        "assert_after_nondet.qnt",
        include_str!("../../models/quint_seventh/assert_after_nondet/assert_after_nondet.qnt"),
        &[],
    );
    assert_eq!(
        verdicts(
            &report,
            "assertAfterNondet",
            &["wholeTest", "andTest", "valTest", "stepTest"]
        ),
        vec![
            ("wholeTest".to_owned(), "broken"),
            ("andTest".to_owned(), "broken"),
            ("valTest".to_owned(), "broken"),
            ("stepTest".to_owned(), "broken"),
        ],
        "`quint test --backend typescript` of Quint 0.33.1 fails all four runs within a few \
         samples, each with `Error [QNT508]: Assertion failed`: a false `assert` is an error, \
         which stops the run at once, and is never a value to skip. The docs say so too (\"an \
         error, which breaks the run …, on a false `assert`\"). Earnest breaks `stepTest`, \
         where the `assert` sits in a step, and passes the three where it is the run itself\n\
         {report}"
    );
}
