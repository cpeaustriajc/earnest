#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{binary, rule};

const RECORDED: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
main=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "--main" ]; then main="$argument"; fi
  if [ "$previous" = "--out" ]; then out="$argument"; fi
  if [ "$previous" = "--source-map" ]; then map="$argument"; fi
  previous="$argument"
done
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  parse)
    folder="$(cd "$(dirname "$2")" && pwd)"
    cp "$here/parse.json" "$out"
    sed "s#\"0\":\"#\"0\":\"$folder/#" "$here/sourcemap.json" > "$map"
    ;;
  typecheck)
    exit 1
    ;;
  compile)
    cat "$here/compile.$main.json"
    ;;
  verify)
    cat "$here/verify.txt"
    exit 1
    ;;
  test)
    cat "$here/test.txt"
    exit 1
    ;;
esac
"##;

const NO_TLC: &str = "#!/bin/sh\necho 'Error: TLC is not here' >&2\nexit 1\n";

const CASE: &str = "bignum";

const HOLDING: [&str; 4] = [
    "sumIsPositive",
    "squareIsWide",
    "powersAreOrdered",
    "quotientIsExact",
];

const TLC_LIMIT: &str = "the model declares the number 18446744073709551616, and TLC's \
                         integers hold 32 bits, from -2147483648 to 2147483647";

fn case_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("models/quint_integers")
        .join(CASE)
}

fn recorded(name: &str) -> String {
    let path = case_folder().join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

fn program(path: &Path, text: &str) {
    fs::write(path, text).expect("the fake program can be written");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
}

fn ran(arguments: &[&str]) -> (String, String) {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    let java = folder.path().join("java");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    fs::create_dir_all(&java).expect("the folder of the stand-in for Java can be made");
    program(&fake.join("quint"), RECORDED);
    program(&java.join("java"), NO_TLC);
    for name in [
        "parse.json",
        "sourcemap.json",
        "compile.bignum.json",
        "verify.txt",
        "test.txt",
    ] {
        fs::write(fake.join(name), recorded(name)).expect("a recorded output can be copied");
    }
    fs::write(folder.path().join("bignum.qnt"), recorded("bignum.qnt"))
        .expect("the model can be copied");
    fs::write(folder.path().join("tla2tools.jar"), "").expect("a stand-in jar can be written");
    let output = Command::new(binary())
        .args(arguments)
        .env("EARNEST_QUINT", fake.join("quint"))
        .env("PATH", format!("{}:/usr/bin:/bin", java.display()))
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn verified() -> Value {
    let (stdout, stderr) = ran(&["verify", "--json", "--no-cache", "."]);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("earnest verify prints JSON ({error}):\n{stdout}\n{stderr}"))
}

fn checked(report: &Value) -> &Value {
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == CASE))
        .unwrap_or(&Value::Null)
}

fn quint_test_verdicts() -> Vec<(String, &'static str)> {
    recorded("test.txt")
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            if let Some(rest) = line.strip_prefix("ok ") {
                let name = rest.split_whitespace().next()?;
                return Some((name.to_owned(), "holds"));
            }
            let (_, rest) = line.split_once(") ")?;
            let name = rest.strip_suffix(" test(s)")?.split_whitespace().next()?;
            rest.contains(" failed after ")
                .then(|| (name.to_owned(), "broken"))
        })
        .collect()
}

#[test]
fn sums_products_and_powers_past_64_bits_give_the_verdicts_of_quint_test() {
    let report = verified();
    let model = checked(&report);
    let runs = quint_test_verdicts();
    assert_eq!(
        runs.len(),
        4,
        "`quint test --backend typescript` of Quint 0.33.1 judged the four runs: {runs:?}"
    );
    for (run, verdict) in &runs {
        assert_eq!(
            rule(model, run)["verdict"],
            *verdict,
            "`quint test --backend typescript` says the run `{run}` {verdict}, with sums, \
             products and powers past 2^64 in its actions and its expectation\n{report:#}"
        );
    }
}

#[test]
fn invariants_past_64_bits_give_the_verdicts_of_quint_run() {
    let report = verified();
    let model = checked(&report);
    for invariant in HOLDING {
        assert_eq!(
            rule(model, invariant)["verdict"],
            "holds",
            "`quint run --backend typescript --invariant {invariant}` of Quint 0.33.1 finds no \
             violation in the five states it reaches\n{report:#}"
        );
    }
    assert_eq!(
        (&model["status"], &model["states"]),
        (&Value::from("fail"), &Value::from(5)),
        "the counter goes from 2^63 - 2 to 2^63 + 2 and stops, as `quint run` shows\n{report:#}"
    );
}

#[test]
fn a_counter_that_passes_2_to_the_63_is_checked_and_its_path_prints_the_numbers_exactly() {
    let report = verified();
    let broken = rule(checked(&report), "belowTwoToThe63Plus2");
    let steps = broken["path"]["steps"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        (&broken["verdict"], steps.len()),
        (&Value::from("broken"), 3),
        "`quint run --invariant belowTwoToThe63Plus2` breaks it at the third step, where `n` is \
         9223372036854775809\n{report:#}"
    );
    let last = steps.last().map_or(&Value::Null, |step| &step["state"]);
    assert_eq!(
        last,
        &serde_json::json!([
            {"name": "n", "value": {"#bigint": "9223372036854775809"}},
            {"name": "p", "value": {"#bigint": "147573952589676412928"}}
        ]),
        "a number past 64 bits is written as ITF writes it, with every digit\n{report:#}"
    );
    let (text, _) = ran(&["verify", "--no-cache", "."]);
    assert!(
        text.contains(
            "3. step  n: 9223372036854775808 -> 9223372036854775809, p: 73786976294838206464 \
             -> 147573952589676412928"
        ),
        "the path prints the numbers as Quint's REPL writes them\n{text}"
    );
}

#[test]
fn confirm_with_quint_cannot_confirm_numbers_past_32_bits_and_names_tlc_s_limit() {
    let (stdout, stderr) = ran(&["confirm", "--quint", "."]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("cannot   bignum  5 states  fail  (Quint)"),
        "`quint verify --backend tlc` stops with TLC's `can't handle a number this big`\n\
         {stdout}\n{stderr}"
    );
    let why = "    Quint can't confirm this: Quint stopped: TLC can't handle a number this big.";
    assert!(
        lines
            .iter()
            .skip(1)
            .filter(|line| line.starts_with("    "))
            .all(|line| *line == why),
        "each line says why in TLC's words\n{stdout}"
    );
    assert!(
        !stdout.contains("differs"),
        "no answer is called different\n{stdout}"
    );
}

#[test]
fn confirm_with_tlc_cannot_confirm_numbers_past_32_bits_and_names_tlc_s_limit() {
    let (stdout, stderr) = ran(&["confirm", "--tlc", "tla2tools.jar", "."]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("cannot   bignum  5 states  fail"),
        "{stdout}\n{stderr}"
    );
    assert!(
        lines.contains(&format!("  TLC can't confirm this model: {TLC_LIMIT}").as_str()),
        "the reason names TLC's limit\n{stdout}"
    );
    assert!(
        !stdout.contains("differs"),
        "no answer is called different\n{stdout}"
    );
}
