use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Sandbox, is_broken_after, rule};

const FILE: &str = "case.model.ts";

fn binary() -> PathBuf {
    Sandbox::with(&[]).verify(&[]);
    let test = env::current_exe().expect("a test knows where its own binary is");
    let folder = test
        .ancestors()
        .find(|folder| {
            folder
                .file_name()
                .is_some_and(|name| name == "deps" || name == "build")
        })
        .and_then(Path::parent)
        .expect("a test binary sits in `deps` or `build` under the folder of its build profile");
    folder.join(format!("earnest{}", env::consts::EXE_SUFFIX))
}

struct Ran {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

impl Ran {
    fn json(&self) -> Value {
        serde_json::from_str(&self.stdout).unwrap_or(Value::Null)
    }

    fn said(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

fn run_in_folder(files: &[(&str, &str)], arguments: &[&str]) -> Ran {
    let binary = binary();
    let folder = TempDir::new().expect("a temporary folder can be made");
    for (name, text) in files {
        let path = folder.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("a folder in the sandbox can be made");
        }
        fs::write(path, text).expect("a file in the sandbox can be written");
    }
    let output = Command::new(binary)
        .args(arguments)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    Ran {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code(),
    }
}

fn run_of<'a>(json: &'a Value, name: &str, setup: Option<&str>) -> &'a Value {
    json["models"]
        .as_array()
        .and_then(|models| {
            models
                .iter()
                .find(|model| model["name"] == name && model["setup"]["id"].as_str() == setup)
        })
        .unwrap_or(&Value::Null)
}

#[test]
fn a_rule_of_a_model_with_a_setup_can_be_chosen_with_rule_alone() {
    let ran = run_in_folder(
        &[(FILE, include_str!("../../models/seventh/jar.model.ts"))],
        &["verify", "--json", "--no-cache", "--rule", "low"],
    );
    let json = ran.json();
    let declared = run_of(&json, "Jar", None);
    let small = run_of(&json, "Jar", Some("small"));
    assert!(
        ran.code == Some(1)
            && is_broken_after(rule(declared, "low"), 2)
            && rule(small, "low")["verdict"] == "holds",
        "`low` is a rule of one model, `Jar`, which is checked as declared and once more in its \
         setup `small`: `--rule low` checks it in both runs, broken after two steps with the cap \
         of 2 and holding with the cap of 1, and exits 1; no `--model` can tell the two runs \
         apart, since both are `Jar`\nexit {:?}\n{}",
        ran.code,
        ran.said()
    );
}

#[test]
fn a_removed_rule_about_runs_and_a_removed_witness_are_listed_as_removed() {
    let sandbox = Sandbox::with(&[(
        "climb.model.ts",
        include_str!("../../models/seventh/changes/before/climb.model.ts"),
    )]);
    sandbox.commit();
    sandbox.write(
        "climb.model.ts",
        include_str!("../../models/seventh/changes/after/climb.model.ts"),
    );
    let report = sandbox.verify(&["--base", "HEAD"]);
    for id in ["climbs", "three"] {
        assert_eq!(
            report.change(id)["change"],
            "removed",
            "`{id}` is in `Climb` at the base and not in the working tree, and `removed` is \
             \"present at `--base`, absent in the working tree (deleted rule ...)\"\n{report}"
        );
    }
}

#[test]
fn a_rule_loosened_in_the_states_of_a_setup_is_listed_as_loosened() {
    let sandbox = Sandbox::with(&[(
        "large.model.ts",
        include_str!("../../models/seventh/changes/before/large.model.ts"),
    )]);
    sandbox.commit();
    sandbox.write(
        "large.model.ts",
        include_str!("../../models/seventh/changes/after/large.model.ts"),
    );
    let report = sandbox.verify(&["--base", "HEAD"]);
    assert_eq!(
        report.change("low")["change"],
        "loosened",
        "in the setup `large` the cap is 2 and the state `n = 2, cap = 2` is reached: it passes \
         `s.n <= 2` and breaks `s.n <= 1`, so the run `Jar (large)` goes from `fail` to `pass`; \
         `loosened` is \"some reachable state of the new model satisfies the new rule and breaks \
         the old rule\"\n{report}"
    );
}
