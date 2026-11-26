#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde_json::Value;

use crate::support::{Run, Sandbox, model, models, path_under, repository, rule};

const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
main=""
invariants=""
temporal=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "--main" ]; then main="$argument"; fi
  if [ "$previous" = "--invariant" ]; then invariants="$argument"; fi
  if [ "$previous" = "--temporal" ]; then temporal="$argument"; fi
  if [ "$previous" = "--out" ]; then out="$argument"; fi
  if [ "$previous" = "--source-map" ]; then map="$argument"; fi
  previous="$argument"
  file="$argument"
done
case "$1" in
  parse|typecheck) file="$2" ;;
esac
name="$(basename "$file" .qnt)"
folder="$(cd "$(dirname "$file")" && pwd)"
set="$(basename "$folder")"
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  parse)
    cp "$here/fixtures/parsed/$set/$name.parse.json" "$out"
    sed "s#\"0\":\"$name.qnt\"#\"0\":\"$folder/$name.qnt\"#" \
      "$here/fixtures/parsed/$set/$name.sourcemap.json" > "$map"
    ;;
  typecheck)
    if [ ! -f "$here/fixtures/typechecked/$set/$name.json" ]; then exit 1; fi
    cp "$here/fixtures/typechecked/$set/$name.json" "$out"
    ;;
  compile)
    fixture="$here/fixtures/compiled/$set/$main@$invariants@$temporal.json"
    if [ ! -f "$fixture" ]; then
      echo "no fixture for $set/$main@$invariants@$temporal" >&2
      exit 1
    fi
    cat "$fixture"
    ;;
esac
"##;

const SETS: [&str; 7] = ["specs", "failing", "typo", "lost", "runs", "rules", "old"];

fn marked() -> PathBuf {
    repository().join("tasks/quint/marked")
}

fn copy_folder(sandbox: &Sandbox, from: &PathBuf, to: &str) {
    let entries = fs::read_dir(from)
        .unwrap_or_else(|error| panic!("{} can't be listed: {error}", from.display()));
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            copy_folder(sandbox, &path, &format!("{to}/{name}"));
        } else {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
            sandbox.write(&format!("{to}/{name}"), &text);
        }
    }
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
        copy_folder(&sandbox, &marked().join("fixtures"), "fake/fixtures");
        for set in SETS {
            copy_folder(&sandbox, &marked().join(set), set);
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

    fn compiles(&self) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .filter(|call| call.starts_with("compile"))
            .map(str::to_owned)
            .collect()
    }

    fn edit(&self, relative: &str, from: &str, to: &str) {
        let text = self.sandbox.read(relative);
        assert!(text.contains(from), "{relative} holds {from:?}");
        self.sandbox.write(relative, &text.replace(from, to));
    }

    fn quint_entries(&self) -> Vec<(String, String)> {
        let folder = self.sandbox.path().join(".earnest/quint");
        let mut entries: Vec<(String, String)> = fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| {
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read_to_string(entry.path()).unwrap_or_default(),
                )
            })
            .collect();
        entries.sort();
        entries
    }
}

