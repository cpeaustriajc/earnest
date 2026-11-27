#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::support::{Run, Sandbox, find_model, model, repository, without_timings};
use serde_json::Value;

const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
case "$1" in
  --version)
    echo "VERSION"
    exit 0
    ;;
  compile)
    main=""
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--main" ]; then main="$argument"; fi
      previous="$argument"
    done
    if [ -f "$here/pause" ]; then
      mkdir -p "$here/running"
      touch "$here/running/$$"
      ls "$here/running" | wc -l >> "$here/peaks"
      sleep 0.3
      rm "$here/running/$$"
    fi
    if [ -f "$here/fails/$main" ]; then
      cat "$here/fails/$main" >&2
      exit 1
    fi
    cat "$here/compiled/$main.json"
    ;;
  typecheck)
    name="$(basename "$2" .qnt)"
    for argument in "$@"; do
      if [ "$previous" = "--out" ]; then out="$argument"; fi
      previous="$argument"
    done
    if [ ! -f "$here/parsed/$name.typecheck.json" ]; then
      echo "error: no typecheck of $name.qnt is recorded" >&2
      exit 1
    fi
    cp "$here/parsed/$name.typecheck.json" "$out"
    ;;
  parse)
    file="$2"
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--out" ]; then out="$argument"; fi
      if [ "$previous" = "--source-map" ]; then map="$argument"; fi
      previous="$argument"
    done
    name="$(basename "$file" .qnt)"
    folder="$(cd "$(dirname "$file")" && pwd)"
    if [ -f "$here/parsed/$name.parse.json" ]; then
      cp "$here/parsed/$name.parse.json" "$out"
      sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" \
        "$here/parsed/$name.sourcemap.json" > "$map"
    else
      echo "error: no parse of $name.qnt is recorded" >&2
      exit 1
    fi
    ;;
esac
"##;

const FAILED_COMPILE: &str = "
 Error [QNT404]: Name 'y' not found

  at FILE:4:22
  4:   action step = x' = y + 1
                          ^

error: parsing failed
";

