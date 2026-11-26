#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::support::{Run, Sandbox, find_model, model, repository};

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

const SETS: [&str; 20] = [
    "samename",
    "exported",
    "nospace",
    "aliasruns",
    "noinit",
    "migrated",
    "leftover",
    "twowords",
    "wrapped",
    "onename",
    "importedtests",
    "importedrun",
    "specassume",
    "spechelper",
    "twosetups",
    "commasentence",
    "sumtype",
    "typesmodule",
    "barecomma",
    "falseassume",
];

const TIGHT_DEFINES: [&str; 5] = ["neverFull", "Half", "belowHalf", "reset", "stepOrReset"];

fn review() -> PathBuf {
    repository().join("tasks/quint/review")
}

fn copy_folder(sandbox: &Sandbox, from: &Path, to: &str) {
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
        copy_folder(&sandbox, &review().join("fixtures"), "fake/fixtures");
        for set in SETS {
            copy_folder(&sandbox, &review().join(set), set);
        }
        sandbox.write("fake/calls", "");
        Self { sandbox, program }
    }

    fn without_typecheck(&self, set: &str) {
        fs::remove_dir_all(
            self.sandbox
                .path()
                .join("fake/fixtures/typechecked")
                .join(set),
        )
        .expect("the typecheck of the set goes");
    }

    fn compiles(&self) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .filter(|call| call.starts_with("compile"))
            .map(str::to_owned)
            .collect()
    }

    fn report(&self, set: &str) -> Value {
        let empty = self.sandbox.path().join("no-programs");
        fs::create_dir_all(&empty).expect("an empty folder can be made");
        Run::of(
            self.sandbox
                .command()
                .env("EARNEST_QUINT", &self.program)
                .env("PATH", &empty)
                .args(["verify", set, "--json", "--no-cache"]),
        )
        .json()
    }
}

