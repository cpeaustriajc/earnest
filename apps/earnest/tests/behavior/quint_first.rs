#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

use crate::support::{Run, Sandbox, model, repository};

const FAKE: &str = r##"#!/bin/sh
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
    sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" "$here/maps/$name.json" > "$map"
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
"##;

const FILES: [&str; 13] = [
    "Plain",
    "Other",
    "OwnRuns",
    "RunFromHelper",
    "TwoInstances",
    "OnlyRuns",
    "FirstSteps",
    "Unset",
    "RunsOnly",
    "HelperRun",
    "Spec",
    "Shapes",
    "Helpers",
];

const COMPILED: [(&str, &str); 15] = [
    ("Plain", "PlainNested"),
    ("RunFromHelper", "RunFromHelper"),
    ("TwoInstances", "TwoInstances"),
    ("FirstSteps", "FirstSteps"),
    ("Unset", "Unset"),
    ("RunsOnly", "RunsOnly"),
    ("HelperRun", "HelperRun"),
    ("Helpers", "Helpers"),
    ("Helpers", "StepIsChainOnward"),
    ("Shapes", "AsImport"),
    ("Shapes", "StarImport"),
    ("Shapes", "Several"),
    ("Shapes", "FromFile"),
    ("Shapes", "OwnVariables"),
    ("Shapes", "Mixed"),
];

const SHAPES: [(&str, &[&str]); 8] = [
    ("StarImport", &["starTest"]),
    ("AsImport", &["asTest"]),
    ("StarInstance", &["starInstanceTest"]),
    ("AsInstance", &["asInstanceTest"]),
    ("FromFile", &["fromFileTest"]),
    ("Several", &["aTest", "bTest", "fTest"]),
    ("OwnVariables", &["ownTest", "onlyOwnTest"]),
    ("Mixed", &["mixTest"]),
];

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
    fn new() -> Self {
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
            sandbox.write(
                &format!("fake/maps/{file}.json"),
                &first(&format!("fixtures/{file}.sourcemap.json")),
            );
        }
        for (file, main) in COMPILED {
            sandbox.write(
                &format!("fake/compiled/{main}.json"),
                &first(&format!("fixtures/{file}.{main}.json")),
            );
        }
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
fn the_owner_s_samples_pass_and_a_failing_test_run_fails_its_check_with_its_path() {
    let quint = Quint::new();
    let run = quint.earnest(&[
        "verify",
        "specs/RunFromHelper.qnt",
        "specs/TwoInstances.qnt",
        "specs/OnlyRuns.qnt",
    ]);
    run.exits(1);
    for (status, main) in [
        ("pass", "RunFromHelper"),
        ("pass", "TwoInstances"),
        ("fail", "OnlyRuns"),
    ] {
        assert_eq!(
            run.model_line(main)[..2],
            [status, main],
            "a check of runs only searches nothing\n{run}"
        );
        assert!(
            !run.model_line(main)[2].contains("states"),
            "a check of runs only prints no states field\n{run}"
        );
    }
    let detail = run.detail("OnlyRuns");
    assert!(
        detail
            .first()
            .is_some_and(|line| line.starts_with("broken  thirdOverTest")),
        "the failing run is named\n{run}"
    );
    assert!(
        detail.iter().any(|line| line == "start  Third::n = 10")
            && detail
                .iter()
                .any(|line| line == "why    step 4, `Third::step`, can't happen here"),
        "the path of the failing run is printed\n{run}"
    );
    assert_eq!(run.summary_line(), "3 models: 2 pass, 1 fail, 0 unfinished");
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
    let quint = Quint::new();
    let report = quint
        .earnest(&[
            "verify",
            "--json",
            "specs/RunFromHelper.qnt",
            "specs/TwoInstances.qnt",
            "specs/OnlyRuns.qnt",
            "specs/FirstSteps.qnt",
            "specs/Unset.qnt",
        ])
        .json();
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
            ("`x`".to_owned(), 12),
            ("`y`".to_owned(), 14),
            ("`y`".to_owned(), 15),
            ("`y`".to_owned(), 16),
        ],
        "each run that reads a variable its first action leaves unset\n{report}"
    );
}

fn held(ids: &[&str]) -> Vec<(String, String)> {
    pairs(&ids.iter().map(|id| (*id, "holds")).collect::<Vec<_>>())
}

