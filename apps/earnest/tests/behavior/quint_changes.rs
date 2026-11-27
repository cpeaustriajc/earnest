#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

use crate::support::{Run, Sandbox, fields, repository};

const FAKE: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
if [ "$1" = "--version" ]; then echo "0.33.1"; exit 0; fi
named=""
out=""
map=""
previous=""
file=""
for argument in "$@"; do
  case "$previous" in
    --out) out="$argument" ;;
    --source-map) map="$argument" ;;
    --main|--init|--step|--invariant|--temporal) named="$named $previous=$argument" ;;
  esac
  previous="$argument"
  file="$argument"
done
case "$1" in
  parse|typecheck) file="$2" ;;
esac
folder="$(cd "$(dirname "$file")" && pwd -P)"
key="$( {
  echo "$1$named $(basename "$file")"
  for each in "$folder"/*.qnt; do basename "$each"; cat "$each"; done
} | /usr/bin/sha256sum | /usr/bin/cut -c1-16)"
fixtures="$here/fixtures"
if [ ! -f "$fixtures/$key.code" ] && [ -n "$EARNEST_FAKE_RECORD" ]; then
  fixtures="$EARNEST_FAKE_RECORD"
  said="$(mktemp)"
  PATH="$EARNEST_FAKE_PATH" "$EARNEST_FAKE_REAL" "$@" > "$said"
  echo "$?" > "$fixtures/$key.code"
  sed "s#$folder#@FOLDER@#g" "$said" > "$fixtures/$key.stdout"
  rm -f "$said"
  if [ -n "$out" ] && [ -f "$out" ]; then sed "s#$folder#@FOLDER@#g" "$out" > "$fixtures/$key.out"; fi
  if [ -n "$map" ] && [ -f "$map" ]; then sed "s#$folder#@FOLDER@#g" "$map" > "$fixtures/$key.map"; fi
fi
if [ ! -f "$fixtures/$key.code" ]; then
  echo "no fixture $key for quint $*" >&2
  exit 1
fi
if [ -n "$out" ] && [ -f "$fixtures/$key.out" ]; then sed "s#@FOLDER@#$folder#g" "$fixtures/$key.out" > "$out"; fi
if [ -n "$map" ] && [ -f "$fixtures/$key.map" ]; then sed "s#@FOLDER@#$folder#g" "$fixtures/$key.map" > "$map"; fi
sed "s#@FOLDER@#$folder#g" "$fixtures/$key.stdout"
exit "$(cat "$fixtures/$key.code")"
"#;

const HEADER: &str = "rule changes since HEAD";
const CLOCK: &str = "specs/clock.qnt";
const REPOSITORY: &str = "repo";

fn changes_folder() -> PathBuf {
    repository().join("tasks/quint/changes")
}

fn original(name: &str) -> String {
    let path = changes_folder().join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

fn which(program: &str) -> PathBuf {
    let listed = std::env::var_os("PATH").expect("the tests run with a `PATH`");
    std::env::split_paths(&listed)
        .map(|folder| folder.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("`{program}` is on the `PATH`"))
}

struct Repo {
    sandbox: Sandbox,
}

impl Repo {
    fn committed(files: &[(&str, &str)]) -> Self {
        let sandbox = Sandbox::empty();
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(
            sandbox.path().join("fake/quint"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the fake can be made a program");
        for entry in fs::read_dir(changes_folder().join("fixtures"))
            .expect("the recorded answers of `quint` can be listed")
            .filter_map(Result::ok)
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            let text = fs::read_to_string(entry.path()).expect("a fixture is text");
            sandbox.write(&format!("fake/fixtures/{name}"), &text);
        }
        sandbox.write("fake/calls", "");
        fs::create_dir_all(sandbox.path().join("programs")).expect("a folder can be made");
        symlink(which("git"), sandbox.path().join("programs/git")).expect("a link can be made");
        let repo = Self { sandbox };
        fs::create_dir_all(repo.root()).expect("the repository's folder can be made");
        for (relative, text) in files {
            repo.write(relative, text);
        }
        repo.git(&["init", "--quiet"]);
        repo.write(".gitignore", ".earnest/\n");
        repo.git(&["add", "--all"]);
        repo.git(&["commit", "--quiet", "--allow-empty", "--message", "base"]);
        repo
    }

    fn root(&self) -> PathBuf {
        self.sandbox.path().join(REPOSITORY)
    }

    fn git(&self, arguments: &[&str]) {
        let output = Command::new("git")
            .current_dir(self.root())
            .args(["-c", "user.name=Conformance"])
            .args(["-c", "user.email=conformance@earnest.invalid"])
            .args(["-c", "commit.gpgsign=false"])
            .args(["-c", "init.defaultBranch=main"])
            .args(arguments)
            .output()
            .expect("`git` is on the `PATH`");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write(&self, relative: &str, text: &str) {
        self.sandbox
            .write(&format!("{REPOSITORY}/{relative}"), text);
    }

    fn read(&self, relative: &str) -> String {
        self.sandbox.read(&format!("{REPOSITORY}/{relative}"))
    }

    fn edit(&self, relative: &str, from: &str, to: &str) {
        let text = self.read(relative);
        assert!(text.contains(from), "{relative} holds {from:?}");
        self.write(relative, &text.replacen(from, to, 1));
    }

    fn remove(&self, relative: &str) {
        fs::remove_file(self.root().join(relative)).expect("a file can be deleted");
    }

    fn run(&self, arguments: &[&str], quint: &Path) -> Run {
        let fake_path = std::env::var_os("PATH").unwrap_or_default();
        Run::of(
            self.sandbox
                .command()
                .current_dir(self.root())
                .env("EARNEST_QUINT", quint)
                .env("EARNEST_FAKE_PATH", fake_path)
                .env("PATH", self.sandbox.path().join("programs"))
                .args(arguments),
        )
    }

    fn earnest(&self, arguments: &[&str]) -> Run {
        self.run(arguments, &self.sandbox.path().join("fake/quint"))
    }

    fn calls(&self) -> Vec<String> {
        self.sandbox
            .read("fake/calls")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn base_parses(&self) -> usize {
        self.calls()
            .iter()
            .filter(|call| call.starts_with("parse") && call.contains("/.earnest/quint/base-"))
            .count()
    }
}

fn with_clock() -> Repo {
    Repo::committed(&[(CLOCK, &original("clock.qnt"))])
}

fn section(run: &Run) -> Vec<Vec<String>> {
    run.stdout
        .lines()
        .skip_while(|line| line.trim() != HEADER)
        .skip(1)
        .take_while(|line| line.starts_with(char::is_whitespace))
        .map(fields)
        .collect()
}

fn heads(run: &Run) -> Vec<(String, String)> {
    section(run)
        .into_iter()
        .filter(|line| {
            line.first()
                .is_some_and(|word| word != "was" && word != "now")
        })
        .map(|line| (line[0].clone(), line[1].clone()))
        .collect()
}

fn listed(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(change, rule)| ((*change).to_owned(), (*rule).to_owned()))
        .collect()
}

const SMALL_RULES: [&str; 5] = [
    "Small.inRange",
    "Small.belowLimit",
    "Small.atTop",
    "Small.alwaysInRange",
    "Small.tickTest (witness)",
];

const LARGE_RULES: [&str; 6] = [
    "Large.inRange",
    "Large.belowLimit",
    "Large.atTop",
    "Large.belowSix",
    "Large.alwaysInRange",
    "Large.tickTest (witness)",
];

fn every(change: &str) -> Vec<(String, String)> {
    SMALL_RULES
        .iter()
        .chain(&LARGE_RULES)
        .map(|rule| (change.to_owned(), (*rule).to_owned()))
        .collect()
}

#[test]
fn a_quint_file_the_base_does_not_have_lists_every_rule_and_run_of_its_checks_as_added() {
    let repo = Repo::committed(&[]);
    repo.write(CLOCK, &original("clock.qnt"));
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(heads(&run), every("added"), "{run}");
    assert!(
        run.stdout.contains(
            "  added      Small.inRange  \"The time stays between zero and the limit.\"  \
             specs/clock.qnt\n"
        ),
        "the line of an added rule, as a native model's added rule has it\n{run}"
    );
    assert!(
        section(&run)
            .iter()
            .any(|line| line[..2] == ["added", "Small.tickTest (witness)"]),
        "a run is listed as a witness, as the results list it\n{run}"
    );
    assert_eq!(repo.base_parses(), 0, "the base has no file to read");
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(
        json["ruleChanges"][0],
        json!({
            "file": "specs/clock.qnt",
            "model": "Small",
            "id": "inRange",
            "change": "added",
            "was": null,
            "now": {
                "says": "The time stays between zero and the limit.",
                "holds": "time >= 0 and time <= top",
            },
        })
    );
    assert_eq!(
        json["ruleChanges"][4]["kind"], "witness",
        "{}",
        json["ruleChanges"]
    );
    assert_eq!(
        json["ruleChanges"][2]["now"]["holds"], "reachable: time == Limit",
        "the mark is part of what the rule says"
    );
    let report = repo.earnest(&["report", "--base", "HEAD"]);
    report.exits(0);
    assert!(
        report.stdout.contains("## Rule changes")
            && report
                .stdout
                .contains("  added      Large.belowSix  specs/clock.qnt\n"),
        "the report lists them too, and a rule with no sentence of its own with no quoted \
         sentence, as the results print it\n{report}"
    );
    let below_six = json["ruleChanges"]
        .as_array()
        .and_then(|changes| changes.iter().find(|change| change["id"] == "belowSix"))
        .cloned()
        .unwrap_or_default();
    let judged = json["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == "Large"))
        .and_then(|model| model["rules"].as_array())
        .and_then(|rules| rules.iter().find(|rule| rule["id"] == "belowSix"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        below_six["now"]["says"], judged["says"],
        "its `says` is the one the result JSON gives it: {below_six} {judged}"
    );
    assert_eq!(below_six["now"]["says"], "Quint invariant `belowSix`");
    assert_eq!(
        json["ruleChanges"][4]["now"]["says"], "Quint run `tickTest`",
        "{}",
        json["ruleChanges"][4]
    );
}

#[test]
fn a_quint_file_deleted_since_the_base_lists_the_rules_of_its_checks_as_removed() {
    let repo = with_clock();
    repo.remove(CLOCK);
    repo.write("specs/lamp.qnt", &original("lamp.qnt"));
    repo.write("specs/switch.qnt", &original("switch.qnt"));
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    let mut expected = listed(&[("added", "lamp.litOrDark")]);
    expected.extend(every("removed"));
    assert_eq!(heads(&run), expected, "{run}");
}

#[test]
fn a_rule_is_added_removed_reworded_or_changed_by_name_within_its_check() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "  val belowSix",
        "  /// @check\n  val fresh = time >= 0\n  val belowSix",
    );
    repo.edit(
        CLOCK,
        "  /// @check\n  /// The time is in range in every state of every run.\n",
        "  /// The time is in range in every state of every run.\n",
    );
    repo.edit(
        CLOCK,
        "The clock shows the limit.",
        "The clock shows its limit.",
    );
    repo.edit(
        CLOCK,
        "val belowLimit = time < Limit",
        "val belowLimit = time <= Limit",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD", "--model", "Small"]);
    run.exits(1);
    assert_eq!(
        heads(&run),
        listed(&[
            ("changed", "Small.belowLimit"),
            ("reworded", "Small.atTop"),
            ("added", "Small.fresh"),
            ("removed", "Small.alwaysInRange"),
        ]),
        "the rules the check has now in its order, then the one it lost\n{run}"
    );
    let lines = section(&run);
    assert_eq!(
        lines[1..3],
        [
            fields(
                "    was  \"The time reaches the limit, so it is not always below it.\"  broken: \
                 time < Limit"
            ),
            fields(
                "    now  \"The time reaches the limit, so it is not always below it.\"  broken: \
                 time <= Limit"
            ),
        ],
        "{run}"
    );
    assert_eq!(repo.base_parses(), 1, "{:?}", repo.calls());
}

#[test]
fn a_reformatted_file_or_a_changed_helper_under_a_rule_lists_nothing() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "val inRange = time >= 0 and time <= top",
        "val inRange =\n    time >= 0\n    and time <= top",
    );
    repo.edit(CLOCK, "  run tickTest", "\n\n  run   tickTest");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(section(&run), [fields("  none")], "{run}");
    repo.edit(CLOCK, "val top = Limit", "val top = Limit + 0");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(
        section(&run),
        [fields("  none")],
        "a part is listed when what it says changed, not when the model under it did\n{run}"
    );
}

#[test]
fn a_property_marked_broken_since_the_base_is_changed() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "  /// @check\n  /// The time stays",
        "  /// @check broken\n  /// The time stays",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD", "--model", "Small"]);
    assert_eq!(
        heads(&run),
        listed(&[("changed", "Small.inRange")]),
        "{run}"
    );
    assert_eq!(
        section(&run)[2],
        fields(
            "    now  \"The time stays between zero and the limit.\"  broken: time >= 0 and time <= \
             top"
        ),
        "{run}"
    );
}

fn warned_and_not_listed(run: &Run, file: &str) {
    assert!(
        run.stderr.contains(&format!(
            "warning: {file} is unreadable at HEAD, so its rule changes are not listed"
        )),
        "{run}"
    );
}

#[test]
fn a_base_that_does_not_parse_is_warned_of() {
    let repo = Repo::committed(&[(CLOCK, "module Clock {\n")]);
    repo.write(CLOCK, &original("clock.qnt"));
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    warned_and_not_listed(&run, CLOCK);
    assert_eq!(
        section(&run),
        [fields(
            "  not compared  specs/clock.qnt  unreadable at HEAD"
        )],
        "the section names the file it could not compare, and does not say `none`\n{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(json["ruleChanges"], json!([]));
    assert_eq!(
        json["notCompared"],
        json!([{ "file": "specs/clock.qnt", "reason": "unreadable" }])
    );
}

#[test]
fn a_base_whose_checks_a_checks_file_lists_is_warned_of() {
    let repo = Repo::committed(&[
        (CLOCK, &original("clock.qnt")),
        ("specs/earnest.quint.json", "[]\n"),
    ]);
    repo.remove("specs/earnest.quint.json");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    warned_and_not_listed(&run, CLOCK);
    assert_eq!(
        section(&run),
        [fields(
            "  not compared  specs/clock.qnt  an earnest.quint.json lists its checks at HEAD"
        )],
        "{run}"
    );
    assert_eq!(repo.base_parses(), 0, "{:?}", repo.calls());
}

#[test]
fn a_base_is_warned_of_when_quint_is_not_installed() {
    let repo = with_clock();
    repo.edit(CLOCK, "val top = Limit", "val top = Limit + 0");
    let run = repo.run(
        &["verify", "--base", "HEAD"],
        &repo.sandbox.path().join("nowhere/quint"),
    );
    warned_and_not_listed(&run, CLOCK);
}

#[test]
fn a_changed_property_of_an_imported_file_is_listed_under_the_check_that_judges_it() {
    let repo = Repo::committed(&[
        ("specs/lamp.qnt", &original("lamp.qnt")),
        ("specs/switch.qnt", &original("switch.qnt")),
    ]);
    repo.edit("specs/switch.qnt", "not(on)", "if (on) false else true");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(section(&run), [fields("  none")], "a helper changed\n{run}");
    repo.edit("specs/switch.qnt", "lit or not(lit)", "not(lit) or lit");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(
        heads(&run),
        listed(&[("changed", "lamp.litOrDark")]),
        "`lamp.qnt` is as the base has it, and the file it imports is not\n{run}"
    );
}

#[test]
fn model_and_rule_narrow_the_quint_changes() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "The clock shows the limit.",
        "The clock shows its limit.",
    );
    repo.edit(CLOCK, "val belowSix = time < 6", "val belowSix = time < 7");
    let large = repo.earnest(&["verify", "--base", "HEAD", "--model", "Large"]);
    assert_eq!(
        heads(&large),
        listed(&[("reworded", "Large.atTop"), ("changed", "Large.belowSix")]),
        "{large}"
    );
    let rule = repo.earnest(&["verify", "--base", "HEAD", "--rule", "belowSix"]);
    assert_eq!(
        heads(&rule),
        listed(&[("reworded", "Large.atTop"), ("changed", "Large.belowSix")]),
        "only `Large` judges `belowSix`, and it is listed whole, as a native model is\n{rule}"
    );
    let every = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        heads(&every),
        listed(&[
            ("reworded", "Small.atTop"),
            ("reworded", "Large.atTop"),
            ("changed", "Large.belowSix"),
        ]),
        "{every}"
    );
}

#[test]
fn a_folder_with_native_and_quint_models_lists_the_changes_of_both() {
    let counter = repository().join("tasks/conformance/models/counter.model.ts");
    let counter = fs::read_to_string(&counter).expect("the fixture can be read");
    let repo = Repo::committed(&[("specs/counter.model.ts", &counter)]);
    repo.write(CLOCK, &original("clock.qnt"));
    repo.edit(
        "specs/counter.model.ts",
        "The count is never above three",
        "The count stays at most three",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    let mut expected = listed(&[("reworded", "Counter.bounded")]);
    expected.extend(every("added"));
    assert_eq!(heads(&run), expected, "{run}");
}

#[test]
fn a_file_as_the_base_has_it_starts_no_quint_for_the_base() {
    let repo = with_clock();
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(section(&run), [fields("  none")], "{run}");
    assert_eq!(repo.base_parses(), 0, "{:?}", repo.calls());
    repo.edit(CLOCK, "val top = Limit", "val top = Limit + 0");
    repo.earnest(&["verify", "--base", "HEAD"]).exits(0);
    assert_eq!(repo.base_parses(), 1, "{:?}", repo.calls());
    repo.earnest(&["verify", "--base", "HEAD"]).exits(0);
    assert_eq!(
        repo.base_parses(),
        1,
        "the base's parse is kept in the cache: {:?}",
        repo.calls()
    );
}

#[test]
fn a_folder_with_no_quint_file_gives_the_json_it_gave_before() {
    let counter = repository().join("tasks/conformance/models/counter.model.ts");
    let counter = fs::read_to_string(&counter).expect("the fixture can be read");
    let repo = Repo::committed(&[]);
    repo.write("counter.model.ts", &counter);
    let run = repo.earnest(&["verify", "--base", "HEAD", "--json"]);
    run.exits(0);
    assert_eq!(
        run.json()["ruleChanges"],
        json!([
            {
                "file": "counter.model.ts",
                "model": "Counter",
                "id": "bounded",
                "change": "added",
                "was": null,
                "now": { "says": "The count is never above three", "holds": "s.count <= 3" },
            },
            {
                "file": "counter.model.ts",
                "model": "Counter",
                "id": "even",
                "change": "added",
                "was": null,
                "now": { "says": "Twice the count is even", "holds": "m.rem(s.count * 2, 2) === 0" },
            },
        ]),
        "{run}"
    );
    assert!(repo.calls().is_empty(), "{:?}", repo.calls());
}

const UNREAD_TANK: &str =
    "import { m } from \"@earnestjs/model\";\nexport const now = Date.now();\n";

#[test]
fn a_native_file_unreadable_at_the_base_is_named_in_the_text_the_json_and_the_report() {
    let counter = repository().join("tasks/conformance/models/counter.model.ts");
    let counter = fs::read_to_string(&counter).expect("the fixture can be read");
    let repo = Repo::committed(&[("counter.model.ts", UNREAD_TANK)]);
    repo.write("counter.model.ts", &counter);
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    warned_and_not_listed(&run, "counter.model.ts");
    assert_eq!(
        section(&run),
        [fields(
            "  not compared  counter.model.ts  unreadable at HEAD"
        )],
        "{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(
        json["notCompared"],
        json!([{ "file": "counter.model.ts", "reason": "unreadable" }]),
        "{json}"
    );
    let report = repo.earnest(&["report", "--base", "HEAD"]);
    report.exits(0);
    assert!(
        report.stdout.contains(
            "## Rule changes\n\n```text\nrule changes since HEAD\n  not compared  \
             counter.model.ts  unreadable at HEAD\n```"
        ),
        "{report}"
    );
    assert!(!report.stdout.contains("  none"), "{report}");
}

const MARKED_AT_BASE: &str = "module Core {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 3
  val a = n >= 0
  val b = n < 3
}
/// @check a
module Good { import Core.* }
/// @check nosuch
module Bad { import Core.* }
";

#[test]
fn a_check_whose_marks_are_unreadable_at_the_base_does_not_hide_the_others_of_its_file() {
    let repo = Repo::committed(&[("specs/a.qnt", MARKED_AT_BASE)]);
    repo.write(
        "specs/a.qnt",
        &MARKED_AT_BASE
            .replace("val a = n >= 0", "val a = n >= 1")
            .replace("@check nosuch", "@check b"),
    );
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        heads(&run),
        listed(&[("changed", "Good.a"), ("not compared", "specs/a.qnt")]),
        "{run}"
    );
    assert_eq!(
        section(&run)[3],
        fields(
            "  not compared  specs/a.qnt  Bad  its marks are unreadable at HEAD: 10:1 the doc \
             comment of `Bad` names `nosuch`, which the module does not see"
        ),
        "{run}"
    );
    assert!(
        run.stderr.contains(
            "warning: the check `Bad` of specs/a.qnt is unreadable at HEAD, so its rule changes \
             are not listed"
        ),
        "{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(
        json["notCompared"],
        json!([{
            "file": "specs/a.qnt",
            "model": "Bad",
            "reason": "marks",
            "problem": {
                "file": "specs/a.qnt",
                "line": 10,
                "column": 1,
                "message": "the doc comment of `Bad` names `nosuch`, which the module does not see: \
                            name a definition or a run of `Bad`, or of a module it imports, as \
                            `Bad` writes it",
                "more": 0,
            },
        }])
    );
}

#[test]
fn a_folder_named_twice_under_two_spellings_lists_nothing_as_removed() {
    let counter = repository().join("tasks/conformance/models/counter.model.ts");
    let counter = fs::read_to_string(&counter).expect("the fixture can be read");
    let repo = Repo::committed(&[
        (CLOCK, &original("clock.qnt")),
        ("specs/counter.model.ts", &counter),
    ]);
    let whole = repo.root().join("specs");
    let whole = whole.to_string_lossy();
    for spellings in [["specs", "./specs"], [whole.as_ref(), "specs"]] {
        let mut arguments = vec!["verify", "--base", "HEAD"];
        arguments.extend(spellings);
        let run = repo.earnest(&arguments);
        run.exits(0);
        assert_eq!(
            section(&run),
            [fields("  none")],
            "the native and the Quint file are each compared once, by their real path\n{run}"
        );
    }
}

#[test]
fn a_quint_file_reached_through_a_link_is_compared_once_and_not_warned_of() {
    let repo = with_clock();
    symlink("clock.qnt", repo.root().join("specs/link.qnt")).expect("a link can be made");
    repo.git(&["add", "--all"]);
    repo.git(&["commit", "--quiet", "--message", "link"]);
    repo.edit(
        CLOCK,
        "val belowLimit = time < Limit",
        "val belowLimit = time <= Limit",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD", "--model", "Small"]);
    assert!(!run.stderr.contains("warning"), "{run}");
    assert_eq!(
        heads(&run),
        listed(&[("changed", "Small.belowLimit")]),
        "{run}"
    );
}

#[test]
fn a_renamed_local_of_a_block_lists_nothing() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "  val belowSix = time < 6",
        "  /// @check\n  val ahead = { val t = time + 1 t > 0 }\n  val belowSix = time < 6",
    );
    repo.git(&["commit", "--quiet", "--all", "--message", "block"]);
    repo.edit(
        CLOCK,
        "{ val t = time + 1 t > 0 }",
        "{ val u = time + 1 u > 0 }",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    run.exits(0);
    assert_eq!(section(&run), [fields("  none")], "{run}");
    repo.edit(
        CLOCK,
        "{ val u = time + 1 u > 0 }",
        "{ val u = time + 2 u > 0 }",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD", "--model", "Small"]);
    assert_eq!(heads(&run), listed(&[("changed", "Small.ahead")]), "{run}");
}

const VIEWED_AT_BASE: &str = "module Core {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 3
  /// @check
  val a = n >= 0
}
/// @check
module Good { import Core.* }

/// @check
module Other {
  var k: int
  action init = k' = 0
  action step = k' = (k + 1) % 3
  /// @check
  val b = k >= 0
  /// @View state
  val v = k
}
";

#[test]
fn a_mark_of_no_check_unreadable_at_the_base_is_named_and_every_check_is_compared() {
    let repo = Repo::committed(&[("m.qnt", VIEWED_AT_BASE)]);
    repo.write(
        "m.qnt",
        &VIEWED_AT_BASE
            .replace("val a = n >= 0", "val a = n >= 1")
            .replace("@View state", "@view state"),
    );
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        heads(&run),
        listed(&[("changed", "Good.a"), ("not compared", "m.qnt")]),
        "the base judges `Good` and `Other` as `earnest verify` would, so `Good.a` is listed\n{run}"
    );
    assert_eq!(
        section(&run)[3],
        fields(
            "  not compared  m.qnt  its marks are unreadable at HEAD: 18:1 the doc comment holds \
             `@View state`, which is not a mark"
        ),
        "the line names the mark and its place in the base's file\n{run}"
    );
    assert!(
        run.stderr.contains(
            "warning: a mark of m.qnt is unreadable at HEAD, 18:1 the doc comment holds `@View \
             state`, which is not a mark, so what it marks is not compared"
        ),
        "{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(json["notCompared"][0]["reason"], "marks", "{json}");
    assert_eq!(json["notCompared"][0]["problem"]["line"], 18, "{json}");
    assert_eq!(json["notCompared"][0]["problem"]["column"], 1, "{json}");
    assert!(
        json["notCompared"][0]["problem"]["message"]
            .as_str()
            .is_some_and(|message| message.starts_with("the doc comment holds `@View state`")),
        "{json}"
    );
    assert!(
        json["notCompared"][0].get("model").is_none(),
        "the mark belongs to no check: {json}"
    );
}

#[test]
fn several_unreadable_marks_of_one_check_name_the_first_and_count_the_others() {
    let repo = Repo::committed(&[(
        "specs/a.qnt",
        &MARKED_AT_BASE.replace("@check nosuch", "@check nosuch, nothing"),
    )]);
    repo.write(
        "specs/a.qnt",
        &MARKED_AT_BASE.replace("@check nosuch", "@check b"),
    );
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    let line = section(&run)
        .into_iter()
        .find(|line| line.first().is_some_and(|word| word == "not compared"))
        .unwrap_or_default();
    assert!(
        line.last().is_some_and(
            |said| said.starts_with("its marks are unreadable at HEAD: 10:1 ")
                && said.ends_with(", and 1 more")
        ),
        "{run}"
    );
}

const SHOP: &str = "/// @check
module Shop {
  var n: int
  var paid: bool
  var shipped: bool
  action init = all { n' = 0, paid' = false, shipped' = false }
  action order = all { n < 3, n' = n + 1, paid' = false, shipped' = false }
  action pay = all { n > 0, not(paid), paid' = true, n' = n, shipped' = shipped }
  action ship = all { paid, not(shipped), shipped' = true, n' = n, paid' = paid }
  action step = any { order, pay, ship }
  /// @view state
  val st = if (shipped) \"shipped\" else if (paid) \"paid\" else if (n > 0) \"ordered\" else \"empty\"
  /// @view group
  val grp = if (n >= 2) \"busy\" else \"quiet\"
  /// @check
  /// Shipping needs payment.
  val shipNeedsPay = shipped implies paid
  /// @check
  val payNeedsOrder = paid implies n > 0
  /// @check reachable
  val canShip = shipped and n == 3
  /// @check broken
  val neverBusy = n < 2
  /// @check
  temporal paidLeadsToShip = always(paid implies eventually(shipped))
  /// @check
  val tooMany = n < 3
}
";

#[test]
fn a_check_unreadable_now_is_not_compared_and_its_base_rules_are_not_removed() {
    let repo = Repo::committed(&[("m.qnt", SHOP)]);
    repo.write(
        "m.qnt",
        &SHOP
            .replacen(
                "/// @check\nmodule Shop",
                "/// @check nosuchname\nmodule Shop",
                1,
            )
            .replace(
                "  /// @check\n  /// Shipping",
                "  /// @view state\n  /// @check\n  /// Shipping",
            ),
    );
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        section(&run),
        [fields("  not compared  m.qnt  Shop  unreadable now")],
        "`Shop` is in its file and this build can't read it now, so none of its six rules is \
         removed\n{run}"
    );
    assert!(
        !run.stderr.contains("rule changes are not listed"),
        "the run already says why it is unreadable\n{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(json["ruleChanges"], json!([]));
    assert_eq!(
        json["notCompared"],
        json!([{ "file": "m.qnt", "model": "Shop", "reason": "unreadableNow" }])
    );
    let report = repo.earnest(&["report", "--base", "HEAD"]);
    assert!(
        report
            .stdout
            .contains("rule changes since HEAD\n  not compared  m.qnt  Shop  unreadable now\n"),
        "{report}"
    );
}

#[test]
fn one_check_unreadable_now_leaves_the_others_of_its_file_compared() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "/// @check\n/// The clock that counts",
        "/// @check nosuchname\n/// The clock that counts",
    );
    repo.edit(CLOCK, "val belowSix = time < 6", "val belowSix = time < 7");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        heads(&run),
        listed(&[
            ("changed", "Large.belowSix"),
            ("not compared", "specs/clock.qnt"),
        ]),
        "{run}"
    );
    assert_eq!(
        section(&run)[3],
        fields("  not compared  specs/clock.qnt  Small  unreadable now"),
        "{run}"
    );
    let large = repo.earnest(&["verify", "--base", "HEAD", "--model", "Large"]);
    assert_eq!(
        heads(&large),
        listed(&[("changed", "Large.belowSix")]),
        "`--model` leaves out the check it does not name\n{large}"
    );
}

#[test]
fn a_quint_file_that_no_longer_parses_is_not_compared() {
    let repo = with_clock();
    repo.edit(CLOCK, "module Small {", "module Small {{");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        section(&run),
        [fields("  not compared  specs/clock.qnt  unreadable now")],
        "{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(
        json["notCompared"],
        json!([{ "file": "specs/clock.qnt", "reason": "unreadableNow" }])
    );
}

#[test]
fn a_native_file_unreadable_now_is_not_compared_and_one_deleted_lists_its_rules_as_removed() {
    let counter = repository().join("tasks/conformance/models/counter.model.ts");
    let counter = fs::read_to_string(&counter).expect("the fixture can be read");
    let repo = Repo::committed(&[("kept.model.ts", &counter), ("gone.model.ts", &counter)]);
    repo.write("kept.model.ts", UNREAD_TANK);
    repo.remove("gone.model.ts");
    let run = repo.earnest(&["verify", "--base", "HEAD"]);
    assert_eq!(
        section(&run),
        [
            fields(
                "  removed    Counter.bounded  \"The count is never above three\"  gone.model.ts"
            ),
            fields("  removed    Counter.even  \"Twice the count is even\"  gone.model.ts"),
            fields("  not compared  kept.model.ts  unreadable now"),
        ],
        "{run}"
    );
    let json = repo.earnest(&["verify", "--base", "HEAD", "--json"]).json();
    assert_eq!(
        json["notCompared"],
        json!([{ "file": "kept.model.ts", "reason": "unreadableNow" }])
    );
}

#[test]
fn a_check_whose_mark_was_taken_off_lists_its_rules_as_removed() {
    let repo = with_clock();
    repo.edit(
        CLOCK,
        "/// @check\n/// The clock that counts",
        "/// The clock that counts",
    );
    let run = repo.earnest(&["verify", "--base", "HEAD", "--model", "Small"]);
    assert_eq!(
        heads(&run),
        SMALL_RULES
            .iter()
            .map(|rule| ("removed".to_owned(), (*rule).to_owned()))
            .collect::<Vec<_>>(),
        "`Small` is no longer a check, which this build reads\n{run}"
    );
}
