#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

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
    if [ -f "$here/fails/$main@$invariants@$temporal" ]; then
      cat "$here/fails/$main@$invariants@$temporal" >&2
      exit 1
    fi
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

const COMPILED: [(&str, &str); 8] = [
    ("expect@small@", "expect-small.json"),
    ("parts@@partly", "parts-partly.json"),
    ("parts@@wholly", "parts-wholly.json"),
    ("expect@small,tiny@", "expect-small-tiny.json"),
    ("expect@small,nonNegative@", "expect-nonNegative-small.json"),
    ("counter@inRange@", "counter-inRange.json"),
    ("swing@@swingFair,safe", "swing-safe-swingFair.json"),
    ("stuck@ok@", "stuck.json"),
];

const MODULES: [&str; 5] = ["expect", "counter", "swing", "stuck", "parts"];

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
    fn new() -> Self {
        let sandbox = Sandbox::empty();
        let program = sandbox.path().join("fake/quint");
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the fake can be made a program");
        for (compiled, fixture) in COMPILED {
            sandbox.write(&format!("fake/compiled/{compiled}.json"), &sample(fixture));
        }
        sandbox.write("fake/calls", "");
        Self { sandbox, program }
    }

    fn marked(module: &str, marks: &[(&str, &str)]) -> Self {
        let quint = Self::new();
        quint.mark(module, marks);
        quint
    }

    fn mark(&self, module: &str, marks: &[(&str, &str)]) {
        assert!(MODULES.contains(&module), "{module} is a sample");
        let mut parsed: Value = serde_json::from_str(&sample(&format!("{module}.parse.json")))
            .expect("the parse fixture is JSON");
        let doc_of = |name: &str| {
            marks
                .iter()
                .find(|(marked, _)| *marked == name)
                .map(|(_, mark)| Value::from(*mark))
        };
        for each in parsed["modules"].as_array_mut().into_iter().flatten() {
            if each["name"] == module {
                each["doc"] = Value::from("@check");
            }
            for declaration in each["declarations"].as_array_mut().into_iter().flatten() {
                if let Some(doc) = declaration["name"].as_str().and_then(doc_of) {
                    declaration["doc"] = doc;
                }
            }
        }
        self.sandbox.write(
            &format!("fake/parsed/{module}.parse.json"),
            &parsed.to_string(),
        );
        self.sandbox.write(
            &format!("fake/parsed/{module}.sourcemap.json"),
            &sample(&format!("{module}.sourcemap.json")),
        );
        let written: Vec<String> = marks
            .iter()
            .map(|(name, mark)| format!("{name} {mark}"))
            .collect();
        self.sandbox.write(
            &format!("models/{module}.qnt"),
            &format!(
                "{}// marked: {}\n",
                sample(&format!("{module}.qnt")),
                written.join(", ")
            ),
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

const SMALL: [(&str, &str); 1] = [("small", "@check broken")];
const SMALL_AND_TINY: [(&str, &str); 2] = [("small", "@check broken"), ("tiny", "@check broken")];
const WITH_ORDINARY: [(&str, &str); 2] = [("nonNegative", "@check"), ("small", "@check broken")];

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
fn two_marked_modules_are_two_models_and_model_selects_one() {
    let quint = Quint::marked("expect", &SMALL);
    quint.mark("counter", &[("inRange", "@check")]);
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(0);
    let report = run.json();
    assert_eq!(names(&report), ["counter", "expect"], "{run}");
    assert_eq!(
        model(&report, "expect")["file"],
        "models/expect.qnt",
        "a check is named after its module\n{run}"
    );
    let picked = quint.earnest(&["verify", "--json", "--model", "expect"]);
    picked.exits(0);
    assert_eq!(names(&picked.json()), ["expect"], "{picked}");
}

#[test]
fn a_met_expectation_prints_one_line_and_keeps_its_path_in_the_json() {
    let quint = Quint::marked("expect", &SMALL);
    let run = quint.earnest(&["verify"]);
    run.exits(0);
    assert_eq!(
        run.model_line("expect"),
        [
            "pass",
            "expect",
            "5 states",
            &run.model_line("expect")[3],
            "1 rule, 0 runs",
            &format!("({NO_END})")
        ],
        "{run}"
    );
    assert_eq!(run.detail("expect"), ["broken  small  (expected)"], "{run}");
    let json = quint.earnest(&["verify", "--json", "--no-cache"]);
    json.exits(0);
    let report = json.json();
    let found = model(&report, "expect");
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
    let quint = Quint::marked("counter", &[("inRange", "@check broken")]);
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
    let quint = Quint::marked("expect", &SMALL);
    let run = quint.earnest(&["verify", "--json", "--max-states", "2"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, "expect");
    assert_eq!(found["status"], "unfinished", "{run}");
    assert_eq!(rule(found, "small")["verdict"], "unknown", "{run}");
}

#[test]
fn a_model_with_no_end_passes_by_stopping_and_an_ordinary_invariant_keeps_it_searching() {
    let quint = Quint::marked("expect", &SMALL_AND_TINY);
    let run = quint.earnest(&["verify", "--json", "--max-states", "1000"]);
    run.exits(0);
    let report = run.json();
    let stopped = model(&report, "expect");
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
    quint.mark("expect", &WITH_ORDINARY);
    let run = quint.earnest(&["verify", "--json", "--max-states", "1000"]);
    run.exits(1);
    let report = run.json();
    let searched = model(&report, "expect");
    assert_eq!(searched["status"], "unfinished", "{run}");
    assert_eq!(searched["reason"], "state limit of 1000", "{run}");
    assert_eq!(rule(searched, "small")["verdict"], "broken", "{run}");
    assert_eq!(rule(searched, "nonNegative")["verdict"], "unknown", "{run}");
}

#[test]
fn rule_selects_an_expected_and_an_ordinary_property() {
    let quint = Quint::marked("expect", &WITH_ORDINARY);
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
    let found = model(&report, "expect");
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
    let found = model(&report, "expect");
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
    let quint = Quint::marked(
        "swing",
        &[("safe", "@check"), ("swingFair", "@check broken")],
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
    let quint = Quint::marked(
        "stuck",
        &[
            ("ok", "@check"),
            ("thirdTest", "@check broken"),
            ("wrongExpectTest", "@check broken"),
        ],
    );
    let run = quint.earnest(&["verify"]);
    run.exits(0);
    assert_eq!(
        run.detail("stuck"),
        [
            "broken  thirdTest  (expected)",
            "broken  wrongExpectTest  (expected)"
        ],
        "{run}"
    );
    quint.mark(
        "stuck",
        &[
            ("ok", "@check"),
            ("toTwoTest", "@check broken"),
            ("thirdTest", "@check broken"),
            ("wrongExpectTest", "@check broken"),
        ],
    );
    let held = quint.earnest(&["verify"]);
    held.exits(1);
    assert_eq!(
        held.detail("stuck"),
        [
            "holds  toTwoTest  a counterexample was expected",
            "broken  thirdTest  (expected)",
            "broken  wrongExpectTest  (expected)"
        ],
        "{held}"
    );
}

#[test]
fn a_mark_on_a_definition_that_is_no_property_is_unreadable_and_named() {
    let quint = Quint::marked("stuck", &[("ok", "@check"), ("step", "@check broken")]);
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(feature_of(&report), "`step`", "{run}");
    assert!(
        message_of(&report).contains("`step` is an `action`, which no check judges"),
        "{run}"
    );
}

#[test]
fn a_cached_run_and_any_number_of_jobs_give_what_a_fresh_run_gives() {
    let quint = Quint::marked("expect", &SMALL_AND_TINY);
    quint.mark("counter", &[("inRange", "@check")]);
    quint.mark(
        "swing",
        &[("safe", "@check"), ("swingFair", "@check broken")],
    );
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
fn editing_a_mark_checks_again() {
    let quint = Quint::marked("expect", &SMALL);
    quint.earnest(&["verify"]).exits(0);
    quint.forget_calls();
    quint.mark("expect", &[("small", "@check")]);
    let ordinary = quint.earnest(&["verify"]);
    ordinary.exits(1);
    assert!(
        !ordinary
            .model_line("expect")
            .contains(&"(cached)".to_owned()),
        "{ordinary}"
    );
    assert_eq!(
        ordinary.model_line("expect")[0],
        "fail",
        "`small` is no longer expected to break\n{ordinary}"
    );
    assert_eq!(
        quint.compiles(),
        1,
        "the mark is in the `.qnt` file, so the file is compiled again\n{ordinary}"
    );
}

#[test]
fn a_temporal_property_in_parts_is_met_when_one_part_is_broken() {
    let quint = Quint::marked("parts", &[("partly", "@check broken")]);
    let run = quint.earnest(&["verify", "--json"]);
    run.exits(0);
    let report = run.json();
    let found = model(&report, "parts");
    let parts: Vec<(String, String, String)> = found["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
                each["expected"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        parts,
        [
            ("partly".to_owned(), "holds".to_owned(), "broken".to_owned()),
            (
                "partly#2".to_owned(),
                "broken".to_owned(),
                "broken".to_owned()
            ),
        ],
        "each part carries the expectation, and one broken part meets it\n{run}"
    );
    let text = quint.earnest(&["verify"]);
    assert_eq!(
        text.detail("parts"),
        ["broken  partly  (expected)"],
        "{text}"
    );
    quint.mark("parts", &[("wholly", "@check broken")]);
    let held = quint.earnest(&["verify"]);
    held.exits(1);
    assert_eq!(
        held.detail("parts"),
        ["holds  wholly  a counterexample was expected"],
        "{held}"
    );
}