#[test]
fn a_module_of_runs_only_on_a_plain_import_is_played_with_or_without_init_and_step_named() {
    let quint = Quint::new();
    let report = quint
        .earnest(&["verify", "--json", "specs/RunsOnly.qnt"])
        .json();
    for main in ["RunsOnly", "RunsOnlyNamed"] {
        assert_eq!(
            verdicts(&report, main),
            held(&["stepsOnceTest"]),
            "{main}\n{report}"
        );
    }
    assert!(
        quint.compiled().is_empty(),
        "a plain import is built from the typecheck: {:?}",
        quint.compiled()
    );
}

#[test]
fn a_module_of_runs_only_brings_in_what_its_runs_use_in_every_way_quint_allows() {
    let quint = Quint::new();
    let report = quint
        .earnest(&["verify", "--json", "specs/Shapes.qnt"])
        .json();
    assert_eq!(report["unreadable"], serde_json::json!([]), "{report}");
    for (main, runs) in SHAPES {
        assert_eq!(
            verdicts(&report, main),
            held(runs),
            "as `quint test --main {main}` judges them\n{report}"
        );
    }
    assert!(
        quint.compiled().is_empty(),
        "every shape is built from the typecheck: {:?}",
        quint.compiled()
    );
}

#[test]
fn a_definition_that_chains_steps_is_played_where_a_run_calls_it() {
    let quint = Quint::new();
    let report = quint
        .earnest(&[
            "verify",
            "--json",
            "specs/HelperRun.qnt",
            "specs/Helpers.qnt",
        ])
        .json();
    assert_eq!(verdicts(&report, "HelperRun"), held(&["twoStepsTest"]));
    assert_eq!(
        verdicts(&report, "Helpers"),
        pairs(&[
            ("twoStepsTest", "holds"),
            ("argumentsTest", "holds"),
            ("startTest", "holds"),
            ("middleTest", "holds"),
            ("nestedTest", "holds"),
            ("repsTest", "holds"),
            ("paramRepsTest", "holds"),
            ("runParamTest", "holds"),
            ("failTest", "holds"),
            ("overTest", "broken"),
            ("nondetArgTest", "holds"),
            ("wrongTest", "broken"),
        ]),
        "as `quint test --main Helpers` judges them\n{report}"
    );
    let refused: Vec<&str> = report["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|problem| problem["message"].as_str())
        .collect();
    assert!(
        matches!(refused.as_slice(), [only] if only.contains("make a run, not one step")),
        "a chain of steps as the search's `step` is refused: {refused:?}"
    );
}

#[test]
fn a_module_of_runs_only_with_variables_of_its_own_is_built_from_the_typecheck() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "specs/OwnRuns.qnt"]);
    run.exits(1);
    assert_eq!(run.model_line("OwnRuns")[..3], ["fail", "OwnRuns", "0 ms"]);
    let report = quint
        .earnest(&["verify", "--json", "specs/OwnRuns.qnt"])
        .json();
    assert_eq!(
        verdicts(&report, "OwnRuns"),
        pairs(&[
            ("seededTest", "holds"),
            ("closedTest", "holds"),
            ("wrongTest", "broken"),
        ]),
        "as `quint test --main OwnRuns` judges them\n{report}"
    );
    assert_eq!(
        run.detail("OwnRuns"),
        [
            "broken  wrongTest",
            "start  open = true, count = 0",
            "1. bump  count: 0 -> 1",
            "end    open = true, count = 1",
            "why    what the run expects after step 1 is false here",
        ]
    );
    assert!(
        quint.compiled().is_empty(),
        "nothing to compile: {:?}",
        quint.compiled()
    );
}

#[test]
fn a_module_of_runs_only_plays_the_runs_of_a_module_it_imports_plainly() {
    let quint = Quint::new();
    let report = quint
        .earnest(&["verify", "--json", "specs/Plain.qnt"])
        .json();
    let spec = [("specTest", "holds"), ("specOverTest", "broken")];
    for main in ["PlainAs", "PlainStar"] {
        assert_eq!(
            verdicts(&report, main),
            pairs(&[spec[0], spec[1], ("ownTest", "holds")]),
            "as `quint test --main {main}` judges them, `Main::specTest` and `specTest` among them\n{report}"
        );
    }
    assert_eq!(
        verdicts(&report, "PlainFrom"),
        pairs(&[
            spec[0],
            spec[1],
            ("bothTest", "holds"),
            ("otherTest", "holds")
        ]),
        "`O::otherTest` of another file is played too\n{report}"
    );
    assert_eq!(
        verdicts(&report, "PlainNested"),
        held(&["nestedTest"]),
        "a spec that imports a module of helpers is built from the typecheck\n{report}"
    );
    assert!(
        quint.compiled().is_empty(),
        "nothing is compiled: {:?}",
        quint.compiled()
    );
}
