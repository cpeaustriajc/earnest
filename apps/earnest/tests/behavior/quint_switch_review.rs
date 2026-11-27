#![cfg(unix)]

use std::fs;

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
