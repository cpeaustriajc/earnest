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
    sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" "$here/maps/$name.json" > "$map"
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

const SMALL_RUNS: &str = ", tickTwice, tickMany, overTick, hearAny";

const ILL_TYPED: (&str, &str) = ("val levelOk = level <= 6", "val levelOk = level <= true");

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
            sandbox.write(
                &format!("fake/maps/{spec}.json"),
                &setups(&format!("fixtures/{spec}.sourcemap.json")),
            );
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

    fn flattened_by_quint(&self) -> Value {
        let mut arguments: Vec<String> = vec!["verify".to_owned(), "--json".to_owned()];
        arguments.extend(COMPILED.iter().map(|compiled| {
            let main = compiled.split_once('.').map_or(*compiled, |(_, main)| main);
            format!("fake/compiled/{main}.json")
        }));
        let borrowed: Vec<&str> = arguments.iter().map(String::as_str).collect();
        self.earnest(&borrowed).json()
    }

    fn without_named_runs(&self) {
        let blank = " ".repeat(SMALL_RUNS.len());
        for file in [
            "specs/clock.qnt",
            "fake/parsed/clock.json",
            "fake/typechecked/clock.json",
        ] {
            let text = self.sandbox.read(file);
            assert!(text.contains(SMALL_RUNS), "{file} names the runs of Small");
            self.sandbox.write(file, &text.replace(SMALL_RUNS, &blank));
        }
    }
}

fn unplayed(report: &Value) -> BTreeSet<String> {
    report["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|each| each.get("runsNotPlayed").is_some())
        .filter_map(|each| each["name"].as_str().map(str::to_owned))
        .collect()
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
    quint.without_named_runs();
    let ours = quint.earnest(&["verify", "--json", "specs"]).json();
    assert!(
        quint.calls().is_empty(),
        "no `quint` is started: {:?}",
        quint.calls()
    );
    assert_eq!(
        unplayed(&ours),
        names(&["Single"]),
        "of the setups of other shapes, which are compiled, only `Single` sees a run of its spec, \
         which it can't play; every setup built from the typecheck plays them\n{ours}"
    );
    let theirs = quint.flattened_by_quint();
    let mut compared = 0;
    let mains = COMPILED
        .iter()
        .map(|compiled| compiled.split_once('.').map_or(*compiled, |(_, main)| main))
        .map(|main| (main, main))
        .chain(SAME_AS);
    for (main, flattened_as) in mains {
        let flattened = model(&theirs, flattened_as);
        let built = model(&ours, main);
        assert_eq!(
            built["states"], flattened["states"],
            "{main}: states\n{built}\n{flattened}"
        );
        assert_eq!(
            search_reason(built),
            search_reason(flattened),
            "{main}: the reason the search gives\n{built}\n{flattened}"
        );
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
fn seventeen_setups_of_one_file_start_no_quint_and_a_second_run_comes_from_the_cache() {
    let quint = Quint::new();
    quint.sandbox.write("specs/many.qnt", &setups("many.qnt"));
    for kind in ["parsed", "typechecked"] {
        quint.sandbox.write(
            &format!("fake/{kind}/many.json"),
            &setups("fixtures/many.typecheck.json"),
        );
    }
    quint.sandbox.write(
        "fake/maps/many.json",
        &setups("fixtures/many.sourcemap.json"),
    );
    let run = quint.earnest(&["verify", "specs/many.qnt"]);
    run.exits(0);
    assert_eq!(
        quint.calls(),
        Vec::<String>::new(),
        "no `quint` at all\n{run}"
    );
    assert_eq!(
        run.summary_line(),
        "17 models: 17 pass, 0 fail, 0 unfinished",
        "{run}"
    );
    assert_eq!(run.model_line("S9")[2], "20 states", "{run}");
    let again = quint.earnest(&["verify", "specs/many.qnt"]);
    again.exits(0);
    assert!(
        again
            .model_lines()
            .iter()
            .all(|line| line.contains(&"(cached)".to_owned())),
        "{again}"
    );
}

#[test]
fn a_setup_of_another_shape_is_compiled_and_a_second_run_comes_from_the_cache() {
    let quint = Quint::new();
    let files = ["verify", "specs/clock.qnt", "specs/compiled.qnt"];
    let run = quint.earnest(&files);
    run.exits(1);
    let json: Vec<&str> = files.iter().copied().chain(["--json"]).collect();
    assert_eq!(
        unplayed(&quint.earnest(&json).json()),
        names(&["Single"]),
        "a name imported alone is compiled and can't play the spec's run; `Small` and `Large` \
         are built and play theirs\n{run}"
    );
    let again = quint.earnest(&files);
    again.exits(1);
    assert!(
        again
            .model_lines()
            .iter()
            .all(|line| line.contains(&"(cached)".to_owned())),
        "{again}"
    );
    assert_eq!(quint.calls(), Vec::<String>::new(), "{again}");
}

#[test]
fn a_failed_typecheck_falls_back_to_compiling_each_setup() {
    let quint = Quint::new();
    let lamp = quint.sandbox.read("specs/lamp.qnt");
    assert!(lamp.contains(ILL_TYPED.0), "{lamp}");
    quint.sandbox.write(
        "specs/lamp.qnt",
        &lamp.replacen(ILL_TYPED.0, ILL_TYPED.1, 1),
    );
    let run = quint.earnest(&["verify", "--json", "specs/lamp.qnt"]);
    run.exits(1);
    let compiled: BTreeSet<String> = run.json()["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|problem| {
            let feature = problem["feature"].as_str()?;
            let main = feature.strip_prefix("compiling `")?;
            Some(main.strip_suffix("` as Quint")?.to_owned())
        })
        .collect();
    assert_eq!(
        compiled,
        names(&["Blinding", "Bright", "Dim"]),
        "each setup is compiled, and refused with the compile's error\n{run}"
    );
}

fn search_reason(model: &Value) -> Value {
    let unplayed: Vec<&str> = model["runsNotPlayed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if unplayed.is_empty() {
        return model["reason"].clone();
    }
    assert_ne!(
        model["status"], "pass",
        "a compiled check that did not play every run it sees does not pass: {model}"
    );
    assert!(
        model["reason"].as_str().is_some_and(|reason| {
            reason.starts_with(&format!("runs not played: {}", unplayed.join(", ")))
        }),
        "the reason names the runs not played: {model}"
    );
    Value::Null
}
