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

fn without_times(run: &Run) -> serde_json::Value {
    let mut report = run.json();
    for each in report["models"].as_array_mut().into_iter().flatten() {
        if let Some(fields) = each.as_object_mut() {
            fields.remove("milliseconds");
            fields.remove("cached");
        }
    }
    report
}

fn counter() -> String {
    let path = crate::support::repository().join("tasks/quint/checks/counter/counter.qnt");
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

#[test]
fn a_verdict_kept_from_a_run_before_lines_were_added_prints_what_a_fresh_run_prints() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/counter.qnt", &counter());
    with_no_programs(&sandbox, &["verify", "specs"]).exits(1);
    let moved = counter()
        .replacen("/// @check", "// above the mark\n/// @check", 1)
        .replacen("  val tooBig", "  // above the rule\n\n  val tooBig", 1);
    sandbox.write("specs/counter.qnt", &moved);
    let kept = with_no_programs(&sandbox, &["verify", "--json", "specs"]);
    kept.exits(1);
    assert_eq!(
        kept.json()["models"][0]["cached"],
        true,
        "the lines added change nothing the check reads, so its verdict is kept\n{kept}"
    );
    let fresh = with_no_programs(&sandbox, &["verify", "--json", "--no-cache", "specs"]);
    assert_eq!(
        without_times(&kept),
        without_times(&fresh),
        "{kept}\n{fresh}"
    );
    let text = with_no_programs(&sandbox, &["verify", "specs"]);
    let again = with_no_programs(&sandbox, &["verify", "--no-cache", "specs"]);
    assert_eq!(
        text.model_lines()
            .into_iter()
            .map(|mut line| {
                line.retain(|field| field != "(cached)" && !field.ends_with(" ms"));
                line
            })
            .collect::<Vec<_>>(),
        again
            .model_lines()
            .into_iter()
            .map(|mut line| {
                line.retain(|field| !field.ends_with(" ms"));
                line
            })
            .collect::<Vec<_>>(),
        "{text}\n{again}"
    );
}

#[test]
fn a_problem_of_an_unreadable_file_moves_with_the_lines_added_above_it() {
    let sandbox = Sandbox::empty();
    let path = crate::support::repository().join("tasks/quint/checks/choose/choose.qnt");
    let choose = fs::read_to_string(&path).expect("the sample can be read");
    sandbox.write("specs/choose.qnt", &choose);
    let first = with_no_programs(&sandbox, &["verify", "--json", "specs"]);
    assert_eq!(problems(&first)[0].1, 7, "{first}");
    sandbox.write("specs/choose.qnt", &format!("// moved\n{choose}"));
    let moved = with_no_programs(&sandbox, &["verify", "--json", "specs"]);
    assert_eq!(
        (problems(&moved)[0].1, problems(&moved)[0].2),
        (8, 22),
        "the construct is one line lower\n{moved}"
    );
}

#[test]
fn a_renamed_file_is_shown_by_its_new_name() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/counter.qnt", &counter());
    with_no_programs(&sandbox, &["verify", "specs"]).exits(1);
    fs::rename(
        sandbox.path().join("specs/counter.qnt"),
        sandbox.path().join("specs/renamed.qnt"),
    )
    .expect("the file can be renamed");
    let run = with_no_programs(&sandbox, &["verify", "--json", "specs"]);
    run.exits(1);
    assert_eq!(
        model(&run.json(), "counter")["file"],
        "specs/renamed.qnt",
        "{run}"
    );
}

const CHECKED: &[u8] = b"/// @check
module A {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  /// @check
  val ok = x <= 3
}
";

fn with_bytes(files: &[(&str, Vec<u8>)]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for (name, bytes) in files {
        let path = sandbox.path().join(name);
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder).expect("the folder can be made");
        }
        fs::write(&path, bytes).expect("the file can be written");
    }
    sandbox
}

fn replaced(text: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let at = text
        .windows(from.len())
        .position(|window| window == from)
        .expect("the text holds what is replaced");
    [&text[..at], to, &text[at + from.len()..]].concat()
}

fn passes(sandbox: &Sandbox) {
    let run = with_no_programs(sandbox, &["verify", "specs"]);
    run.exits(0);
    assert_eq!(
        run.model_line("A")[..3],
        ["pass", "A", "4 states"],
        "Quint 0.33.1 reads a byte that is not UTF-8 as U+FFFD, and so does earnest\n{run}"
    );
}

#[test]
fn a_byte_that_is_not_utf_8_in_a_comment_is_read_as_quint_reads_it() {
    passes(&with_bytes(&[(
        "specs/a.qnt",
        [b"// caf\xe9\n".as_slice(), CHECKED].concat(),
    )]));
}

#[test]
fn a_byte_that_is_not_utf_8_in_a_string_is_read_as_quint_reads_it() {
    passes(&with_bytes(&[(
        "specs/a.qnt",
        replaced(
            CHECKED,
            b"val ok = x <= 3",
            b"val ok = x <= 3 and \"\xff\" == \"\xff\"",
        ),
    )]));
}

#[test]
fn a_byte_that_is_not_utf_8_in_an_imported_file_is_read_as_quint_reads_it() {
    let importing = replaced(
        &replaced(
            &replaced(
                CHECKED,
                b"module A {\n",
                b"module A {\n  import lib.* from \"./lib\"\n",
            ),
            b"x < 3",
            b"x < Top",
        ),
        b"x <= 3",
        b"x <= Top",
    );
    passes(&with_bytes(&[
        ("specs/a.qnt", importing),
        (
            "specs/lib.qnt",
            b"// \xe9\nmodule lib {\n  pure val Top = 3\n}\n".to_vec(),
        ),
    ]));
}

#[test]
fn a_place_after_bytes_that_are_not_utf_8_is_quints_line_and_column() {
    for (written, column) in [
        (
            b"if (\"\xe9\xe2\x82\" == \"\xff\") x + true else 0".as_slice(),
            39,
        ),
        (
            b"if (\"\xc0\xaf\xe0\x80\xaf\" == \"\") x + true else 0".as_slice(),
            41,
        ),
    ] {
        let sandbox = with_bytes(&[(
            "specs/a.qnt",
            replaced(CHECKED, b"if (x < 3) x + 1 else 0", written),
        )]);
        let run = with_no_programs(&sandbox, &["verify", "--json", "specs"]);
        run.exits(1);
        let found = problems(&run);
        assert_eq!(
            (found[0].1, found[0].2, found[0].4.as_str()),
            (5, column, "Error [QNT000]: Couldn't unify int and bool"),
            "a truncated sequence is one U+FFFD and an overlong one a U+FFFD for each byte, \
             as Quint 0.33.1 reads them\n{run}"
        );
    }
}

#[test]
fn an_import_of_a_file_that_is_not_there_is_refused_as_quint_refuses_it() {
    let sandbox = with_bytes(&[(
        "specs/a.qnt",
        replaced(
            CHECKED,
            b"module A {\n",
            b"module A {\n  import lib.* from \"./nothere\"\n",
        ),
    )]);
    let run = with_no_programs(&sandbox, &["verify", "--json", "specs"]);
    run.exits(1);
    assert_eq!(
        problems(&run),
        [(
            "specs/a.qnt".to_owned(),
            3,
            3,
            "`quint parse`".to_owned(),
            "[QNT013] import ... from './nothere': could not load".to_owned()
        )],
        "as Quint 0.33.1 prints it, with the file as shown\n{run}"
    );
}
