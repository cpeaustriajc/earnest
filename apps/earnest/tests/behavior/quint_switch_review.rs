#![cfg(unix)]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;

use crate::support::{Run, Sandbox};

const COUNTER: &str = "/// @check
module A {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else 0
  /// @check
  val ok = x <= 3
  /// @check broken
  val small = x < 2
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

fn write_bytes(sandbox: &Sandbox, relative: &str, bytes: &[u8]) {
    let path = sandbox.path().join(relative);
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).expect("the folder can be made");
    }
    fs::write(&path, bytes).expect("the file can be written");
}

#[test]
fn a_byte_that_is_not_utf8_in_a_comment_is_read_as_quint_reads_it_and_the_check_passes() {
    let sandbox = Sandbox::empty();
    let mut bytes = b"// caf\xe9\n".to_vec();
    bytes.extend_from_slice(COUNTER.as_bytes());
    write_bytes(&sandbox, "specs/a.qnt", &bytes);
    let run = with_no_programs(&sandbox, &["verify", "--no-cache", "specs/a.qnt"]);
    assert!(
        run.stdout.starts_with("pass        A  4 states")
            && run.stdout.contains("2 rules, 0 runs")
            && run.stdout.contains("1 model: 1 pass, 0 fail, 0 unfinished"),
        "the build before the switch, with Quint 0.33.1, printed:\n\
         pass        A  4 states  0 ms  2 rules, 0 runs\n  \
         broken  small  (expected)\n\
         1 model: 1 pass, 0 fail, 0 unfinished\n\
         and exited 0\n{run}"
    );
    run.exits(0);
}

#[test]
fn a_string_with_a_byte_that_is_not_utf8_is_read_as_quint_reads_it_and_the_check_passes() {
    let sandbox = Sandbox::empty();
    let text = COUNTER.replace(
        "val ok = x <= 3",
        "val ok = x <= 3 and \"\u{1}\" == \"\u{1}\"",
    );
    let bytes: Vec<u8> = text
        .bytes()
        .map(|byte| if byte == 1 { 0xff } else { byte })
        .collect();
    write_bytes(&sandbox, "specs/a.qnt", &bytes);
    let run = with_no_programs(&sandbox, &["verify", "--no-cache", "specs/a.qnt"]);
    assert!(
        run.stdout.starts_with("pass        A  4 states")
            && run.stdout.contains("1 model: 1 pass, 0 fail, 0 unfinished"),
        "the build before the switch, with Quint 0.33.1, printed:\n\
         pass        A  4 states  0 ms  2 rules, 0 runs\n  \
         broken  small  (expected)\n\
         1 model: 1 pass, 0 fail, 0 unfinished\n{run}"
    );
    run.exits(0);
}

#[test]
fn a_qnt_file_whose_name_is_not_utf8_is_refused_with_the_message_and_place_of_the_build_before_the_switch()
 {
    let sandbox = Sandbox::empty();
    let folder = sandbox.path().join("specs");
    fs::create_dir_all(&folder).expect("the folder can be made");
    fs::write(
        folder.join(OsStr::from_bytes(b"caf\xe9.qnt")),
        COUNTER.as_bytes(),
    )
    .expect("the file can be written");
    let run = with_no_programs(&sandbox, &["verify", "--no-cache", "--json", "specs"]);
    run.exits(1);
    let report = run.json();
    let problem = &report["unreadable"][0];
    assert_eq!(
        (
            problem["file"].as_str(),
            problem["line"].as_u64(),
            problem["column"].as_u64(),
            problem["feature"].as_str(),
            problem["message"].as_str(),
        ),
        (
            Some("specs/caf\u{fffd}.qnt"),
            Some(1),
            Some(1),
            Some("`quint parse`"),
            Some("the Quint file can't be parsed"),
        ),
        "the build before the switch, with Quint 0.33.1, printed:\n\
         unreadable  specs/caf\u{fffd}.qnt\n  \
         specs/caf\u{fffd}.qnt:1:1  `quint parse`  the Quint file can't be parsed\n{run}"
    );
}

fn first_problem(run: &Run) -> (u64, u64, String) {
    let report = run.json();
    let problem = &report["unreadable"][0];
    (
        problem["line"].as_u64().unwrap_or_default(),
        problem["column"].as_u64().unwrap_or_default(),
        problem["message"]
            .as_str()
            .unwrap_or_default()
            .split(' ')
            .next()
            .unwrap_or_default()
            .to_owned(),
    )
}

#[test]
fn a_syntax_error_in_the_body_after_a_nondet_is_placed_where_quint_places_it() {
    let sandbox = Sandbox::empty();
    sandbox.write(
        "specs/a.qnt",
        "/// @check
module A {
  var x: int
  action init = x' = 0
  action step = { nondet v = 0.to(2).oneOf() x' = v + }
  /// @check
  val ok = x >= 0
}
",
    );
    let run = with_no_programs(&sandbox, &["verify", "--no-cache", "--json", "specs/a.qnt"]);
    run.exits(1);
    assert_eq!(
        first_problem(&run),
        (5, 55, "[QNT000]".to_owned()),
        "the build before the switch, with Quint 0.33.1, printed:\n\
         unreadable  specs/a.qnt\n  \
         specs/a.qnt:5:55  `quint parse`  [QNT000] no viable alternative at input '+}}'\n\
         where the switched build places it at 5:46, the start of `x' = v +`\n{run}"
    );
}

#[test]
fn a_syntax_error_in_the_body_after_a_local_def_keeps_quints_place_and_code() {
    let sandbox = Sandbox::empty();
    sandbox.write(
        "specs/a.qnt",
        "/// @check
module A {
  var x: int
  action init = x' = 0
  action step = {
    def g(a) = a
    x' = 1 +
  }
  /// @check
  val ok = x >= 0
}
",
    );
    let run = with_no_programs(&sandbox, &["verify", "--no-cache", "--json", "specs/a.qnt"]);
    run.exits(1);
    assert_eq!(
        first_problem(&run),
        (8, 3, "[QNT000]".to_owned()),
        "the build before the switch, with Quint 0.33.1, printed:\n\
         unreadable  specs/a.qnt\n  \
         specs/a.qnt:8:3  `quint parse`  [QNT000] no viable alternative at input '+}}'\n\
         where the switched build prints 7:5 `[QNT006] unexpected '=', did you mean '=='?`\n{run}"
    );
}