fn unreadable(report: &Value) -> Vec<(String, String, String)> {
    report["unreadable"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|problem| {
            (
                problem["file"].as_str().unwrap_or_default().to_owned(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn verdicts(found: &Value) -> Vec<(String, String)> {
    found["rules"]
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

fn speaks_of(report: &Value, file: &str, name: &str) -> bool {
    unreadable(report).iter().any(|(each, feature, message)| {
        each.ends_with(file) && (feature.contains(name) || message.contains(name))
    })
}

#[test]
fn a_mark_no_check_sees_is_reported_though_another_file_has_a_module_of_the_same_name() {
    let report = Quint::new().report("samename");
    assert!(
        speaks_of(&report, "a.qnt", "small"),
        "`Lib::small` of a.qnt is marked and no check sees it; the `Lib` of b.qnt is another \
         module: {report}"
    );
}

#[test]
fn a_mark_exported_by_a_middle_module_is_judged_in_the_check_that_imports_that_module() {
    let report = Quint::new().report("exported");
    let top = find_model(&report, "Top")
        .unwrap_or_else(|| panic!("`Top` sees `init`, `step` and `small` through `Mid`'s `export Spec.*`, as `quint verify --main Top --invariant small` does: {report}"));
    assert!(
        verdicts(top).contains(&("small".to_owned(), "broken".to_owned())),
        "{top}"
    );
}

#[test]
fn a_mark_written_with_no_space_after_the_three_slashes_is_not_dropped_without_a_word() {
    let report = Quint::new().report("nospace");
    assert!(
        find_model(&report, "A").is_some() || speaks_of(&report, "nospace.qnt", "A"),
        "`///@check` above `module A` reaches earnest as the doc `check`, and `A`'s failing \
         `upTest` is never played, with nothing said: {report}"
    );
    let judged = verdicts(model(&report, "B"));
    assert!(
        judged.iter().any(|(id, _)| id == "wraps") || speaks_of(&report, "nospace.qnt", "wraps"),
        "`///@check broken` above `wraps` is dropped: {judged:?}"
    );
}

#[test]
fn a_run_reached_through_an_instance_under_a_name_is_played_when_a_marked_run_is_beside_it() {
    let report = Quint::new().report("aliasruns");
    let found = find_model(&report, "A").unwrap_or_else(|| {
        panic!("`quint test --main A` plays `A::S::upTest` and `A::S::badTest`: {report}")
    });
    assert_eq!(found["status"], "fail", "`S::badTest` fails: {found}");
    let judged = verdicts(found);
    assert!(
        judged
            .iter()
            .any(|(id, verdict)| id.ends_with("badTest") && verdict == "broken"),
        "{judged:?}"
    );
}

#[test]
fn a_marked_property_of_a_check_with_no_step_is_not_left_unjudged_without_a_word() {
    let report = Quint::new().report("noinit");
    let judged = find_model(&report, "N").map_or_else(Vec::new, verdicts);
    assert!(
        judged.iter().any(|(id, _)| id == "small") || speaks_of(&report, "noinit.qnt", "small"),
        "`small` is marked `@check` and false, and `N` passes on its run alone: {report}"
    );
}

#[test]
fn the_module_the_leftover_checks_file_asks_for_defines_each_name_of_its_setup_once() {
    let report = Quint::new().report("leftover");
    let (_, _, message) = unreadable(&report)
        .into_iter()
        .find(|(file, ..)| file.ends_with("earnest.quint.json"))
        .unwrap_or_else(|| panic!("{report}"));
    for name in TIGHT_DEFINES {
        for keyword in ["val", "action", "pure val"] {
            let added = format!("{keyword} {name} =");
            assert!(
                !message.lines().any(|line| line.trim().starts_with(&added)),
                "the message says to copy `Tight` and add `{added} …` to the copy, which \
                 already defines `{name}`: Quint rejects the file with QNT101: {message}"
            );
        }
    }
}

#[test]
fn following_the_message_for_the_setups_checks_file_keeps_the_verdicts_of_two_and_large() {
    let report = Quint::new().report("migrated");
    let large = find_model(&report, "Large")
        .unwrap_or_else(|| panic!("`Large` passed with its checks file: {report}"));
    assert_eq!(large["status"], "pass", "{large}");
    let two = model(&report, "TwoChecked");
    assert_eq!(
        two["status"], "pass",
        "`Two` judged `ok` and `fine` and passed; the message's marks on `Base` for `Exported` \
         and `Plain` also land in `TwoChecked`, and no line of it says so: {two}"
    );
}

#[test]
fn a_property_seen_through_an_import_under_a_name_and_its_export_is_judged_once() {
    let report = Quint::new().report("migrated");
    let ids: Vec<String> = verdicts(model(&report, "Exported"))
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let mut once = ids.clone();
    once.dedup();
    assert_eq!(
        ids, once,
        "`Exported` is `import Base(K = 1) as B` and `export B.*`"
    );
}

#[test]
fn a_name_a_module_doc_lists_under_two_words_is_not_judged_by_the_last_word_without_a_word() {
    let report = Quint::new().report("twowords");
    assert!(
        speaks_of(&report, "twowords.qnt", "small"),
        "`Twice` writes `@check small` and `@check broken small`, and is judged with `small` \
         broken, as if the first line were not there: {report}"
    );
}

#[test]
fn names_a_module_doc_wraps_onto_the_next_line_are_judged_or_reported() {
    let report = Quint::new().report("wrapped");
    let judged = find_model(&report, "Bare").map_or_else(Vec::new, verdicts);
    for name in ["small", "tiny"] {
        assert!(
            judged.iter().any(|(id, _)| id == name) || speaks_of(&report, "wrapped.qnt", name),
            "`/// @check inRange` then `///   small, tiny` above `Bare`: `{name}` is read as a \
             sentence, and `Bare` passes, where `quint verify --main Bare --invariant small` \
             finds a violation: {report}"
        );
    }
}

#[test]
fn a_name_list_that_ends_its_line_with_a_comma_is_read_on_or_said_to_continue() {
    let report = Quint::new().report("wrapped");
    let judged = find_model(&report, "Comma").map_or_else(Vec::new, verdicts);
    for name in ["small", "tiny"] {
        assert!(
            judged.iter().any(|(id, _)| id == name) || speaks_of(&report, "wrapped.qnt", name),
            "`/// @check inRange,` then `///   small, tiny` above `Comma` is called `not a mark \
             the checker knows`, which sends the author to the three words and not to the \
             wrapped line: {report}"
        );
    }
}

#[test]
fn a_module_doc_of_check_and_one_name_with_no_space_is_not_dropped_without_a_word() {
    let report = Quint::new().report("onename");
    assert!(
        find_model(&report, "Unspaced").is_some() || speaks_of(&report, "onename.qnt", "Unspaced"),
        "`///@check small` above `module Unspaced` reaches earnest as the doc `check small`, and \
         the folder reports no model and no problem: {report}"
    );
}

#[test]
fn a_check_that_imports_a_failing_test_run_does_not_pass() {
    let report = Quint::new().report("importedtests");
    let found = find_model(&report, "Plain")
        .unwrap_or_else(|| panic!("`Plain` imports `Counter.*` and marks `inRange`: {report}"));
    assert_ne!(
        found["status"], "pass",
        "`quint test --main Plain` plays `badTest`, which fails; the check plays every run it \
         sees whose name ends in `Test`: {found}"
    );
}

#[test]
fn a_run_marked_in_an_imported_module_is_judged_or_said_how_to_judge_it() {
    let report = Quint::new().report("importedrun");
    if let Some(found) = find_model(&report, "Plain") {
        let judged = verdicts(found);
        assert!(
            judged.iter().any(|(id, _)| id.ends_with("skips")),
            "`quint test --main Plain --match skips` plays `skips`: {judged:?}"
        );
        return;
    }
    for (file, feature, message) in unreadable(&report) {
        assert!(
            !message.contains("the check names this run"),
            "{file} {feature}: `Plain` names no run, `skips` is marked `@check broken` on its \
             own line in `Counter`, and the message says neither why it can't be played nor \
             what to write: {message}"
        );
    }
}

fn played_and_broken(report: &Value, main: &str, run: &str) {
    let found = model(report, main);
    assert_eq!(
        found["status"], "fail",
        "`quint test --main {main} --backend typescript` fails `{run}`: {found}"
    );
    assert!(
        verdicts(found)
            .iter()
            .any(|(id, verdict)| id.ends_with(run) && verdict == "broken"),
        "{found}"
    );
}

#[test]
fn a_failing_run_of_a_spec_that_holds_an_assumption_is_played_in_the_check_that_imports_it() {
    let report = Quint::new().report("specassume");
    played_and_broken(&report, "Setup", "upTest");
}

#[test]
fn a_failing_run_of_a_spec_that_imports_a_helper_under_a_name_is_played_in_the_check_that_imports_it()
 {
    let report = Quint::new().report("spechelper");
    played_and_broken(&report, "Setup", "upTest");
}

#[test]
fn a_check_that_imports_a_spec_whole_is_judged_when_another_check_of_the_file_imports_it_under_a_name()
 {
    let report = Quint::new().report("twosetups");
    let whole = find_model(&report, "Whole").unwrap_or_else(|| {
        panic!(
            "`Whole` is `import Spec.*` beside `Named`'s `import Spec as S`, no module exports \
             anything, and the message blames an export; earnest builds `Whole` once `Spec` has \
             a run ending in `Test`: {report}"
        )
    });
    assert!(
        verdicts(whole).contains(&("small".to_owned(), "broken".to_owned())),
        "{whole}"
    );
}

#[test]
fn a_name_list_whose_comma_is_followed_by_a_sentence_is_not_called_an_unknown_mark() {
    let report = Quint::new().report("commasentence");
    for (file, feature, message) in unreadable(&report) {
        assert!(
            !message.contains("not a mark the checker knows"),
            "{file} {feature}: `/// @check broken small,` then `/// which the third step \
             breaks.` uses a known word; the message sends the author to the three words and \
             not to the trailing comma: {message}"
        );
    }
}

#[test]
fn a_failing_run_of_a_spec_whose_state_holds_a_sum_type_is_played_in_the_check_that_imports_it() {
    let report = Quint::new().report("sumtype");
    played_and_broken(&report, "Setup", "twiceTest");
}

#[test]
fn a_failing_run_of_a_spec_that_imports_its_types_from_a_types_file_is_played_in_the_setup_file() {
    let report = Quint::new().report("typesmodule");
    played_and_broken(&report, "Setup", "twiceTest");
}

#[test]
fn a_bare_mark_followed_by_a_comma_as_the_unreadable_names_line_asks_is_read_on() {
    let report = Quint::new().report("barecomma");
    let judged = find_model(&report, "W3").map_or_else(Vec::new, verdicts);
    assert!(
        judged.iter().any(|(id, _)| id == "ok"),
        "`/// @check` then `/// ok` is unreadable with `end the line of the mark with a comma`; \
         doing so, `/// @check,` then `/// ok`, is called `not a mark the checker knows` and no \
         module is checked: {report}"
    );
}

fn built_and_compiled(set: &str, main: &str) {
    let built = Quint::new();
    let ours = built.report(set);
    assert!(
        !built
            .compiles()
            .iter()
            .any(|call| call.contains(&format!("--main {main} "))),
        "{set}: `{main}` is built from the typecheck: {:?}",
        built.compiles()
    );
    let compiled = Quint::new();
    compiled.without_typecheck(set);
    let theirs = compiled.report(set);
    let (ours, theirs) = (model(&ours, main), model(&theirs, main));
    assert_eq!(
        ours["states"], theirs["states"],
        "{set}: states\n{ours}\n{theirs}"
    );
    let searched = |found: &Value| -> Vec<Value> {
        found["rules"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|rule| rule["kind"] != "witness")
            .cloned()
            .collect()
    };
    assert_eq!(
        searched(ours),
        searched(theirs),
        "{set}: each property, its verdict and its path, as `quint compile` gives them"
    );
    assert!(
        ours["runsNotPlayed"].is_null()
            && ours["rules"]
                .as_array()
                .is_some_and(|rules| rules.iter().any(|rule| rule["kind"] == "witness")),
        "{set}: the built check plays the runs of the spec it imports: {ours}"
    );
}

#[test]
fn a_spec_whose_state_holds_a_sum_type_is_built_as_the_model_quint_compile_writes() {
    built_and_compiled("sumtype", "Setup");
}

#[test]
fn a_spec_that_imports_its_types_from_another_file_is_built_as_the_model_quint_compile_writes() {
    built_and_compiled("typesmodule", "Setup");
}

#[test]
fn a_spec_that_holds_an_assumption_is_built_as_the_model_quint_compile_writes() {
    built_and_compiled("specassume", "Setup");
    built_and_compiled("falseassume", "Setup");
}

#[test]
fn a_spec_that_imports_a_helper_under_a_name_is_built_as_the_model_quint_compile_writes() {
    built_and_compiled("spechelper", "Setup");
}
