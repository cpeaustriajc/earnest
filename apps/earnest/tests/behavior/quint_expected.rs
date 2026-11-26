#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::support::{Run, Sandbox, model, repository, rule, without_timings};

const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
case "$1" in
  --version)
    echo "0.33.1"
    exit 0
    ;;
  compile)
    main=""
    invariants=""
    temporal=""
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--main" ]; then main="$argument"; fi
      if [ "$previous" = "--invariant" ]; then invariants="$argument"; fi
      if [ "$previous" = "--temporal" ]; then temporal="$argument"; fi
      previous="$argument"
    done
    if [ -f "$here/compiled/$main@$invariants@$temporal.json" ]; then
      cat "$here/compiled/$main@$invariants@$temporal.json"
    else
      echo "no fixture for $main@$invariants@$temporal" >&2
      exit 1
    fi
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
    cp "$here/parsed/$name.parse.json" "$out"
    sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" \
      "$here/parsed/$name.sourcemap.json" > "$map"
    ;;
esac
"##;

const COMPILED: [(&str, &str); 6] = [
    ("expect@small@", "expect-small.json"),
    ("expect@small,tiny@", "expect-small-tiny.json"),
    ("expect@nonNegative,small@", "expect-nonNegative-small.json"),
    ("counter@inRange@", "counter-inRange.json"),
    ("swing@@safe,swingFair", "swing-safe-swingFair.json"),
    ("stuck@ok@", "stuck.json"),
];

const MODULES: [&str; 4] = ["expect", "counter", "swing", "stuck"];

const NO_END: &str = "every expected counterexample was found";

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
    fn with_checks(checks: &str) -> Self {
        let sandbox = Sandbox::empty();
        let program = sandbox.path().join("fake/quint");
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the fake can be made a program");
        for (compiled, fixture) in COMPILED {
            sandbox.write(&format!("fake/compiled/{compiled}.json"), &sample(fixture));
        }
        for module in MODULES {
            for kind in ["parse", "sourcemap"] {
                let name = format!("{module}.{kind}.json");
                sandbox.write(&format!("fake/parsed/{name}"), &sample(&name));
            }
            sandbox.write(
                &format!("models/{module}.qnt"),
                &sample(&format!("{module}.qnt")),
            );
        }
        sandbox.write("fake/calls", "");
        let quint = Self { sandbox, program };
        quint.checks(checks);
        quint
    }

    fn checks(&self, checks: &str) {
        self.sandbox.write(
            "models/earnest.quint.json",
            &format!("{{ \"checks\": [{checks}] }}"),
        );
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

    fn compiles(&self) -> usize {
        self.sandbox
            .read("fake/calls")
            .lines()
            .filter(|call| call.starts_with("compile"))
            .count()
    }

    fn forget_calls(&self) {
        self.sandbox.write("fake/calls", "");
    }
}

const SMALL: &str = r#"{ "file": "expect.qnt", "main": "expect", "name": "bugSmall", "expectCounterexample": ["small"] }"#;
const SMALL_AND_TINY: &str = r#"{ "file": "expect.qnt", "main": "expect", "name": "bugsSmallTiny", "expectCounterexample": ["small", "tiny"] }"#;
const WITH_ORDINARY: &str = r#"{ "file": "expect.qnt", "main": "expect", "name": "bugAndSafety", "invariants": ["nonNegative"], "expectCounterexample": ["small"] }"#;