fn verdicts(report: &Value, name: &str) -> Vec<(String, String)> {
    model(report, name)["rules"]
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

fn problems(report: &Value) -> Vec<(String, u64, String, String)> {
    report["unreadable"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|problem| {
            (
                problem["file"].as_str().unwrap_or_default().to_owned(),
                problem["line"].as_u64().unwrap_or_default(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

type Listed<'a> = (&'a str, u64, &'a [(&'a str, &'a str)]);

fn pairs(all: &[(&str, &str)]) -> Vec<(String, String)> {
    all.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

#[test]
fn a_folder_of_marked_files_gives_what_the_same_checks_gave_when_a_checks_file_listed_them() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "specs", "--json"]);
    run.exits(0);
    let report = run.json();
    let names: Vec<String> = models(&report)
        .iter()
        .map(|each| each["name"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(names, ["bank", "Small", "Large"], "{report}");
    let listed: [Listed<'_>; 3] = [
        (
            "bank",
            4,
            &[("nonNegative", "holds"), ("depositTest", "holds")],
        ),
        (
            "Small",
            4,
            &[
                ("inRange", "holds"),
                ("belowLimit", "broken"),
                ("alwaysInRange", "holds"),
                ("tickTest", "holds"),
            ],
        ),
        (
            "Large",
            6,
            &[
                ("inRange", "holds"),
                ("belowLimit", "broken"),
                ("alwaysInRange", "holds"),
                ("tickTest", "holds"),
            ],
        ),
    ];
    for (name, states, judged) in listed {
        let found = model(&report, name);
        assert_eq!(
            (&found["status"], &found["states"]),
            (&serde_json::json!("pass"), &serde_json::json!(states)),
            "{name}: as the checks file of the first version of #72 gave it"
        );
        let verdicts = verdicts(&report, name);
        for each in pairs(judged) {
            assert!(verdicts.contains(&each), "{name}: {each:?} in {verdicts:?}");
        }
    }
    assert_eq!(
        rule(model(&report, "Small"), "atTop")["verdict"],
        "broken",
        "the marks also say a state is reached, which a checks file could not"
    );
    assert_eq!(
        rule(model(&report, "Small"), "inRange")["says"],
        "The time stays between zero and the limit.",
        "the doc comment below the mark is the rule's sentence"
    );
}

#[test]
fn a_marked_module_with_no_init_or_step_plays_its_runs_and_an_unmarked_one_is_not_judged() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "runs", "--json"]);
    run.exits(0);
    let report = run.json();
    assert_eq!(
        models(&report).len(),
        1,
        "only `Plays` is a check: {report}"
    );
    let plays = model(&report, "Plays");
    assert_eq!(plays["states"], 0, "{plays}");
    assert_eq!(
        verdicts(&report, "Plays"),
        pairs(&[
            ("upTest", "holds"),
            ("twice", "holds"),
            ("tooFar", "broken")
        ]),
        "a run ending in `Test`, a marked run of another name, and a run marked broken"
    );
    assert_eq!(rule(plays, "tooFar")["expected"], "broken", "{plays}");
    let text = quint.earnest(&["verify", "runs"]);
    assert_eq!(text.model_line("Plays")[4], "0 rules, 3 runs", "{text}");
}

#[test]
fn each_way_a_mark_is_unreadable_is_reported_at_its_line() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "rules", "--json"]);
    run.exits(1);
    let found: Vec<(u64, String)> = problems(&run.json())
        .into_iter()
        .map(|(_, line, feature, _)| (line, feature))
        .collect();
    let expected: Vec<(u64, String)> = [
        (2, "`module Spec`"),
        (10, "`module Empty`"),
        (21, "`zero`"),
        (26, "`module Worded`"),
    ]
    .iter()
    .map(|(line, feature)| (*line, (*feature).to_owned()))
    .collect();
    let mut sorted = found;
    sorted.sort();
    assert_eq!(
        sorted, expected,
        "a constant unset, nothing to judge, a mark no check sees, and a name on a module the \
         module does not see: {run}"
    );
    assert!(models(&run.json()).is_empty(), "nothing is judged: {run}");
}

#[test]
fn a_folder_with_a_checks_file_is_unreadable_at_it_with_what_to_write_for_every_entry() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "old", "--json"]);
    run.exits(1);
    let found = problems(&run.json());
    let [(file, line, feature, message)] = found.as_slice() else {
        panic!("one problem: {run}");
    };
    assert_eq!(
        (file.as_str(), *line, feature.as_str()),
        ("old/earnest.quint.json", 1, "`earnest.quint.json`"),
        "{run}"
    );
    for written in [
        "1. `clock.qnt`, the check of `Small`: write these lines above `module Small` \
         (`clock.qnt:14`):\n      /// @check inRange, alwaysInRange\n",
        "2. `clock.qnt`, the check of `Small` named `SmallBroken`: add this module to \
         `clock.qnt`, and mark nothing in `Small` for it:\n      /// @check broken \
         V::belowLimit\n      module SmallBroken {\n        import Clock(Limit = 3) as V\n        \
         action init = V::init\n        action step = V::step\n      }\n",
        "so `tickTest` is played from now on, which the entry left out",
        "3. `clock.qnt`, the check of `Large`: write these lines above `module Large` \
         (`clock.qnt:18`):\n      /// @check C::inRange\n",
        "so `C::tickTest` is played from now on",
    ] {
        assert!(message.contains(written), "{written}\nin\n{message}");
    }
    assert!(
        models(&run.json()).is_empty(),
        "nothing is checked by the file: {run}"
    );
    let text = quint.earnest(&["verify", "old"]);
    assert!(
        text.to_string()
            .contains("\n            module SmallBroken {\n"),
        "the text output keeps the module on lines of its own, to copy: {text}"
    );
}

