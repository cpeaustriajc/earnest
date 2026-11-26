#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

use crate::support::{Run, Sandbox, model, repository, rule};

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
    if [ -f "$here/parsed/$name.json" ]; then
      cp "$here/parsed/$name.json" "$out"
    else
      echo '{"modules":[]}' > "$out"
    fi
    echo "{\"sourceIndex\":{\"0\":\"$folder/$name.qnt\"},\"map\":{}}" > "$map"
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
"#;

const SPECS: [&str; 11] = [
    "ints",
    "kinds",
    "pure",
    "own",
    "shadow",
    "runs",
    "clock",
    "clockbits",
    "lamp",
    "more",
    "compiled",
];

const COMPILED: [&str; 24] = [
    "ints.Low",
    "ints.High",
    "kinds.Setup",
    "pure.Four",
    "own.Tight",
    "shadow.S",
    "runs.Three",
    "runs.One",
    "clock.Small",
    "clock.Large",
    "clockbits.SmallBits",
    "lamp.Dim",
    "lamp.Bright",
    "lamp.Blinding",
    "more.Named",
    "more.Reexported",
    "more.ByOwn",
    "more.ByName",
    "more.Two",
    "more.Pair",
    "more.Plain",
    "compiled.Nested",
    "compiled.Renamed",
    "compiled.Single",
];

const SAME_AS: [(&str, &str); 1] = [("Exported", "Plain")];

const FALLBACKS: [&str; 2] = ["Nested", "Single"];

