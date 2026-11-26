#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::support::{Run, Sandbox, find_model, model, repository};

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

    fn reads(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|call| call.starts_with("compile") || call.starts_with("typecheck"))
            .collect()
    }

    fn forget_calls(&self) {
        self.sandbox.write("fake/calls", "");
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
    let calls = quint.calls();
    assert_eq!(
        calls.first().map(String::as_str),
        Some("--version"),
        "the version first: {calls:?}"
    );
    let mut compiles = quint.compiles();
    compiles.sort();
    assert_eq!(
        compiles,
        [
            "compile --target json --main counter --init init --step step --invariant \
             inRange,tooBig models/counter.qnt",
            "compile --target json --main dead --init init --step step --invariant ok \
             --temporal later,sooner models/dead.qnt",
        ],
        "one compile per check side by side, the properties joined by commas\n{run}"
    );
    let mut parses: Vec<String> = calls
        .iter()
        .filter(|call| call.starts_with("parse"))
        .filter_map(|call| call.split(' ').nth(1).map(str::to_owned))
        .collect();
    parses.sort();
    assert_eq!(
        parses,
        ["models/counter.qnt", "models/dead.qnt"],
        "one parse per file, for the files it reads: {calls:?}"
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
    assert_eq!(report["summary"]["models"], 2, "{report}");
}

#[test]
fn compiles_run_side_by_side_but_never_more_than_jobs_at_once() {
    let quint = Quint::new();
    quint.sandbox.write("fake/pause", "");
    let specs = [
        ("inrange", "counter"),
        ("dead", "dead"),
        ("starts", "starts"),
        ("inbounds", "mixed"),
    ];
    for (variant, file) in specs {
        quint.with_check("models", variant, file);
    }
    let peaks = |jobs: &str| -> Vec<u64> {
        quint.sandbox.write("fake/peaks", "");
        let run = quint.earnest(&["verify", "--no-cache", "--jobs", jobs]);
        assert_eq!(quint.compiles().len(), specs.len(), "{run}");
        quint.forget_calls();
        fs::remove_dir_all(quint.sandbox.path().join(".earnest")).expect("the cache goes");
        quint
            .sandbox
            .read("fake/peaks")
            .lines()
            .map(|line| line.trim().parse().expect("a count"))
            .collect()
    };
    let alone = peaks("1");
    assert!(
        alone.iter().all(|peak| *peak == 1),
        "one at a time with one job: {alone:?}"
    );
    let two = peaks("2");
    assert!(
        two.iter().all(|peak| *peak <= 2),
        "at most two at a time with two jobs: {two:?}"
    );
    assert!(
        two.contains(&2),
        "two compiles overlap with two jobs: {two:?}"
    );
}

#[test]
fn an_unchanged_check_starts_no_quint_and_an_edit_compiles_only_its_folder_again() {
    let quint = Quint::new();
    quint.with_check("first", "counter", "counter");
    quint.with_check("second", "dead", "dead");
    let first = quint.earnest(&["verify"]);
    first.exits(1);
    assert_eq!(
        quint.compiles().len(),
        2,
        "both checks are compiled\n{first}"
    );
    quint.forget_calls();
    let again = quint.earnest(&["verify"]);
    again.exits(1);
    assert_eq!(
        quint.calls(),
        Vec::<String>::new(),
        "nothing changed, so `quint` is not started at all, not even for its version\n{again}"
    );
    assert!(
        again.model_line("counter").contains(&"(cached)".to_owned()),
        "the result cache serves the unchanged JSON: {again}"
    );
    let marked = quint.sandbox.read("second/dead.qnt");
    quint
        .sandbox
        .write("second/dead.qnt", &format!("{marked}// edited\n"));
    let edited = quint.earnest(&["verify"]);
    edited.exits(1);
    assert_eq!(
        quint.compiles(),
        [
            "compile --target json --main dead --init init --step step --invariant ok \
          second/dead.qnt"
        ],
        "only the check of the edited folder\n{edited}"
    );
    quint.forget_calls();
    quint.with_version("0.34.0-next");
    let upgraded = quint.earnest(&["verify"]);
    upgraded.exits(1);
    let calls = quint.calls();
    assert_eq!(
        calls.first().map(String::as_str),
        Some("--version"),
        "a changed program is asked its version again: {calls:?}"
    );
    assert_eq!(
        quint.compiles().len(),
        2,
        "a new version of Quint compiles everything again: {calls:?}"
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
fn without_quint_each_quint_file_is_unreadable_and_nothing_is_guessed() {
    let quint = Quint::new();
    quint.with_check("models", "counter", "counter");
    quint.with_check("models", "dead", "dead");
    let run = quint.without_quint(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        problems(&report),
        [
            ("models/counter.qnt".to_owned(), 1, 1, "`quint`".to_owned()),
            ("models/dead.qnt".to_owned(), 1, 1, "`quint`".to_owned()),
        ],
        "{report}"
    );
    assert_eq!(
        message_of(&report),
        "`quint` was not found: install it with `npm install --save-dev \
         @informalsystems/quint`, or set `EARNEST_QUINT`"
    );
    assert_eq!(report["summary"]["unreadable"], 2, "{report}");
    let named = Run::of(
        quint
            .sandbox
            .command()
            .env("EARNEST_QUINT", quint.sandbox.path().join("nowhere/quint"))
            .env("PATH", quint.sandbox.path().join("no-programs"))
            .args(["verify", "--json"]),
    );
    assert!(
        message_of(&named.json()).contains("`EARNEST_QUINT` names"),
        "a variable that names no file is said: {named}"
    );
}

#[test]
fn quint_in_node_modules_above_the_checks_is_used() {
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
    assert_eq!(quint.compiles().len(), 1, "{run}");
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
    assert_eq!(quint.compiles().len(), 1, "{run}");
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
    let parses = |calls: Vec<String>| {
        calls
            .iter()
            .filter(|call| call.starts_with("parse"))
            .count()
    };
    assert_eq!(parses(quint.calls()), 1, "one parse for the file");
    quint.forget_calls();
    let again = quint.earnest(&["verify", "--json"]);
    assert_eq!(problems(&again.json()), expected, "{again}");
    assert_eq!(
        quint.calls(),
        Vec::<String>::new(),
        "the places are cached too"
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
    for main in ["counter", "wide"] {
        quint.sandbox.write(
            &format!("fake/fails/{main}"),
            "error: name resolution failed\n",
        );
    }
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
    quint.earnest(&["verify"]).exits(1);
    assert_eq!(
        quint.reads().len(),
        1,
        "the first run reads the check from Quint"
    );
    quint.forget_calls();
    quint.earnest(&["verify"]).exits(1);
    assert_eq!(quint.calls(), Vec::<String>::new(), "nothing changed");
    quint.sandbox.write(
        "app/shared/lib.qnt",
        "module lib {\n  pure val Limit = 6\n}\n",
    );
    let edited = quint.earnest(&["verify"]);
    edited.exits(1);
    assert_eq!(
        quint.reads().len(),
        1,
        "the file Quint read from `../shared` changed, so the check is read from Quint again\n{edited}"
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
    quint
        .sandbox
        .write("fake/compiled/allNumbers.json", &fixture("allnumbers.json"));
    for name in ["allnumbers.parse.json", "allnumbers.sourcemap.json"] {
        quint
            .sandbox
            .write(&format!("fake/parsed/{name}"), &fixture(name));
    }
    quint
        .sandbox
        .write("models/allnumbers.qnt", &fixture("allnumbers.qnt"));
    let mut parsed: Value =
        serde_json::from_str(&fixture("allnumbers.parse.json")).expect("the parse fixture is JSON");
    for module in parsed["modules"].as_array_mut().into_iter().flatten() {
        if module["name"] == "allNumbers" {
            module["doc"] = Value::from("@check");
            if let Some(declarations) = module["declarations"].as_array_mut() {
                declarations.push(serde_json::json!({
                    "id": 999_999, "kind": "def", "name": "ok", "qualifier": "val", "doc": "@check"
                }));
            }
        }
    }
    quint
        .sandbox
        .write("fake/parsed/allnumbers.parse.json", &parsed.to_string());
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