#[test]
fn a_broken_mark_passes_while_its_counterexample_exists_and_fails_when_the_property_holds() {
    let quint = Quint::new();
    let met = quint.earnest(&["verify", "specs"]);
    met.exits(0);
    let detail = met.detail("Small");
    assert!(
        detail.contains(&"broken  belowLimit  (expected)".to_owned()),
        "{met}"
    );
    let reached = path_under(&detail, &["reached", "atTop"]);
    assert_eq!(reached.start, "time = 0", "{met}");
    assert_eq!(reached.actions(), ["step", "step", "step"], "{met}");
    let json = quint.earnest(&["verify", "specs", "--json"]);
    let small = model(&json.json(), "Small").clone();
    assert_eq!(rule(&small, "atTop")["expected"], "reached", "{small}");
    assert_eq!(rule(&small, "belowLimit")["expected"], "broken", "{small}");
    let unmet = quint.earnest(&["verify", "failing"]);
    unmet.exits(1);
    assert_eq!(unmet.model_line("wrong")[0], "fail", "{unmet}");
    assert_eq!(
        unmet.detail("wrong"),
        [
            "holds  small  a counterexample was expected",
            "never reached  three"
        ],
        "{unmet}"
    );
}

#[test]
fn a_search_that_stopped_early_leaves_a_broken_and_a_reachable_mark_unknown() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "failing", "--max-states", "1"]);
    run.exits(1);
    assert_eq!(run.model_line("wrong")[0], "unfinished", "{run}");
    assert_eq!(
        run.detail("wrong"),
        ["unknown  small", "unknown  three"],
        "{run}"
    );
}

#[test]
fn a_mark_with_a_word_the_checker_does_not_know_makes_the_check_unreadable_at_its_line() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "typo", "--json"]);
    run.exits(1);
    let found = problems(&run.json());
    let [(file, line, feature, message)] = found.as_slice() else {
        panic!("one problem: {run}");
    };
    assert_eq!(
        (file.as_str(), *line, feature.as_str()),
        ("typo/typo.qnt", 7, "`atStart`"),
        "the line of `/// @check reachble`: {run}"
    );
    assert!(message.contains("`@check reachble`"), "{message}");
    assert!(quint.compiles().is_empty(), "nothing is compiled");
}

#[test]
fn the_text_output_counts_the_rules_and_runs_of_each_model_so_a_lost_mark_shows() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "specs"]);
    run.exits(0);
    assert_eq!(run.model_line("Small")[4], "4 rules, 1 run", "{run}");
    assert_eq!(run.model_line("bank")[4], "1 rule, 1 run", "{run}");
    let lost = quint.earnest(&["verify", "lost"]);
    lost.exits(0);
    assert_eq!(
        lost.model_line("lost")[4],
        "1 rule, 0 runs",
        "`/// @chek` marks nothing, and the count says so: {lost}"
    );
}

#[test]
fn a_quint_file_with_no_marked_module_is_no_model_and_named_says_how_to_mark() {
    let quint = Quint::new();
    let searched = quint.earnest(&["verify", "specs", "--json"]);
    assert!(
        problems(&searched.json()).is_empty(),
        "`lib.qnt` is left out of a search: {searched}"
    );
    let named = quint.earnest(&["verify", "specs/lib.qnt", "--json"]);
    named.exits(1);
    let found = problems(&named.json());
    let [(file, _, _, message)] = found.as_slice() else {
        panic!("one problem: {named}");
    };
    assert_eq!(file, "specs/lib.qnt", "{named}");
    assert!(message.contains("`/// @check`"), "{message}");
}

#[test]
fn a_changed_mark_checks_again_and_changes_the_quint_entries() {
    let quint = Quint::new();
    quint.earnest(&["verify", "specs"]).exits(0);
    let before = quint.quint_entries();
    let cached = quint.earnest(&["verify", "specs"]);
    assert!(
        cached.model_line("bank").contains(&"(cached)".to_owned()),
        "{cached}"
    );
    let said = "The balance never goes below zero.";
    let reworded = "The balance stays at zero or above.";
    quint.edit("specs/bank.qnt", said, reworded);
    quint.edit("fake/fixtures/parsed/specs/bank.parse.json", said, reworded);
    let again = quint.earnest(&["verify", "specs", "--json"]);
    again.exits(0);
    let bank = model(&again.json(), "bank").clone();
    assert_eq!(bank["cached"], false, "{bank}");
    assert_eq!(rule(&bank, "nonNegative")["says"], reworded, "{bank}");
    assert_ne!(
        quint.quint_entries(),
        before,
        "the entries are written again"
    );
    quint.edit(
        "specs/bank.qnt",
        "/// @check\n  /// The balance",
        "/// @check broken\n  /// The balance",
    );
    quint.edit(
        "fake/fixtures/parsed/specs/bank.parse.json",
        "\"@check\\nThe balance",
        "\"@check broken\\nThe balance",
    );
    let broken = quint.earnest(&["verify", "specs"]);
    broken.exits(1);
    assert_eq!(
        broken.detail("bank"),
        ["holds  nonNegative  a counterexample was expected"],
        "{broken}"
    );
}