fn setups(name: &str) -> String {
    let path = repository().join("tasks/quint/setups").join(name);
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
        for spec in SPECS {
            sandbox.write(
                &format!("specs/{spec}.qnt"),
                &setups(&format!("{spec}.qnt")),
            );
            for kind in ["parsed", "typechecked"] {
                sandbox.write(
                    &format!("fake/{kind}/{spec}.json"),
                    &setups(&format!("fixtures/{spec}.typecheck.json")),
                );
            }
        }
        for compiled in COMPILED {
            let main = compiled.split_once('.').map_or(compiled, |(_, main)| main);
            sandbox.write(
                &format!("fake/compiled/{main}.json"),
                &setups(&format!("fixtures/{compiled}.json")),
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

    fn calls(&self) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn of_kind(&self, kind: &str) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|call| call.split(' ').next() == Some(kind))
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

fn names(set: &[&str]) -> BTreeSet<String> {
    set.iter().map(|each| (*each).to_owned()).collect()
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

#[test]
fn every_setup_built_from_one_typecheck_checks_as_the_model_quint_compile_flattens() {
    let quint = Quint::new();
    let ours = quint.earnest(&["verify", "--json", "specs"]).json();
    assert_eq!(
        quint.compiled_mains(),
        names(&FALLBACKS),
        "only the setups of other shapes are compiled: {:?}",
        quint.calls()
    );
    assert_eq!(
        quint.of_kind("typecheck").len(),
        SPECS.len(),
        "one typecheck per file: {:?}",
        quint.calls()
    );
    quint.forget();
    quint.without_typecheck();
    let theirs = quint.earnest(&["verify", "--json", "specs"]).json();
    assert_eq!(
        quint.compiled_mains().len(),
        COMPILED.len() + SAME_AS.len(),
        "with no typecheck, every setup is compiled: {:?}",
        quint.calls()
    );
    let mut compared = 0;
    let mains = COMPILED
        .iter()
        .map(|compiled| compiled.split_once('.').map_or(*compiled, |(_, main)| main))
        .map(|main| (main, main))
        .chain(SAME_AS);
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
    assert_eq!(
        compared,
        COMPILED.len() + SAME_AS.len(),
        "every setup is compared\n{theirs}"
    );
}

#[test]
fn the_runs_of_the_spec_module_are_played_in_a_setup_built_from_the_typecheck() {
    let quint = Quint::new();
    let report = quint.earnest(&["verify", "--json", "specs"]).json();
    let verdicts = |main: &str| -> Vec<(String, String)> {
        let found = model(&report, main);
        found["rules"]
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
    };
    let pairs = |list: &[(&str, &str)]| -> Vec<(String, String)> {
        list.iter()
            .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
            .collect()
    };
    assert_eq!(
        verdicts("Small"),
        pairs(&[
            ("tickTwice", "holds"),
            ("tickMany", "holds"),
            ("overTick", "broken"),
            ("hearAny", "holds"),
        ]),
        "as `quint test --main Small` judges them\n{report}"
    );
    assert_eq!(
        verdicts("Three"),
        pairs(&[
            ("twoTest", "holds"),
            ("allTest", "holds"),
            ("overTest", "broken"),
            ("ownTest", "holds"),
        ]),
        "the spec's runs ending in `Test` and the setup's own\n{report}"
    );
    assert_eq!(
        verdicts("One"),
        pairs(&[
            ("twoTest", "broken"),
            ("allTest", "holds"),
            ("overTest", "broken"),
        ]),
        "{report}"
    );
    for (main, runs) in [
        ("Named", &["upTest", "fullTest"][..]),
        ("Reexported", &["upTest", "fullTest", "ownTest"][..]),
        ("ByOwn", &["upTest", "fullTest"][..]),
        ("ByName", &["upTest", "fullTest"][..]),
        ("Exported", &["upTest", "fullTest"][..]),
        ("Renamed", &["upTest"][..]),
    ] {
        let holding: Vec<(&str, &str)> = runs.iter().map(|run| (*run, "holds")).collect();
        assert_eq!(
            verdicts(main),
            pairs(&holding),
            "as `quint test --main {main}` judges them\n{report}"
        );
    }
    let problem = |feature: &str, file: &str| -> String {
        report["unreadable"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|problem| problem["feature"] == feature && problem["file"] == file)
            .and_then(|problem| problem["message"].as_str())
            .unwrap_or_else(|| panic!("{feature} in {file} is unreadable\n{report}"))
            .to_owned()
    };
    assert_eq!(
        model(&report, "Single")["runsNotPlayed"],
        serde_json::json!(["Single::Base::upTest"]),
        "a compiled setup can't play an imported run, and says so\n{report}"
    );
    assert_eq!(
        verdicts("Pair"),
        pairs(&[
            ("P::upTest", "holds"),
            ("P::fullTest", "holds"),
            ("R::upTest", "holds"),
            ("R::fullTest", "holds")
        ]),
        "a run of one of two instances is played over its own instance's variable, as `quint \
         test --main Pair` judges it\n{report}"
    );
}

#[test]
fn seventeen_setups_of_one_file_start_quint_three_times_and_a_second_run_never() {
    let quint = Quint::new();
    quint.sandbox.write("specs/many.qnt", &setups("many.qnt"));
    for kind in ["parsed", "typechecked"] {
        quint.sandbox.write(
            &format!("fake/{kind}/many.json"),
            &setups("fixtures/many.typecheck.json"),
        );
    }
    let run = quint.earnest(&["verify", "specs/many.qnt"]);
    run.exits(0);
    let kinds: Vec<String> = quint
        .calls()
        .iter()
        .filter_map(|call| call.split(' ').next().map(str::to_owned))
        .collect();
    assert_eq!(
        kinds,
        ["--version", "parse", "typecheck"],
        "no compile at all\n{run}"
    );
    assert_eq!(
        run.summary_line(),
        "17 models: 17 pass, 0 fail, 0 unfinished",
        "{run}"
    );
    assert_eq!(run.model_line("S9")[2], "20 states", "{run}");
    quint.sandbox.write("fake/calls", "");
    let again = quint.earnest(&["verify", "specs/many.qnt"]);
    again.exits(0);
    assert_eq!(quint.calls(), Vec::<String>::new(), "{again}");
}

#[test]
fn a_setup_of_another_shape_is_compiled_and_a_second_run_compiles_nothing() {
    let quint = Quint::new();
    let files = ["verify", "specs/clock.qnt", "specs/compiled.qnt"];
    let run = quint.earnest(&files);
    run.exits(1);
    assert_eq!(
        quint.compiled_mains(),
        names(&FALLBACKS),
        "an instance of an instance and a name imported alone are compiled, `Small` and `Large` \
         are built\n{run}"
    );
    assert_eq!(quint.of_kind("typecheck").len(), 2, "{:?}", quint.calls());
    quint.sandbox.write("fake/calls", "");
    let again = quint.earnest(&files);
    again.exits(1);
    assert_eq!(quint.calls(), Vec::<String>::new(), "{again}");
}

#[test]
fn a_failed_typecheck_falls_back_to_compiling_each_setup() {
    let quint = Quint::new();
    let mut failed: Value = serde_json::from_str(&setups("fixtures/lamp.typecheck.json"))
        .expect("the typecheck fixture is JSON");
    failed["errors"] = serde_json::json!([{ "code": "QNT000" }]);
    quint
        .sandbox
        .write("fake/typechecked/lamp.json", &failed.to_string());
    let run = quint.earnest(&["verify", "specs/lamp.qnt"]);
    run.exits(1);
    assert_eq!(
        quint.compiled_mains(),
        names(&["Blinding", "Bright", "Dim"]),
        "{run}"
    );
}
