#![cfg(unix)]

use std::fs;

use crate::support::{Run, Sandbox, model, rule};

const WIDE: &str = "/// @check
module wide {
  var m: int
  action init = m' = 18446744073709551617
  action step = m' = m
  /// @check
  val below = m < 9223372036854775808
}
";

const VAL_AS_STEP: &str = "/// @check
module a {
  var x: int
  action init = x' = 0
  val step = x > 0
  /// @check
  val ok = x >= 0
  run oneTest = init.expect(x == 0)
}
";

const ILL_TYPED: &str = "/// @check
module b {
  var x: int
  action init = x' = 0
  action step = x' = x + true
  /// @check
  val ok = x >= 0
}
";

fn with_no_programs(sandbox: &Sandbox, arguments: &[&str]) -> Run {
    let empty = sandbox.path().join("no-programs");
    fs::create_dir_all(&empty).expect("an empty folder can be made");
    Run::of(
        sandbox
            .command()
            .env_remove("EARNEST_QUINT")
            .env("PATH", &empty)
            .args(arguments),
    )
}

fn problems(run: &Run) -> Vec<(String, u64, u64, String, String)> {
    run.json()["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|problem| {
            (
                problem["file"].as_str().unwrap_or_default().to_owned(),
                problem["line"].as_u64().unwrap_or_default(),
                problem["column"].as_u64().unwrap_or_default(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn an_integer_past_64_bits_in_a_qnt_file_keeps_every_digit_with_no_quint_and_no_node() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/wide.qnt", WIDE);
    let run = with_no_programs(&sandbox, &["verify", "specs/wide.qnt"]);
    run.exits(1);
    assert!(
        run.stdout.contains("broken  below")
            && run.stdout.contains("start  m = 18446744073709551617"),
        "`m < 2^63` is false for m = 2^64 + 1, read from the file with every digit\n{run}"
    );
    let json = with_no_programs(&sandbox, &["verify", "--json", "specs/wide.qnt"]);
    let report = json.json();
    assert_eq!(
        rule(model(&report, "wide"), "below")["verdict"],
        "broken",
        "{json}"
    );
    assert!(
        !sandbox.path().join(".earnest/quint").exists(),
        "nothing of Quint's is kept"
    );
}

#[test]
fn a_default_step_that_is_a_val_is_refused_with_the_argument_error_quint_prints() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/a.qnt", VAL_AS_STEP);
    let run = with_no_programs(&sandbox, &["verify", "--json", "specs/a.qnt"]);
    run.exits(1);
    assert_eq!(
        problems(&run),
        [(
            "specs/a.qnt".to_owned(),
            1,
            1,
            "`quint compile --main a`".to_owned(),
            "error: [QNT501] default --step action name 'step' is used by a val declaration"
                .to_owned()
        )],
        "Quint 0.33.1 prints this line, with no place\n{run}"
    );
}

#[test]
fn a_type_error_is_refused_at_its_place_with_the_first_line_quint_prints() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/b.qnt", ILL_TYPED);
    let run = with_no_programs(&sandbox, &["verify", "--json", "specs/b.qnt"]);
    run.exits(1);
    assert_eq!(
        problems(&run),
        [(
            "specs/b.qnt".to_owned(),
            5,
            22,
            "`quint compile --main b`".to_owned(),
            "Error [QNT000]: Couldn't unify int and bool".to_owned()
        )],
        "Quint 0.33.1 prints this line first, and the place under it\n{run}"
    );
}

#[test]
fn a_name_not_found_is_refused_by_the_parse_at_its_place() {
    let sandbox = Sandbox::empty();
    sandbox.write(
        "specs/b.qnt",
        &ILL_TYPED.replace("x' = x + true", "x' = y + 1"),
    );
    let run = with_no_programs(&sandbox, &["verify", "--json", "specs/b.qnt"]);
    run.exits(1);
    assert_eq!(
        problems(&run),
        [(
            "specs/b.qnt".to_owned(),
            5,
            22,
            "`quint parse`".to_owned(),
            "[QNT404] Name 'y' not found".to_owned()
        )],
        "as `quint parse` explains it\n{run}"
    );
}

#[test]
fn confirm_with_no_quint_says_the_check_is_not_compared_and_how_to_install_quint() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/b.qnt", &ILL_TYPED.replace("x + true", "x"));
    let run = with_no_programs(&sandbox, &["confirm", "--quint", "specs/b.qnt"]);
    assert!(
        run.stdout.contains("not compared")
            && run.stdout.contains(
                "`quint` was not found: install it with `npm install --save-dev \
                 @informalsystems/quint`, or set `EARNEST_QUINT`"
            ),
        "{run}"
    );
    let verified = with_no_programs(&sandbox, &["verify", "specs/b.qnt"]);
    verified.exits(0);
}