fn names(report: &serde_json::Value) -> Vec<String> {
    report["models"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|each| each["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn message_of(report: &serde_json::Value) -> String {
    report["unreadable"][0]["message"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn feature_of(report: &serde_json::Value) -> String {
    report["unreadable"][0]["feature"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn two_named_checks_of_one_module_are_two_models_selected_by_their_names() {
    let quint = Quint::with_checks(&format!("{SMALL}, {SMALL_AND_TINY}"));
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(0);
    let report = run.json();
    assert_eq!(names(&report), ["bugSmall", "bugsSmallTiny"], "{run}");
    assert_eq!(
        model(&report, "bugsSmallTiny")["file"],
        "models/expect.qnt",
        "the name is the model's, the file stays the module's\n{run}"
    );
    let picked = quint.earnest(&["verify", "--json", "--model", "bugsSmallTiny"]);
    picked.exits(0);
    assert_eq!(names(&picked.json()), ["bugsSmallTiny"], "{picked}");
}

#[test]
fn two_checks_that_compile_alike_start_one_compile() {
    let again = r#"{ "file": "expect.qnt", "main": "expect", "name": "bugSmallAgain", "expectCounterexample": ["small"] }"#;
    let quint = Quint::with_checks(&format!("{SMALL}, {again}, {SMALL_AND_TINY}"));
    let run = quint.earnest(&["verify", "--jobs", "4"]);
    run.exits(0);
    assert_eq!(
        quint.compiles(),
        2,
        "one compile for the two checks of `small`, one for `small,tiny`\n{run}"
    );
    quint.forget_calls();
    let cached = quint.earnest(&["verify"]);
    cached.exits(0);
    assert_eq!(quint.compiles(), 0, "every compile is kept\n{cached}");
}

#[test]
fn two_checks_of_one_module_without_names_are_unreadable_and_told_to_add_a_name() {
    let plain = r#"{ "file": "expect.qnt", "main": "expect", "expectCounterexample": ["small"] }"#;
    let quint = Quint::with_checks(&format!("{plain}, {plain}"));
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(feature_of(&report), "`checks[1]`", "{run}");
    assert!(message_of(&report).contains("its own `name`"), "{run}");
    let clash = r#"{ "file": "counter.qnt", "main": "counter", "name": "bugSmall", "expectCounterexample": ["inRange"] }"#;
    quint.checks(&format!("{SMALL}, {clash}"));
    let named = quint.earnest(&["verify", "--json"]);
    named.exits(1);
    let report = named.json();
    assert_eq!(feature_of(&report), "`checks[1].name`", "{named}");
    assert!(message_of(&report).contains("its own `name`"), "{named}");
    for wrong in [r#""""#, r#"" padded ""#, "3"] {
        quint.checks(&format!(
            r#"{{ "file": "expect.qnt", "main": "expect", "name": {wrong} }}"#
        ));
        let refused = quint.earnest(&["verify", "--json"]);
        refused.exits(1);
        assert_eq!(feature_of(&refused.json()), "`checks[0].name`", "{refused}");
    }
    quint.checks(r#"{ "file": "expect.qnt", "main": "expect", "expectCounterexample": "small" }"#);
    let listed = quint.earnest(&["verify", "--json"]);
    listed.exits(1);
    assert_eq!(
        feature_of(&listed.json()),
        "`checks[0].expectCounterexample`",
        "{listed}"
    );
}

#[test]
fn a_met_expectation_prints_one_line_and_keeps_its_path_in_the_json() {
    let quint = Quint::with_checks(SMALL);
    let run = quint.earnest(&["verify"]);
    run.exits(0);
    assert_eq!(
        run.model_line("bugSmall"),
        [
            "pass",
            "bugSmall",
            "5 states",
            &run.model_line("bugSmall")[3],
            &format!("({NO_END})")
        ],
        "{run}"
    );
    assert_eq!(
        run.detail("bugSmall"),
        ["broken  small  (expected)"],
        "{run}"
    );
    let json = quint.earnest(&["verify", "--json", "--no-cache"]);
    json.exits(0);
    let report = json.json();
    let found = model(&report, "bugSmall");
    assert_eq!(found["status"], "pass", "{json}");
    assert_eq!(found["reason"], NO_END, "{json}");
    let small = rule(found, "small");
    assert_eq!(
        (&small["verdict"], &small["expected"]),
        (&serde_json::json!("broken"), &serde_json::json!("broken")),
        "{json}"
    );
    assert_eq!(
        small["path"]["steps"].as_array().map(Vec::len),
        Some(3),
        "the shortest counterexample, n = 3\n{json}"
    );
}

#[test]
fn an_expected_invariant_that_holds_fails_the_check() {
    let quint = Quint::with_checks(
        r#"{ "file": "counter.qnt", "main": "counter", "expectCounterexample": ["inRange"] }"#,
    );
    let run = quint.earnest(&["verify"]);
    run.exits(1);
    assert_eq!(run.model_line("counter")[0], "fail", "{run}");
    assert_eq!(
        run.detail("counter"),
        ["holds  inRange  a counterexample was expected"],
        "{run}"
    );
    let json = quint.earnest(&["verify", "--json"]);
    let report = json.json();
    let found = rule(model(&report, "counter"), "inRange");
    assert_eq!(
        (&found["verdict"], &found["expected"]),
        (&serde_json::json!("holds"), &serde_json::json!("broken")),
        "{json}"
    );
}

#[test]
fn an_expected_invariant_not_reached_leaves_the_check_unfinished() {
    let quint = Quint::with_checks(SMALL);
    let run = quint.earnest(&["verify", "--json", "--max-states", "2"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, "bugSmall");
    assert_eq!(found["status"], "unfinished", "{run}");
    assert_eq!(rule(found, "small")["verdict"], "unknown", "{run}");
}

#[test]
fn a_model_with_no_end_passes_by_stopping_and_an_ordinary_invariant_keeps_it_searching() {
    let quint = Quint::with_checks(&format!("{SMALL_AND_TINY}, {WITH_ORDINARY}"));
    let run = quint.earnest(&["verify", "--json", "--max-states", "1000"]);
    run.exits(1);
    let report = run.json();
    let stopped = model(&report, "bugsSmallTiny");
    assert_eq!(
        (&stopped["status"], &stopped["states"], &stopped["reason"]),
        (
            &serde_json::json!("pass"),
            &serde_json::json!(5),
            &serde_json::json!(NO_END)
        ),
        "stops after the level of n = 3, and counts the states found in it\n{run}"
    );
    let tiny = rule(stopped, "tiny");
    assert_eq!(
        tiny["path"]["steps"].as_array().map(Vec::len),
        Some(2),
        "each keeps its own shortest path\n{run}"
    );
    let searched = model(&report, "bugAndSafety");
    assert_eq!(searched["status"], "unfinished", "{run}");
    assert_eq!(searched["reason"], "state limit of 1000", "{run}");
    assert_eq!(rule(searched, "small")["verdict"], "broken", "{run}");
    assert_eq!(rule(searched, "nonNegative")["verdict"], "unknown", "{run}");
}

#[test]
fn rule_selects_an_expected_and_an_ordinary_property() {
    let quint = Quint::with_checks(WITH_ORDINARY);
    let expected = quint.earnest(&[
        "verify",
        "--json",
        "--rule",
        "small",
        "--max-states",
        "1000",
    ]);
    expected.exits(0);
    let report = expected.json();
    let found = model(&report, "bugAndSafety");
    assert_eq!(
        (&found["status"], &found["reason"]),
        (&serde_json::json!("pass"), &serde_json::json!(NO_END)),
        "only `small` needs an answer, so the search stops\n{expected}"
    );
    let ordinary = quint.earnest(&[
        "verify",
        "--json",
        "--rule",
        "nonNegative",
        "--max-states",
        "1000",
    ]);
    ordinary.exits(1);
    let report = ordinary.json();
    let found = model(&report, "bugAndSafety");
    assert_eq!(found["status"], "unfinished", "{ordinary}");
    assert_eq!(
        found["rules"].as_array().map(Vec::len),
        Some(1),
        "{ordinary}"
    );
    assert!(
        rule(found, "nonNegative").get("expected").is_none(),
        "{ordinary}"
    );
}

#[test]
fn an_expected_temporal_property_is_met_by_its_counterexample() {
    let quint = Quint::with_checks(
        r#"{ "file": "swing.qnt", "main": "swing", "temporal": ["safe"], "expectCounterexample": ["swingFair"] }"#,
    );
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(0);
    let report = run.json();
    let found = model(&report, "swing");
    assert_eq!(found["status"], "pass", "{run}");
    assert!(
        found["reason"].is_null(),
        "a temporal property needs every state\n{run}"
    );
    let fair = rule(found, "swingFair");
    assert_eq!(
        (&fair["verdict"], &fair["expected"], &fair["kind"]),
        (
            &serde_json::json!("broken"),
            &serde_json::json!("broken"),
            &serde_json::json!("eventually")
        ),
        "{run}"
    );
    assert!(rule(found, "safe").get("expected").is_none(), "{run}");
}

#[test]
fn an_expected_run_is_played_and_met_when_it_breaks() {
    let quint = Quint::with_checks(
        r#"{ "file": "stuck.qnt", "main": "stuck", "invariants": ["ok"], "expectCounterexample": ["wrongExpectTest"] }"#,
    );
    let run = quint.earnest(&["verify"]);
    run.exits(0);
    assert_eq!(
        run.detail("stuck"),
        ["broken  wrongExpectTest  (expected)"],
        "{run}"
    );
    quint.checks(
        r#"{ "file": "stuck.qnt", "main": "stuck", "invariants": ["ok"], "expectCounterexample": ["toTwoTest"] }"#,
    );
    let held = quint.earnest(&["verify"]);
    held.exits(1);
    assert_eq!(
        held.detail("stuck"),
        ["holds  toTwoTest  a counterexample was expected"],
        "{held}"
    );
}

#[test]
fn a_name_the_module_does_not_have_as_a_property_is_unreadable_and_named() {
    let quint = Quint::with_checks(
        r#"{ "file": "stuck.qnt", "main": "stuck", "invariants": ["ok"], "expectCounterexample": ["step"] }"#,
    );
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(feature_of(&report), "`step`", "{run}");
    assert!(
        message_of(&report).contains("not an invariant, a temporal property or a run"),
        "{run}"
    );
}

#[test]
fn a_cached_run_and_any_number_of_jobs_give_what_a_fresh_run_gives() {
    let quint = Quint::with_checks(&format!("{SMALL}, {SMALL_AND_TINY}, {WITH_ORDINARY}"));
    let arguments = ["verify", "--json", "--max-states", "1000"];
    let fresh = quint.earnest(&[&arguments[..], &["--no-cache", "--jobs", "1"]].concat());
    let cached = quint.earnest(&arguments);
    let wide = quint.earnest(&[&arguments[..], &["--no-cache", "--jobs", "4"]].concat());
    let fresh_report = without_timings(&fresh.json());
    assert!(
        cached.json()["models"]
            .as_array()
            .is_some_and(|all| all.iter().all(|each| each["cached"] == true)),
        "{cached}"
    );
    assert_eq!(without_timings(&cached.json()), fresh_report, "{cached}");
    assert_eq!(without_timings(&wide.json()), fresh_report, "{wide}");
}

#[test]
fn editing_the_expectations_or_the_name_checks_again_without_compiling_again() {
    let quint = Quint::with_checks(SMALL);
    quint.earnest(&["verify"]).exits(0);
    quint.forget_calls();
    quint.checks(
        r#"{ "file": "expect.qnt", "main": "expect", "name": "renamed", "expectCounterexample": ["small"] }"#,
    );
    let renamed = quint.earnest(&["verify"]);
    renamed.exits(0);
    assert_eq!(quint.compiles(), 0, "{renamed}");
    assert!(
        !renamed
            .model_line("renamed")
            .contains(&"(cached)".to_owned()),
        "{renamed}"
    );
    quint.checks(
        r#"{ "file": "expect.qnt", "main": "expect", "name": "renamed", "invariants": ["small"] }"#,
    );
    let ordinary = quint.earnest(&["verify"]);
    ordinary.exits(1);
    assert_eq!(quint.compiles(), 0, "the same compile serves\n{ordinary}");
    assert_eq!(
        ordinary.model_line("renamed")[0],
        "fail",
        "`small` is no longer expected to break\n{ordinary}"
    );
}

#[test]
fn editing_the_runs_alone_checks_again() {
    let quint = Quint::with_checks(
        r#"{ "file": "stuck.qnt", "main": "stuck", "invariants": ["ok"], "runs": true }"#,
    );
    quint.earnest(&["verify"]).exits(1);
    quint.checks(r#"{ "file": "stuck.qnt", "main": "stuck", "invariants": ["ok"] }"#);
    let without_runs = quint.earnest(&["verify"]);
    without_runs.exits(0);
    assert!(
        !without_runs
            .model_line("stuck")
            .contains(&"(cached)".to_owned()),
        "the result with the runs is not served for the check without them\n{without_runs}"
    );
}