fn checks(name: &str) -> String {
    let path = repository().join("tasks/quint/checks").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

fn sample(name: &str) -> String {
    let path = repository().join("tasks/quint/models").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

struct Quint {
    sandbox: Sandbox,
    program: PathBuf,
}

impl Quint {
    fn new() -> Self {
        let sandbox = Sandbox::empty();
        let program = sandbox.path().join("fake/quint");
        let quint = Self { sandbox, program };
        quint.with_version("0.33.1");
        let Self { sandbox, program } = quint;
        for module in ["counter", "dead", "starts", "mixed", "choose"] {
            sandbox.write(
                &format!("fake/compiled/{module}.json"),
                &sample(&format!("{module}.json")),
            );
        }
        sandbox.write("fake/calls", "");
        Self { sandbox, program }
    }

    fn with_version(&self, version: &str) {
        self.sandbox
            .write("fake/quint", &FAKE.replace("VERSION", version));
        fs::set_permissions(&self.program, fs::Permissions::from_mode(0o755))
            .expect("the fake can be made a program");
    }

    fn with_check(&self, folder: &str, variant: &str, file: &str) {
        self.sandbox.write(
            &format!("{folder}/{file}.qnt"),
            &checks(&format!("{variant}/{file}.qnt")),
        );
        for kind in ["parse", "sourcemap"] {
            self.sandbox.write(
                &format!("fake/parsed/{file}.{kind}.json"),
                &checks(&format!("fixtures/{variant}/{file}.{kind}.json")),
            );
        }
    }

    fn earnest(&self, arguments: &[&str]) -> Run {
        let empty = self.sandbox.path().join("no-programs");
        fs::create_dir_all(&empty).expect("an empty folder can be made");
        Run::of(
            self.sandbox
                .command()
                .env("EARNEST_QUINT", &self.program)
                .env("PATH", &empty)
                .args(arguments),
        )
    }

    fn without_quint(&self, arguments: &[&str]) -> Run {
        let empty = self.sandbox.path().join("no-programs");
        fs::create_dir_all(&empty).expect("an empty folder can be made");
        Run::of(
            self.sandbox
                .command()
                .env_remove("EARNEST_QUINT")
                .env("PATH", &empty)
                .args(arguments),
        )
    }

    fn calls(&self) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn compiles(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|call| call.starts_with("compile"))
            .collect()
    }
}

fn problems(report: &serde_json::Value) -> Vec<(String, u64, u64, String)> {
    report["unreadable"]
        .as_array()
        .unwrap_or_else(|| panic!("the report has no `unreadable` list: {report}"))
        .iter()
        .map(|problem| {
            (
                problem["file"].as_str().unwrap_or_default().to_owned(),
                problem["line"].as_u64().unwrap_or_default(),
                problem["column"].as_u64().unwrap_or_default(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn message_of(report: &serde_json::Value) -> String {
    report["unreadable"][0]["message"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn a_marked_check_is_compiled_with_its_module_init_step_and_properties_and_checked() {
    let quint = Quint::new();
    quint.with_check("models", "counter", "counter");
    quint.with_check("models", "temporal", "dead");
    let run = quint.earnest(&["verify"]);
    run.exits(1);
    assert_eq!(
        quint.calls(),
        Vec::<String>::new(),
        "earnest reads the files itself and starts no `quint`\n{run}"
    );
    assert_eq!(run.model_line("counter")[0], "fail", "{run}");
    assert_eq!(run.model_line("counter")[2], "12 states", "{run}");
    assert_eq!(run.model_line("dead")[0], "pass", "{run}");
    assert_eq!(
        run.summary_line(),
        "2 models: 1 pass, 1 fail, 0 unfinished",
        "{run}"
    );
    let report = quint.earnest(&["verify", "--json"]).json();
    let counter = model(&report, "counter");
    assert_eq!(
        counter["file"], "models/counter.qnt",
        "the `.qnt` file is the file"
    );
    let ids = |found: &Value| -> Vec<String> {
        found["rules"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|rule| rule["id"].as_str().map(str::to_owned))
            .collect()
    };
    assert_eq!(
        ids(counter),
        ["inRange", "tooBig"],
        "the check is compiled with its module, `init`, `step` and both invariants\n{report}"
    );
    assert_eq!(
        ids(model(&report, "dead")),
        ["ok", "later", "sooner"],
        "and with its invariant and both temporal properties\n{report}"
    );
    assert_eq!(report["summary"]["models"], 2, "{report}");
}

#[test]
fn an_unchanged_check_comes_from_the_cache_and_an_edit_checks_only_its_folder_again() {
    let quint = Quint::new();
    quint.with_check("first", "counter", "counter");
    quint.with_check("second", "dead", "dead");
    let first = quint.earnest(&["verify"]);
    first.exits(1);
    let cached = |run: &Run, name: &str| run.model_line(name).contains(&"(cached)".to_owned());
    assert!(
        !cached(&first, "counter") && !cached(&first, "dead"),
        "both checks are read\n{first}"
    );
    let again = quint.earnest(&["verify"]);
    again.exits(1);
    assert!(
        cached(&again, "counter") && cached(&again, "dead"),
        "the result cache serves both unchanged checks: {again}"
    );
    let marked = quint.sandbox.read("second/dead.qnt");
    quint
        .sandbox
        .write("second/dead.qnt", &marked.replace("a <= 3", "a <= 4"));
    let edited = quint.earnest(&["verify"]);
    edited.exits(1);
    assert!(
        cached(&edited, "counter") && !cached(&edited, "dead"),
        "only the check of the edited folder is checked again\n{edited}"
    );
    assert_eq!(quint.calls(), Vec::<String>::new(), "no `quint` is started");
    assert!(
        !quint.sandbox.path().join(".earnest/quint").exists(),
        "nothing of Quint's is kept"
    );
}

#[test]
fn a_failed_compile_makes_its_check_unreadable_at_the_place_quint_gives() {
    let quint = Quint::new();
    quint.with_check("models", "inrange", "counter");
    quint.with_check("models", "dead", "dead");
    let dead = quint.sandbox.path().join("models/dead.qnt");
    quint.sandbox.write(
        "fake/fails/dead",
        &FAILED_COMPILE.replace("FILE", &dead.to_string_lossy()),
    );
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        problems(&report),
        [(
            "models/dead.qnt".to_owned(),
            4,
            22,
            "`quint compile --main dead`".to_owned()
        )],
        "{report}"
    );
    assert_eq!(
        message_of(&report),
        "Error [QNT404]: Name 'y' not found",
        "Quint's own first error line"
    );
    assert!(
        find_model(&report, "counter").is_some(),
        "the other check is still checked: {report}"
    );
    let text = quint.earnest(&["verify"]);
    assert_eq!(
        text.summary_line(),
        "2 models: 0 pass, 1 fail, 0 unfinished, 1 unreadable",
        "{text}"
    );
}

#[test]
fn without_quint_each_quint_file_is_checked_all_the_same() {
    let quint = Quint::new();
    quint.with_check("models", "counter", "counter");
    quint.with_check("models", "dead", "dead");
    let run = quint.without_quint(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        report["unreadable"],
        serde_json::json!([]),
        "no `quint` and no Node are needed: {report}"
    );
    assert_eq!(model(&report, "counter")["status"], "fail", "{report}");
    assert_eq!(model(&report, "dead")["status"], "pass", "{report}");
    let named = Run::of(
        quint
            .sandbox
            .command()
            .env("EARNEST_QUINT", quint.sandbox.path().join("nowhere/quint"))
            .env("PATH", quint.sandbox.path().join("no-programs"))
            .args(["verify", "--json"]),
    );
    assert_eq!(
        without_timings(&named.json()),
        without_timings(&report),
        "`EARNEST_QUINT` is read by `confirm --quint` only: {named}"
    );
}

#[test]
fn quint_in_node_modules_above_the_checks_is_not_started_by_verify() {
    let quint = Quint::new();
    quint.with_check("app/specs", "counter", "counter");
    fs::create_dir_all(quint.sandbox.path().join("app/node_modules/.bin"))
        .expect("the folder can be made");
    std::os::unix::fs::symlink(
        &quint.program,
        quint.sandbox.path().join("app/node_modules/.bin/quint"),
    )
    .expect("the link can be made");
    let run = quint.without_quint(&["verify"]);
    run.exits(1);
    assert_eq!(run.model_line("counter")[0], "fail", "{run}");
    assert_eq!(
        quint.calls(),
        Vec::<String>::new(),
        "`verify` reads the file itself, so the `quint` beside it is not started\n{run}"
    );
}

#[test]
fn a_checks_file_right_or_wrong_is_unreadable_at_itself_and_judges_nothing() {
    for checks in [
        r#"{ "checks": [{ "file": "counter.qnt", "main": "counter", "invariants": ["inRange"] }] }"#,
        r#"{ "checks": [{ "file": "counter.qnt", "main": "counter", "runs": 3 }] }"#,
        r#"{ "checks": [ "#,
    ] {
        let quint = Quint::new();
        quint.with_check("models", "plain", "counter");
        quint.sandbox.write("models/earnest.quint.json", checks);
        let run = quint.earnest(&["verify", "--json"]);
        run.exits(1);
        let report = run.json();
        assert_eq!(
            problems(&report),
            [(
                "models/earnest.quint.json".to_owned(),
                1,
                1,
                "`earnest.quint.json`".to_owned()
            )],
            "{checks}: {report}"
        );
        assert!(
            message_of(&report).starts_with("earnest no longer reads `earnest.quint.json`"),
            "{checks}: {report}"
        );
        assert_eq!(quint.compiles().len(), 0, "nothing is compiled: {checks}");
    }
}

#[test]
fn a_quint_file_named_with_no_marked_module_says_how_to_mark_one() {
    let quint = Quint::new();
    quint.with_check("models", "counter", "counter");
    quint.with_check("models", "plain", "dead");
    let run = quint.earnest(&["verify", "models/dead.qnt", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        problems(&report),
        [("models/dead.qnt".to_owned(), 1, 1, "`dead.qnt`".to_owned())],
        "{report}"
    );
    let message = message_of(&report);
    assert!(message.contains("`/// @check`"), "{message}");
    assert!(models_of(&report).is_empty(), "nothing else is checked");
    assert!(
        quint.compiles().is_empty(),
        "the file is parsed, and nothing is compiled: {:?}",
        quint.calls()
    );
    let lonely = Quint::new();
    lonely.with_check("alone", "plain", "counter");
    let alone = lonely.earnest(&["verify", "alone/counter.qnt", "--json"]);
    alone.exits(1);
    assert_eq!(
        problems(&alone.json())[0].3,
        "`counter.qnt`",
        "a file alone: {alone}"
    );
}

fn models_of(report: &serde_json::Value) -> Vec<String> {
    report["models"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|each| each["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn a_quint_file_named_on_the_command_line_checks_only_its_marked_modules() {
    let quint = Quint::new();
    quint.with_check("models", "inrange", "counter");
    quint.with_check("models", "dead", "dead");
    let run = quint.earnest(&["verify", "models/dead.qnt", "--json"]);
    run.exits(0);
    assert_eq!(models_of(&run.json()), ["dead"], "{run}");
}

#[test]
fn model_and_rule_select_across_checks() {
    let quint = Quint::new();
    quint.with_check("models", "counter", "counter");
    quint.with_check("models", "mixed", "mixed");
    let one = quint.earnest(&["verify", "--model", "mixed", "--json"]);
    one.exits(1);
    assert_eq!(models_of(&one.json()), ["mixed"], "{one}");
    let rule = quint.earnest(&["verify", "--rule", "inRange", "--json"]);
    rule.exits(0);
    let report = rule.json();
    assert_eq!(models_of(&report), ["counter"], "{rule}");
    assert_eq!(report["summary"]["pass"], 1, "{report}");
    let none = quint.earnest(&["verify", "--model", "nowhere"]);
    assert_ne!(none.code, Some(0), "no such model: {none}");
}

#[test]
fn a_refused_construct_is_reported_at_its_line_in_the_quint_file() {
    let quint = Quint::new();
    quint.with_check("models", "choose", "choose");
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    let expected = [(
        "models/choose.qnt".to_owned(),
        7,
        22,
        "`chooseSome`".to_owned(),
    )];
    assert_eq!(
        problems(&run.json()),
        expected,
        "line 7 is `action step = x' = Set(1, 2).chooseSome()`, and the call starts at its \
         receiver, column 22\n{run}"
    );
    let again = quint.earnest(&["verify", "--json"]);
    assert_eq!(
        problems(&again.json()),
        expected,
        "a second run places it alike\n{again}"
    );
}

#[test]
fn a_compiled_json_named_on_the_command_line_needs_no_quint() {
    let quint = Quint::new();
    quint
        .sandbox
        .write("models/counter.json", &sample("counter.json"));
    let run = quint.earnest(&["verify", "models/counter.json"]);
    run.exits(1);
    assert_eq!(run.model_line("counter")[0], "fail", "{run}");
    assert_eq!(quint.calls(), Vec::<String>::new(), "no `quint` is started");
    assert!(
        !Path::new(&quint.sandbox.path().join(".earnest/quint")).exists(),
        "nothing is compiled"
    );
}

#[test]
fn two_unreadable_checks_of_one_file_are_told_apart_by_their_module() {
    let quint = Quint::new();
    quint.with_check("models", "wide", "counter");
    let source = quint.sandbox.read("models/counter.qnt");
    quint.sandbox.write(
        "models/counter.qnt",
        &source.replace("count < 4", "count < true"),
    );
    let run = quint.earnest(&["verify"]);
    run.exits(1);
    let headers: Vec<&str> = run
        .stdout
        .lines()
        .filter(|line| line.starts_with("unreadable"))
        .collect();
    assert_eq!(
        headers,
        [
            "unreadable  models/counter.qnt  counter",
            "unreadable  models/counter.qnt  wide"
        ],
        "the module after the file, as on a result line\n{run}"
    );
}

#[test]
fn editing_a_file_imported_from_a_parent_folder_reads_the_check_again() {
    let quint = Quint::new();
    quint.with_check("app/specs", "imported/specs", "counter");
    quint.sandbox.write(
        "fake/parsed/counter.typecheck.json",
        &checks("fixtures/imported/specs/counter.typecheck.json"),
    );
    quint.with_check("app/shared", "imported/shared", "lib");
    let root = quint
        .sandbox
        .path()
        .canonicalize()
        .expect("the sandbox has a real path");
    let map = quint.sandbox.read("fake/parsed/counter.sourcemap.json");
    quint.sandbox.write(
        "fake/parsed/counter.sourcemap.json",
        &map.replace("ROOT", &root.join("app").to_string_lossy()),
    );
    let first = quint.earnest(&["verify"]);
    first.exits(1);
    let again = quint.earnest(&["verify"]);
    again.exits(1);
    assert!(
        again.model_line("counter").contains(&"(cached)".to_owned()),
        "nothing changed\n{again}"
    );
    quint.sandbox.write(
        "app/shared/lib.qnt",
        "module lib {\n  pure val Limit = 6\n}\n",
    );
    let edited = quint.earnest(&["verify"]);
    edited.exits(1);
    assert!(
        !edited
            .model_line("counter")
            .contains(&"(cached)".to_owned()),
        "the file read from `../shared` changed, so the check is read again\n{edited}"
    );
    assert_ne!(
        edited.model_line("counter")[2],
        first.model_line("counter")[2],
        "with a limit of 6 the counter reaches more states\n{first}\n{edited}"
    );
}

fn with_stuck(variant: &str) -> Quint {
    let quint = Quint::new();
    quint
        .sandbox
        .write("fake/compiled/stuck.json", &sample("stuck.json"));
    quint.with_check("models", variant, "stuck");
    quint
}

fn verdicts(report: &serde_json::Value) -> Vec<(String, String)> {
    model(report, "stuck")["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|rule| {
            (
                rule["id"].as_str().unwrap_or_default().to_owned(),
                rule["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn pairs(all: &[(&str, &str)]) -> Vec<(String, String)> {
    all.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

#[test]
fn a_check_plays_the_runs_whose_names_end_in_test() {
    let quint = with_stuck("stuck");
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    assert_eq!(
        verdicts(&run.json()),
        pairs(&[
            ("ok", "holds"),
            ("toTwoTest", "holds"),
            ("thirdTest", "broken"),
            ("withNondetTest", "holds"),
            ("assertedTest", "holds"),
            ("wrongExpectTest", "broken"),
        ]),
        "the invariant, then each run as quint test judges it\n{run}"
    );
}

#[test]
fn a_marked_run_is_played_and_one_marked_broken_is_met_when_it_breaks() {
    let quint = with_stuck("broken");
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(0);
    assert_eq!(
        verdicts(&run.json()),
        pairs(&[
            ("ok", "holds"),
            ("toTwoTest", "holds"),
            ("thirdTest", "broken"),
            ("withNondetTest", "holds"),
            ("assertedTest", "holds"),
            ("wrongExpectTest", "broken"),
        ]),
        "every run is played, and the two marked broken pass by breaking\n{run}"
    );
}

#[test]
fn a_setup_prints_short_names_and_its_rule_is_selected_by_the_printed_name() {
    let quint = Quint::new();
    quint
        .sandbox
        .write("fake/compiled/SmallBits.json", &sample("smallbits.json"));
    quint.with_check("models", "smallbits", "clockbits");
    let run = quint.earnest(&["verify", "--rule", "allHeard"]);
    run.exits(1);
    let detail = run.detail("SmallBits");
    let path = crate::support::path_under(&detail, &["broken", "allHeard"]);
    assert!(path.actions().is_empty(), "the run stops at once\n{run}");
    assert!(
        detail
            .iter()
            .any(|line| line.trim() == "start  time = 0, seen = 0"),
        "the fields by their short names\n{run}"
    );
    assert!(
        !run.to_string().contains("SmallBits::"),
        "no flattened name is printed\n{run}"
    );
    let report = quint.earnest(&["verify", "--json"]).json();
    let ids: Vec<&str> = model(&report, "SmallBits")["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|rule| rule["id"].as_str())
        .collect();
    assert_eq!(
        ids,
        ["allHeard", "timeAdvances", "fairHear", "leads"],
        "{report}"
    );
}

#[test]
fn a_pick_among_every_integer_is_reported_at_its_line_in_the_quint_file() {
    let quint = Quint::new();
    let fixture = |name: &str| {
        let path = repository()
            .join("crates/earnest_quint/fixtures")
            .join(name);
        fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
    };
    let source = fixture("allnumbers.qnt");
    let (opening, rest) = source
        .split_once('\n')
        .expect("the module opens on the first line");
    let body = rest
        .trim_end()
        .strip_suffix('}')
        .expect("the module closes on the last line");
    quint.sandbox.write(
        "models/allnumbers.qnt",
        &format!("/// @check ok\n{opening}{body}  val ok = true\n}}\n"),
    );
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    assert_eq!(
        problems(&run.json()),
        [(
            "models/allnumbers.qnt".to_owned(),
            4,
            30,
            "`nondet y`".to_owned(),
        )],
        "line 4 is `action step = {{ nondet y = Int.oneOf() x' = y }}`, and the pick starts at \
         `Int`, column 30\n{run}"
    );
    assert!(
        message_of(&run.json()).contains("`Int.oneOf()` picks among numbers without end"),
        "{run}"
    );
}
