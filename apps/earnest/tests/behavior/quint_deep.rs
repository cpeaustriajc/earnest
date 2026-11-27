#![cfg(unix)]

//! `.qnt` files nested deep, read through the CLI: the samples of `tasks/quint/deep`, written by
//! its `generate.py`, each a check whose `holds` holds and whose `broken` is broken at `x = 3`,
//! as `quint run` and TLC judge them.

use std::fs;

use crate::support::{Run, Sandbox, model, repository, rule};

/// The shapes that read in seconds in a debug build at 300 deep. The others of the folder
/// (`sets`, `records`, `lambdas`) spend minutes there in the front end's type inference, so they
/// are judged in release by the measurements of `tasks/quint/README.md`, not here.
const SHAPES: [&str; 9] = [
    "nots", "parens", "sums", "lets", "ands", "ors", "elseifs", "ifs", "implies",
];

const DEPTHS: [usize; 2] = [100, 300];

fn sample(name: &str) -> String {
    let path = repository()
        .join("tasks/quint/deep")
        .join(format!("{name}.qnt"));
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn names() -> Vec<String> {
    DEPTHS
        .iter()
        .flat_map(|depth| SHAPES.iter().map(move |shape| format!("{shape}_{depth}")))
        .collect()
}

fn with_samples() -> Sandbox {
    let sandbox = Sandbox::empty();
    for name in names() {
        sandbox.write(&format!("deep/{name}.qnt"), &sample(&name));
    }
    sandbox
}

fn judged_as_quint_judges(run: &Run, name: &str) {
    let report = run.json();
    let checked = model(&report, name);
    assert_eq!(checked["status"], "fail", "{name}\n{run}");
    assert_eq!(rule(checked, "holds")["verdict"], "holds", "{name}\n{run}");
    assert_eq!(
        rule(checked, "broken")["verdict"],
        "broken",
        "{name}\n{run}"
    );
    assert_eq!(
        checked["states"], 4,
        "x goes 0, 1, 2, 3 and back: {name}\n{run}"
    );
}

#[test]
fn a_folder_of_files_nested_100_and_300_deep_is_judged_as_quint_judges_it_on_one_job_and_four() {
    let sandbox = with_samples();
    for jobs in ["1", "4"] {
        let run = sandbox.earnest(&["verify", "--json", "--no-cache", "--jobs", jobs, "deep"]);
        run.exits(1);
        assert!(
            run.json()["unreadable"]
                .as_array()
                .is_none_or(Vec::is_empty),
            "{run}"
        );
        for name in names() {
            judged_as_quint_judges(&run, &name);
        }
    }
}

#[test]
fn a_file_nested_300_deep_named_alone_is_judged_as_quint_judges_it() {
    let sandbox = Sandbox::empty();
    sandbox.write("nots_300.qnt", &sample("nots_300"));
    let run = sandbox.earnest(&["verify", "--json", "--no-cache", "nots_300.qnt"]);
    run.exits(1);
    judged_as_quint_judges(&run, "nots_300");
}

fn git(sandbox: &Sandbox, arguments: &[&str]) {
    let done = std::process::Command::new("git")
        .args(arguments)
        .current_dir(sandbox.path())
        .env("GIT_AUTHOR_NAME", "earnest")
        .env("GIT_AUTHOR_EMAIL", "earnest@example.com")
        .env("GIT_COMMITTER_NAME", "earnest")
        .env("GIT_COMMITTER_EMAIL", "earnest@example.com")
        .output()
        .expect("git runs");
    assert!(done.status.success(), "git {arguments:?}: {done:?}");
}

fn shallow(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with("  val holds = ") {
                "  val holds = x <= 3"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_rule_that_nests_300_deep_since_the_base_is_listed_changed() {
    for name in ["nots_300", "sums_300", "ifs_300"] {
        let deep = sample(name);
        let sandbox = Sandbox::empty();
        sandbox.write("m.qnt", &shallow(&deep));
        git(&sandbox, &["init", "-q", "."]);
        git(&sandbox, &["add", "m.qnt"]);
        git(&sandbox, &["commit", "-qm", "base"]);
        sandbox.write("m.qnt", &deep);
        let run = sandbox.earnest(&["verify", "--no-cache", "--base", "HEAD", "m.qnt"]);
        run.exits(1);
        assert!(
            run.stdout
                .lines()
                .any(|line| line.split_whitespace().collect::<Vec<_>>()
                    == ["changed", &format!("{name}.holds"), "m.qnt"]),
            "{name}\n{run}"
        );
    }
}

fn nested_nots(depth: usize) -> String {
    let inner = if depth.is_multiple_of(2) {
        "x <= 3"
    } else {
        "x > 3"
    };
    format!(
        "/// @check\nmodule deep {{\n  var x: int\n  action init = x' = 0\n  action step = x' = x\n  /// @check\n  val holds = {}{inner}{}\n}}\n",
        "not(".repeat(depth),
        ")".repeat(depth)
    )
}

fn a_sum_of(terms: usize) -> String {
    format!(
        "/// @check\nmodule deep {{\n  var x: int\n  action init = x' = 0\n  action step = x' = x\n  /// @check\n  val holds = {}x <= 3\n}}\n",
        "0 + ".repeat(terms)
    )
}

fn problems(run: &Run) -> Vec<(String, u64, String)> {
    run.json()["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|problem| {
            (
                problem["file"].as_str().unwrap_or_default().to_owned(),
                problem["line"].as_u64().unwrap_or_default(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_file_nested_past_the_front_ends_bounds_is_refused_with_the_nesting_and_the_bound_at_its_place()
{
    let sandbox = Sandbox::empty();
    sandbox.write("nested.qnt", &nested_nots(1_999));
    sandbox.write("long.qnt", &a_sum_of(3_998));
    let run = sandbox.earnest(&["verify", "--json", "--no-cache", "nested.qnt", "long.qnt"]);
    run.exits(1);
    let found = problems(&run);
    assert_eq!(
        found,
        [
            (
                "long.qnt".to_owned(),
                7,
                "expressions or types here are nested more than 4000 deep in \
                 Quint's tree, deeper than earnest reads"
                    .to_owned()
            ),
            (
                "nested.qnt".to_owned(),
                7,
                "expressions or types here are nested more than 2000 deep, deeper \
                 than earnest reads"
                    .to_owned()
            ),
        ],
        "{run}"
    );
    assert!(!run.stdout.contains("can't be parsed"), "{run}");
}

#[test]
fn a_file_nested_just_inside_the_front_ends_bounds_is_judged() {
    let sandbox = Sandbox::empty();
    sandbox.write("nested.qnt", &nested_nots(1_000));
    let run = sandbox.earnest(&["verify", "--json", "--no-cache", "nested.qnt"]);
    run.exits(0);
    assert_eq!(
        rule(model(&run.json(), "deep"), "holds")["verdict"],
        "holds",
        "{run}"
    );
}

fn compiled_nested(depth: usize) -> String {
    format!(
        "{{\"stage\":\"compiling\",\"main\":\"deep\",\"warnings\":[],\"modules\":[{{\"id\":1,\"name\":\"deep\",\"declarations\":[{{\"id\":2,\"kind\":\"def\",\"name\":\"holds\",\"qualifier\":\"val\",\"expr\":{}{{\"id\":4,\"kind\":\"bool\",\"value\":true}}{}}}]}}],\"table\":{{}}}}",
        "{\"id\":3,\"kind\":\"app\",\"opcode\":\"not\",\"args\":[".repeat(depth),
        "]}".repeat(depth)
    )
}

#[test]
fn a_compiled_module_nested_5000_deep_is_refused_with_the_nesting_and_the_bound_not_a_crash() {
    let sandbox = Sandbox::empty();
    sandbox.write("compiled.json", &compiled_nested(5_000));
    let run = sandbox.earnest(&["verify", "--json", "compiled.json"]);
    run.exits(1);
    let found = problems(&run);
    assert_eq!(found.len(), 1, "{run}");
    let (file, line, message) = &found[0];
    assert_eq!((file.as_str(), *line), ("compiled.json", 1), "{run}");
    assert_eq!(
        message,
        "the JSON is nested more than 8008 deep here, deeper than earnest reads: a compiled \
         module's expressions may nest up to 4000 deep",
        "{run}"
    );
    assert!(!run.stdout.contains("recursion limit"), "{run}");
}

/// A library for `LD_PRELOAD` that refuses only a thread asked for with 512 MiB of stack, the
/// command's thread of `main.rs`, so that the command runs on the process's main thread at the
/// system's stack, as it does where the system grants no such thread. It writes the file
/// `$REFUSED_MARK` when it refuses one, so a test knows the command ran there.
#[cfg(target_os = "linux")]
const ONLY_THE_SYSTEMS_STACK: &str = r#"
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
int pthread_create(pthread_t *thread, const pthread_attr_t *attributes, void *(*work)(void *), void *argument) {
  int (*real)(pthread_t *, const pthread_attr_t *, void *(*)(void *), void *) = dlsym(RTLD_NEXT, "pthread_create");
  size_t size = 0;
  if (attributes && pthread_attr_getstacksize(attributes, &size) == 0 && size == (size_t)512 * 1024 * 1024) {
    const char *mark = getenv("REFUSED_MARK");
    FILE *file = mark ? fopen(mark, "w") : NULL;
    if (file) fclose(file);
    return EAGAIN;
  }
  return real(thread, attributes, work, argument);
}
"#;

/// `earnest` with `arguments`, its command on the process's main thread with 1 MiB of stack,
/// the main thread's stack on Windows and less than any Linux or macOS gives. Linux only: the
/// library is built with `cc` and loaded with `LD_PRELOAD`.
#[cfg(target_os = "linux")]
fn on_the_systems_stack(sandbox: &Sandbox, arguments: &[&str]) -> Run {
    let source = sandbox.path().join("refuse.c");
    let library = sandbox.path().join("refuse.so");
    let mark = sandbox.path().join("refused");
    fs::write(&source, ONLY_THE_SYSTEMS_STACK).expect("the library's source is written");
    let built = std::process::Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&library)
        .arg(&source)
        .arg("-ldl")
        .output()
        .expect("cc runs: the test builds a library with it");
    assert!(built.status.success(), "cc: {built:?}");
    let _ = fs::remove_file(&mark);
    let mut command = assert_cmd::Command::new("sh");
    command
        .current_dir(sandbox.path())
        .env("LD_PRELOAD", &library)
        .env("REFUSED_MARK", &mark)
        .args(["-c", "ulimit -s 1024 && exec \"$0\" \"$@\""])
        .arg(env!("CARGO_BIN_EXE_earnest"))
        .args(arguments);
    let run = Run::of(&mut command);
    assert!(
        mark.exists(),
        "the 512 MiB thread was asked for and refused, so the command ran on the main thread\n{run}"
    );
    run
}

#[cfg(target_os = "linux")]
fn compiled_with_a_step_nested(depth: usize) -> String {
    let path =
        repository().join("tasks/break_it/models/quint_second/confirm/broken_eventually.json");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let value = r#"{"id":7,"kind":"bool","value":true}"#;
    assert!(text.contains(value), "the step assigns true");
    let nested = format!(
        "{}{value}{}",
        r#"{"id":7,"kind":"app","opcode":"not","args":["#.repeat(depth),
        "]}".repeat(depth)
    );
    text.replacen(value, &nested, 1)
}

#[cfg(target_os = "linux")]
#[test]
fn on_the_systems_stack_a_compiled_module_at_the_bound_is_judged_and_one_past_it_refused() {
    let sandbox = Sandbox::empty();
    sandbox.write("at.json", &compiled_with_a_step_nested(3_990));
    sandbox.write("past.json", &compiled_with_a_step_nested(4_010));
    let run = on_the_systems_stack(&sandbox, &["verify", "--json", "at.json", "past.json"]);
    run.exits(1);
    let found = problems(&run);
    assert_eq!(found.len(), 1, "{run}");
    assert_eq!((found[0].0.as_str(), found[0].1), ("past.json", 1), "{run}");
    assert!(
        found[0]
            .2
            .starts_with("the JSON is nested more than 8008 deep here"),
        "{run}"
    );
    let report = run.json();
    assert!(
        model(&report, "broken_eventually")["states"]
            .as_u64()
            .is_some(),
        "{run}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn on_the_systems_stack_a_file_nested_1000_deep_is_judged() {
    let sandbox = Sandbox::empty();
    sandbox.write("nested.qnt", &nested_nots(1_000));
    let run = on_the_systems_stack(&sandbox, &["verify", "--json", "--no-cache", "nested.qnt"]);
    run.exits(0);
    assert_eq!(
        rule(model(&run.json(), "deep"), "holds")["verdict"],
        "holds",
        "{run}"
    );
}

/// A stand-in for Quint 0.33.1 that knows one answer: `quint test` plays `divTest` and it passes,
/// as `quint test` truncates `(-7) / 2` to -3.
const QUINT_TEST_TRUNCATES: &str = "#!/bin/sh
case \"$1\" in
  --version) echo \"0.33.1\" ;;
  test) printf '\\n  deep\\n    ok divTest passed 1 test(s)\\n\\n  1 passing (5ms)\\n' ;;
  *) echo \"the stand-in has no answer for $*\" >&2; exit 2 ;;
esac
";

#[test]
fn a_run_nested_80_deep_that_divides_still_gets_its_cause_from_confirm_with_quint() {
    use std::os::unix::fs::PermissionsExt;
    let nots = 80;
    let sandbox = Sandbox::empty();
    sandbox.write(
        "specs/deep.qnt",
        &format!(
            "/// @check\nmodule deep {{\n  var x: int\n  action init = x' = -7\n  action step = x' = x\n  run divTest = init.then(step).expect({}x / 2 == -3{})\n}}\n",
            "not(".repeat(nots),
            ")".repeat(nots)
        ),
    );
    for (program, text) in [
        ("fake/quint", QUINT_TEST_TRUNCATES),
        ("java/java", "#!/bin/sh\nexit 0\n"),
    ] {
        sandbox.write(program, text);
        fs::set_permissions(
            sandbox.path().join(program),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the stand-in can be made a program");
    }
    let run = Run::of(
        sandbox
            .command()
            .env("EARNEST_QUINT", sandbox.path().join("fake/quint"))
            .env("PATH", sandbox.path().join("java"))
            .args(["confirm", "--quint", "specs"]),
    );
    let lines: Vec<&str> = run.stdout.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("  run divTest:"))
        .unwrap_or_else(|| panic!("divTest is compared\n{run}"));
    assert_eq!(
        lines.get(at + 1).copied(),
        Some(
            "    Quint can't confirm this: the check's runs use `/` or `%`, which `quint test` \
             truncates toward zero, where the checker and Quint's TLC run take the floor of the \
             quotient: with a negative number they answer otherwise, as `(-7) / 2` is -3 in \
             `quint test` and -4 in the checker"
        ),
        "the division is found in the compiled JSON, nested past serde's limit\n{run}"
    );
}
