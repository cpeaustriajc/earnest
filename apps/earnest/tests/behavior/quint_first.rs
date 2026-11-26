#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

use crate::support::{Run, Sandbox, model, repository};

const FAKE: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
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
    name="$(basename "$2" .qnt)"
    folder="$(cd "$(dirname "$2")" && pwd)"
    cp "$here/typechecked/$name.json" "$out"
    echo "{\"sourceIndex\":{\"0\":\"$folder/$name.qnt\"},\"map\":{}}" > "$map"
    ;;
  typecheck)
    name="$(basename "$2" .qnt)"
    cp "$here/typechecked/$name.json" "$out"
    ;;
  compile)
    if [ ! -f "$here/compiled/$main.json" ]; then
      echo "error: [QNT404] Name 'init' not found" >&2
      exit 1
    fi
    cat "$here/compiled/$main.json"
    ;;
esac
"#;

const FILES: [&str; 5] = [
    "RunFromHelper",
    "TwoInstances",
    "OnlyRuns",
    "FirstSteps",
    "Unset",
];

const COMPILED: [&str; 4] = ["RunFromHelper", "TwoInstances", "FirstSteps", "Unset"];

fn first(name: &str) -> String {
    let path = repository().join("tasks/quint/first").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

struct Quint {
    sandbox: Sandbox,
    program: PathBuf,
}

impl Quint {
    fn new(checks: &str) -> Self {
        let sandbox = Sandbox::empty();
        let program = sandbox.path().join("fake/quint");
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the fake can be made a program");
        for file in FILES {
            sandbox.write(&format!("specs/{file}.qnt"), &first(&format!("{file}.qnt")));
            sandbox.write(
                &format!("fake/typechecked/{file}.json"),
                &first(&format!("fixtures/{file}.typecheck.json")),
            );
        }
        for main in COMPILED {
            sandbox.write(
                &format!("fake/compiled/{main}.json"),
                &first(&format!("fixtures/{main}.{main}.json")),
            );
        }
        sandbox.write("specs/earnest.quint.json", checks);
        sandbox.write("fake/calls", "");
        Self { sandbox, program }
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

    fn compiled(&self) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .filter(|call| call.starts_with("compile"))
            .map(str::to_owned)
            .collect()
    }
}

fn verdicts(report: &Value, main: &str) -> Vec<(String, String)> {
    model(report, main)["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|rule| rule["kind"] == "witness")
        .map(|rule| {
            (
                rule["id"].as_str().unwrap_or_default().to_owned(),
                rule["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

#[test]
fn the_owner_s_three_samples_each_print_one_pass_line() {
    let quint = Quint::new(
        r#"{ "checks": [
            { "file": "RunFromHelper.qnt", "main": "RunFromHelper", "invariants": [], "temporal": [], "runs": true },
            { "file": "TwoInstances.qnt", "main": "TwoInstances", "invariants": [], "temporal": [], "runs": true },
            { "file": "OnlyRuns.qnt", "main": "OnlyRuns", "runs": ["fixedTest", "listTest", "thirdTest", "mixTest"] }
        ] }"#,
    );
    let run = quint.earnest(&["verify", "specs"]);
    run.exits(0);
    for main in ["RunFromHelper", "TwoInstances", "OnlyRuns"] {
        assert_eq!(
            run.model_line(main)[..3],
            ["pass", main, "0 states"],
            "a check of runs only searches nothing\n{run}"
        );
    }
    assert_eq!(run.summary_line(), "3 models: 3 pass, 0 fail, 0 unfinished");
    assert!(
        !quint
            .compiled()
            .iter()
            .any(|call| call.contains("--main OnlyRuns")),
        "the module of runs only is built from the typecheck, and `quint` is never asked for \
         its `init`: {:?}",
        quint.compiled()
    );
}

#[test]
fn every_run_is_judged_as_quint_test_judges_it_and_the_search_has_only_its_own_variables() {
    let quint = Quint::new(&first("earnest.quint.json"));
    let report = quint.earnest(&["verify", "--json", "specs"]).json();
    let held = |ids: &[&str]| -> Vec<(String, String)> {
        pairs(&ids.iter().map(|id| (*id, "holds")).collect::<Vec<_>>())
    };
    assert_eq!(
        verdicts(&report, "RunFromHelper"),
        held(&["fromHelperTest"])
    );
    assert_eq!(
        verdicts(&report, "TwoInstances"),
        held(&["beforeStartsAtFiveTest", "fixedStartsAtZeroTest"])
    );
    assert_eq!(
        verdicts(&report, "OnlyRuns"),
        pairs(&[
            ("fixedTest", "holds"),
            ("listTest", "holds"),
            ("thirdTest", "holds"),
            ("thirdOverTest", "broken"),
            ("mixTest", "holds"),
        ])
    );
    assert_eq!(
        verdicts(&report, "FirstSteps"),
        pairs(&[
            ("seedTest", "holds"),
            ("inPlaceTest", "holds"),
            ("pickTest", "holds"),
            ("pickOverTest", "broken"),
            ("nondetRunTest", "holds"),
            ("ownInitTest", "holds"),
            ("otherTest", "holds"),
            ("otherOverTest", "broken"),
            ("noStartTest", "broken"),
            ("noStartFailTest", "holds"),
        ]),
        "as `quint test --main FirstSteps` judges them"
    );
    let steps = model(&report, "FirstSteps");
    assert_eq!(steps["states"], 4, "`Other::n` is no field of the search");
    let broken = |id: &str| -> Value {
        steps["rules"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|rule| rule["id"] == id)
            .cloned()
            .unwrap_or_default()
    };
    let over = broken("pickOverTest");
    assert_eq!(
        over["path"]["start"],
        serde_json::json!([{ "name": "Other::n", "value": 3 }]),
        "the path starts where the first action put the run's own variable\n{over}"
    );
    assert_eq!(
        over["reason"],
        "what the run expects after step 1 is false here"
    );
    assert_eq!(
        broken("noStartTest")["reason"],
        "the run's first action can't happen: it gives no state to start from"
    );
    let unset: Vec<(String, u64)> = report["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|problem| problem["file"] == "specs/Unset.qnt")
        .map(|problem| {
            (
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["line"].as_u64().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        unset,
        [
            ("`x`".to_owned(), 1),
            ("`y`".to_owned(), 1),
            ("`y`".to_owned(), 1),
            ("`y`".to_owned(), 1),
        ],
        "each run that reads a variable its first action leaves unset\n{report}"
    );
}
