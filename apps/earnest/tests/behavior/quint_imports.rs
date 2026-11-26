#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

use crate::support::{Run, Sandbox, model, repository, rule};

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
    if [ ! -f "$here/parsed/$name.json" ] || [ ! -f "$here/maps/$name.json" ]; then
      echo "error: no parse of $name.qnt is recorded" >&2
      exit 1
    fi
    cp "$here/parsed/$name.json" "$out"
    sed "s#\"\([0-9]*\)\":\"\([^\"/]*\.qnt\)\"#\"\1\":\"$folder/\2\"#g" "$here/maps/$name.json" > "$map"
    ;;
  typecheck)
    name="$(basename "$2" .qnt)"
    if [ ! -f "$here/typechecked/$name.json" ]; then
      echo "error: typechecking failed" >&2
      exit 1
    fi
    cp "$here/typechecked/$name.json" "$out"
    ;;
  compile)
    if [ ! -f "$here/compiled/$main.json" ]; then
      echo "error: module $main does not exist" >&2
      exit 1
    fi
    cat "$here/compiled/$main.json"
    ;;
esac
"##;

const SOURCES: [&str; 6] = ["types", "lib", "doors", "setups", "star", "as"];

const TYPECHECKED: [&str; 3] = ["setups", "star", "as"];

const COMPILED: [&str; 5] = [
    "setups.two",
    "setups.three",
    "setups.named",
    "star.Star",
    "star.Both",
];

const SAME_AS: [(&str, &str); 1] = [("As", "Star")];

fn imports(name: &str) -> String {
    let path = repository().join("tasks/quint/imports").join(name);
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
        for source in SOURCES {
            sandbox.write(
                &format!("specs/{source}.qnt"),
                &imports(&format!("{source}.qnt")),
            );
        }
        for file in TYPECHECKED {
            Self::typechecked(&sandbox, file);
        }
        for compiled in COMPILED {
            let main = compiled.split_once('.').map_or(compiled, |(_, main)| main);
            sandbox.write(
                &format!("fake/compiled/{main}.json"),
                &imports(&format!("fixtures/{compiled}.json")),
            );
        }
        sandbox.write("fake/calls", "");
        Self { sandbox, program }
    }

    fn typechecked(sandbox: &Sandbox, file: &str) {
        for kind in ["parsed", "typechecked"] {
            sandbox.write(
                &format!("fake/{kind}/{file}.json"),
                &imports(&format!("fixtures/{file}.typecheck.json")),
            );
        }
        sandbox.write(
            &format!("fake/maps/{file}.json"),
            &imports(&format!("fixtures/{file}.sourcemap.json")),
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

    fn verify(&self) -> Value {
        self.earnest(&["verify", "--json", "specs"]).json()
    }

    fn of_kind(&self, kind: &str) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .filter(|call| call.split(' ').next() == Some(kind))
            .map(str::to_owned)
            .collect()
    }

    fn compiled_mains(&self) -> BTreeSet<String> {
        self.of_kind("compile")
            .iter()
            .filter_map(|call| {
                let mut words = call.split(' ');
                words.find(|word| *word == "--main")?;
                words.next().map(str::to_owned)
            })
            .collect()
    }

    fn forget(&self) {
        self.sandbox.write("fake/calls", "");
        fs::remove_dir_all(self.sandbox.path().join(".earnest")).unwrap_or_default();
    }

    fn without_typecheck(&self) {
        fs::remove_dir_all(self.sandbox.path().join("fake/typechecked"))
            .expect("the typechecked fixtures go");
    }
}

