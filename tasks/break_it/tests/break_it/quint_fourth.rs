#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Report, Sandbox, binary, rule};

const FAKE_QUINT: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
previous=""
for argument in "$@"; do
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
    cp "$here/typecheck.json" "$out"
    echo "{\"sourceIndex\":{\"0\":\"$folder/$(basename "$2")\"},\"map\":{}}" > "$map"
    ;;
  typecheck)
    cp "$here/typecheck.json" "$out"
    ;;
  compile)
    echo "Error: Internal error while flattening, as quint compile 0.33.1 stops on this file" >&2
    exit 1
    ;;
esac
"#;

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

struct Typechecked {
    folder: TempDir,
    quint: PathBuf,
}

impl Typechecked {
    fn new(file: &str, source: &str, typecheck: &str, checks: &str) -> Self {
        let folder = TempDir::new().expect("a temporary folder can be made");
        let fake = folder.path().join("fake");
        fs::create_dir_all(&fake).expect("the fake's folder can be made");
        let program = fake.join("quint");
        fs::write(&program, FAKE_QUINT).expect("the fake program can be written");
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the fake program can be made a program");
        fs::write(fake.join("typecheck.json"), typecheck).expect("a fixture can be written");
        fs::write(folder.path().join(file), source).expect("the spec can be written");
        fs::write(folder.path().join("earnest.quint.json"), checks)
            .expect("the checks file can be written");
        Self {
            folder,
            quint: program,
        }
    }

    fn verify(&self) -> Value {
        let output = Command::new(binary())
            .args(["verify", "--json", "--no-cache", "."])
            .env("EARNEST_QUINT", &self.quint)
            .current_dir(self.folder.path())
            .output()
            .expect("earnest starts");
        serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON")
    }

    fn path(&self) -> &Path {
        self.folder.path()
    }
}

fn model_named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == name))
        .unwrap_or(&Value::Null)
}

#[test]
fn fairness_of_an_action_the_step_guards_is_judged_by_the_action_alone_as_tlc_judges_it() {
    for (name, json, id) in [
        (
            "guarded_fair",
            include_str!("../../models/quint_fourth/guarded_fair/guarded_fair.json"),
            "onePays",
        ),
        (
            "guarded_step",
            include_str!("../../models/quint_fourth/guarded_fair/guarded_step.json"),
            "pays",
        ),
        (
            "guarded_branch",
            include_str!("../../models/quint_fourth/guarded_fair/guarded_branch.json"),
            "pays",
        ),
    ] {
        let report = compiled(name, json);
        if report.is_unreadable() {
            continue;
        }
        let model = report.json["models"][0].clone();
        assert_eq!(
            rule(&model, id)["verdict"],
            "holds",
            "`weakFair(pay…, vars)` is TLC's `WF_vars(pay…)`, whose `ENABLED` is that of the \
             action alone: `pay` can happen in the start state, where nothing is paid, whatever \
             the guard `open` the step puts before it. A run that stays there, or that only \
             toggles `open`, is not fair to it, so every fair run pays and `quint verify \
             --backend tlc --temporal {id}` finds no violation (`{name}`); the property holds, \
             or the file is refused naming the guard, never broken\n{report}"
        );
    }
}

#[test]
fn a_spec_that_imports_helpers_under_a_name_beside_their_types_module_is_built_from_the_typecheck()
{
    let checked = Typechecked::new(
        "named_helpers.qnt",
        include_str!("../../models/quint_fourth/named_helpers/named_helpers.qnt"),
        include_str!("../../models/quint_fourth/named_helpers/typecheck.json"),
        r#"{ "checks": [ { "file": "named_helpers.qnt", "main": "small",
            "invariants": ["inRange"], "runs": true } ] }"#,
    );
    let report = checked.verify();
    let model = model_named(&report, "small");
    assert!(
        model["states"] == 3
            && rule(model, "inRange")["verdict"] == "holds"
            && rule(model, "upTest")["verdict"] == "holds",
        "`spec` imports `types.*` and `helpers as H`, and `helpers` imports `types.*` too. The \
         docs read a spec that imports modules of definitions and types at any depth, a name \
         imported under a name as if written with `.*`, from the typecheck; `quint test \
         --backend typescript --main small` passes `small::spec::upTest`, and the same file \
         without the spec's own `import types.*` is read, 3 states. Earnest compiles it instead, \
         and `quint compile` stops with `Internal error while flattening`, so the check is \
         unreadable (in {})\n{report:#}",
        checked.path().display()
    );
}