fn rule_ids(model: &Value) -> Vec<String> {
    model["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|rule| rule["kind"] != "witness")
        .filter_map(|rule| rule["id"].as_str().map(str::to_owned))
        .collect()
}

fn verdicts(report: &Value, main: &str) -> Vec<(String, String)> {
    model(report, main)["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|rule| {
            rule["says"]
                .as_str()
                .is_some_and(|says| says.contains("run"))
        })
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
fn a_setup_whose_spec_imports_other_modules_checks_as_the_model_quint_compile_flattens() {
    let quint = Quint::new();
    let ours = quint.verify();
    assert_eq!(
        quint.compiled_mains(),
        BTreeSet::new(),
        "every setup is built from the typecheck"
    );
    assert_eq!(
        quint.of_kind("typecheck").len(),
        TYPECHECKED.len(),
        "one typecheck per file"
    );
    assert_eq!(
        quint.of_kind("parse").len(),
        SOURCES.len(),
        "one parse per `.qnt` file, each read for its marks"
    );
    quint.forget();
    quint.without_typecheck();
    let theirs = quint.verify();
    let mains = COMPILED
        .iter()
        .map(|compiled| compiled.split_once('.').map_or(*compiled, |(_, main)| main))
        .map(|main| (main, main))
        .chain(SAME_AS);
    let mut compared = 0;
    for (main, flattened_as) in mains {
        let flattened = model(&theirs, flattened_as);
        let built = model(&ours, main);
        for key in ["states", "reason"] {
            assert_eq!(
                built[key], flattened[key],
                "{main}: {key}\n{built}\n{flattened}"
            );
        }
        assert_eq!(rule_ids(built), rule_ids(flattened), "{main}");
        for id in rule_ids(flattened) {
            assert_eq!(
                rule(built, &id),
                rule(flattened, &id),
                "{main}: the rule {id}, its verdict and its path"
            );
        }
        compared += 1;
    }
    assert_eq!(compared, COMPILED.len() + SAME_AS.len());
    for (main, states) in [
        ("two", 57),
        ("three", 764),
        ("named", 25),
        ("Star", 16),
        ("Both", 40),
        ("As", 16),
    ] {
        assert_eq!(
            model(&ours, main)["states"],
            states,
            "{main}: as `quint verify --backend tlc` counts them\n{ours}"
        );
    }
}

#[test]
fn the_runs_of_a_spec_that_imports_other_modules_are_played_as_quint_test_plays_them() {
    let quint = Quint::new();
    let report = quint.verify();
    assert_eq!(quint.compiled_mains(), BTreeSet::new(), "{report}");
    let spec = [
        ("openTest", "holds"),
        ("pingTest", "holds"),
        ("overTest", "broken"),
    ];
    for (main, own) in [
        ("two", &[("bothOpenTest", "holds")][..]),
        ("three", &[][..]),
        (
            "scenarios",
            &[("sendTest", "holds"), ("closeTest", "holds")][..],
        ),
    ] {
        let mut expected = own.to_vec();
        expected.extend(spec);
        assert_eq!(
            verdicts(&report, main),
            pairs(&expected),
            "as `quint test --main {main}` judges them\n{report}"
        );
        assert!(
            model(&report, main).get("runsNotPlayed").is_none(),
            "{main}: every run is played\n{report}"
        );
    }
    assert_eq!(
        verdicts(&report, "named"),
        pairs(&[
            ("ownTest", "holds"),
            ("pingTest", "holds"),
            ("openTest", "holds"),
            ("D::pingTest", "holds"),
            ("overTest", "broken"),
        ]),
        "the setup's own `pingTest` keeps its name, the spec's is told apart by its instance, as \
         `quint test` names it `named::D::pingTest`\n{report}"
    );
    let star = [("upTest", "holds"), ("downTest", "holds")];
    assert_eq!(verdicts(&report, "Star"), pairs(&star), "{report}");
    assert_eq!(verdicts(&report, "As"), pairs(&star), "{report}");
    assert_eq!(
        verdicts(&report, "Both"),
        pairs(&[
            ("A::upTest", "holds"),
            ("downTest", "holds"),
            ("B::upTest", "holds"),
            ("bothTest", "holds"),
        ]),
        "two specs' runs of one name are told apart by their instance, a name only one has \
         stays short\n{report}"
    );
}

#[test]
fn a_false_assumption_stops_a_module_checked_directly_and_a_setup_alike() {
    let quint = Quint::new();
    quint
        .sandbox
        .write("specs/assume.qnt", &imports("assume.qnt"));
    Quint::typechecked(&quint.sandbox, "assume");
    quint.sandbox.write(
        "fake/compiled/Direct.json",
        &imports("fixtures/assume.Direct.json"),
    );
    let report = quint
        .earnest(&["verify", "--json", "specs/assume.qnt"])
        .json();
    assert_eq!(
        quint.compiled_mains(),
        ["Direct".to_owned()].into_iter().collect::<BTreeSet<_>>(),
        "a module with no instance is compiled, the setups are built\n{report}"
    );
    let unreadable: Vec<(String, String)> = report["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|problem| {
            (
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(unreadable.len(), 2, "`Direct` and `Bad`\n{report}");
    for (feature, message) in &unreadable {
        assert_eq!(feature, "`assume Positive`", "{report}");
        assert!(
            message.starts_with("the assumption `Positive` is false, so no state is checked"),
            "{message}"
        );
    }
    let good = model(&report, "Good");
    assert_eq!(good["status"], "pass", "{report}");
    assert_eq!(
        good["states"], 3,
        "as `quint verify --backend tlc --main Good`\n{report}"
    );
    assert_eq!(
        verdicts(&report, "Good"),
        pairs(&[("upTest", "holds")]),
        "{report}"
    );
}

#[test]
fn a_step_of_a_quint_path_prints_only_the_key_field_or_item_it_changes() {
    let quint = Quint::new();
    quint
        .sandbox
        .write("specs/printing.qnt", &imports("printing.qnt"));
    Quint::typechecked(&quint.sandbox, "printing");
    for main in ["Printing", "fault"] {
        quint.sandbox.write(
            &format!("fake/compiled/{main}.json"),
            &imports(&format!("fixtures/printing.{main}.json")),
        );
    }
    let run = quint.earnest(&["verify", "specs/printing.qnt"]);
    assert_eq!(
        run.detail("Printing"),
        [
            "broken  annBelowFive  \"The Quint invariant `annBelowFive` holds\"",
            "start  queue = [1, 2, 3, 4, 5], balances = Map(\"ann\" -> 3, \"bob\" -> 5, \"cy\" -> 0, \
             \"dee\" -> 7, \"eve\" -> 1), seen = Set(1, 3, 5, 7)",
            "1. step  queue: + 3 at the end, balances.get(\"ann\"): 3 -> 4",
            "2. step  queue: + 4 at the end, balances.get(\"ann\"): 4 -> 5, seen: + 4",
            "end    queue = [1, 2, 3, 4, 5, 3, 4], balances = Map(\"ann\" -> 5, \"bob\" -> 5, \
             \"cy\" -> 0, \"dee\" -> 7, \"eve\" -> 1), seen = Set(1, 3, 4, 5, 7)",
            "broken  noStartTest  \"The Quint run `noStartTest` passes\"",
            "start  no state: the first action gives none",
            "why    the run's first action can't happen: it gives no state to start from",
        ],
        "the whole values on the `start` and `end` lines, the change alone on a step's\n{run}"
    );
    let report = quint
        .earnest(&["verify", "--json", "specs/printing.qnt"])
        .json();
    let step = &model(&report, "Printing")["rules"][0]["path"]["steps"][0]["changes"][1];
    assert_eq!(step["field"], "balances", "{report}");
    assert_eq!(
        step["after"],
        serde_json::json!([["ann", 4], ["bob", 5], ["cy", 0], ["dee", 7], ["eve", 1]]),
        "the JSON keeps the whole value\n{report}"
    );
    let problem = report["unreadable"][0]["message"]
        .as_str()
        .unwrap_or_default();
    assert!(
        problem.starts_with("a value `P::seed` gives takes a division"),
        "a fault in a run's first action names that action, not `init`\n{report}"
    );
}
