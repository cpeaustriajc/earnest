#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::support::{Run, Sandbox, fields, model, repository, rule, without};

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
    if [ -f "$fixture.stderr" ]; then
      sed "s#@FOLDER@#$folder#g" "$fixture.stderr" >&2
      exit 1
    fi
    if [ ! -f "$fixture" ]; then
      echo "no fixture for $set/$main@$invariants@$temporal" >&2
      exit 1
    fi
    cat "$fixture"
    ;;
  verify)
    cat "$here/verify/$main"
    ;;
esac
"##;

const SETS: [&str; 16] = [
    "labels",
    "bare",
    "kinds",
    "imported",
    "refused",
    "twice",
    "outside",
    "unread",
    "checked",
    "numbers",
    "numbers_bare",
    "defs",
    "defs_bare",
    "override",
    "near",
    "quintrefuses",
];

const SHOP_HOLDS: &str = "[TLC] Running TLC model checker...\n\
                          Model checking completed. No error has been found.\n\
                          2700 states generated, 1000 distinct states found, 0 states left on \
                          queue.\n\
                          Finished in 00s at (2026-11-26 10:00:00)\n\n\
                          [ok] No violation found (692ms).\n";

fn views() -> PathBuf {
    repository().join("tasks/quint/views")
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
        copy_folder(&sandbox, &views().join("fixtures"), "fake/fixtures");
        for set in SETS {
            copy_folder(&sandbox, &views().join(set), set);
        }
        sandbox.write("fake/calls", "");
        sandbox.write("fake/verify/shop", SHOP_HOLDS);
        sandbox.write("java/java", "#!/bin/sh\nexit 0\n");
        fs::set_permissions(
            sandbox.path().join("java/java"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the stand-in for Java can be made a program");
        Self { sandbox, program }
    }

    fn earnest(&self, arguments: &[&str]) -> Run {
        Run::of(
            self.sandbox
                .command()
                .env("EARNEST_QUINT", &self.program)
                .env("PATH", self.sandbox.path().join("java"))
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

    fn use_version(&self, version: &str) {
        let text = fs::read_to_string(views().join(version).join("counter.qnt"))
            .expect("the version of the counter can be read");
        self.sandbox.write("cache/counter.qnt", &text);
        for stage in ["parsed", "compiled", "typechecked"] {
            let from = views().join("fixtures").join(stage).join(version);
            if from.is_dir() {
                copy_folder(
                    &self.sandbox,
                    &from,
                    &format!("fake/fixtures/{stage}/cache"),
                );
            }
        }
    }
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

fn problem_at(found: &[(String, u64, String, String)], line: u64) -> String {
    let at: Vec<&(String, u64, String, String)> =
        found.iter().filter(|each| each.1 == line).collect();
    match at.as_slice() {
        [only] => format!("{} {}", only.2, only.3),
        _ => panic!("one problem at line {line}: {found:?}"),
    }
}

fn states_drawn(chart: &str) -> Vec<String> {
    chart
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("state \""))
        .filter_map(|rest| {
            rest.split_once("\" as s")
                .map(|(label, _)| label.to_owned())
        })
        .collect()
}

fn groups_drawn(chart: &str) -> Vec<String> {
    chart
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("state \""))
        .filter_map(|rest| {
            rest.split_once("\" as g")
                .map(|(label, _)| label.to_owned())
        })
        .collect()
}

fn result_line(run: &Run, name: &str) -> Vec<String> {
    let mut line = run.model_line(name);
    line.retain(|field| !field.ends_with(" ms"));
    line
}

#[test]
fn a_state_is_labelled_by_the_view_val_and_drawn_in_the_box_of_its_group() {
    let quint = Quint::new();
    let run = quint.earnest(&["chart", "labels", "--max-arrows", "200"]);
    run.exits(0);
    let states = states_drawn(&run.stdout);
    assert!(
        states.iter().all(|label| !label.contains("base = ")),
        "no state is labelled with every variable: {}",
        run.stdout
    );
    for label in [
        "nothing placed",
        "placed",
        "accepted, none shown",
        "some shown",
        "all shown",
    ] {
        assert!(
            states.iter().any(|each| each == label),
            "{label}: {states:?}"
        );
    }
    assert_eq!(
        groups_drawn(&run.stdout),
        [
            "#123; retried#58; 0 #125;",
            "#123; retried#58; 1 #125;",
            "#123; retried#58; 2 #125;",
            "#123; retried#58; 3 #125;",
        ],
        "the group is a record, printed as Quint writes it: {}",
        run.stdout
    );
    assert_eq!(
        states
            .iter()
            .filter(|each| *each == "nothing placed")
            .count(),
        1,
        "the states with one label and one group are one box"
    );
    assert!(
        states.len() < 20,
        "1000 states in fewer than 20 boxes: {states:?}"
    );
}

#[test]
fn a_label_may_be_a_tuple_of_parts_of_the_state_and_a_group_a_match_giving_a_text() {
    let quint = Quint::new();
    let run = quint.earnest(&["chart", "kinds"]);
    run.exits(0);
    let states = states_drawn(&run.stdout);
    for label in ["(Red, 0)", "(Green, 2)", "(Amber, 1)"] {
        assert!(
            states.iter().any(|each| each == label),
            "{label}: {}",
            run.stdout
        );
    }
    assert_eq!(groups_drawn(&run.stdout), ["stop", "go"], "{}", run.stdout);
}

#[test]
fn the_marks_change_no_verdict_no_count_and_no_result_line() {
    let quint = Quint::new();
    let marked = quint.earnest(&["verify", "labels"]);
    let bare = quint.earnest(&["verify", "bare"]);
    marked.exits(0);
    bare.exits(0);
    assert_eq!(
        result_line(&marked, "shop"),
        result_line(&bare, "shop"),
        "{marked}\n{bare}"
    );
    assert_eq!(
        result_line(&marked, "shop"),
        ["pass", "shop", "1000 states", "2 rules, 0 runs"]
    );
    assert_eq!(marked.summary_line(), bare.summary_line());
    let keys = ["milliseconds", "cached", "file"];
    let marked = quint.earnest(&["verify", "labels", "--json", "--no-cache"]);
    let bare = quint.earnest(&["verify", "bare", "--json", "--no-cache"]);
    assert_eq!(
        without(&marked.json(), &keys),
        without(&bare.json(), &keys),
        "the JSON output is the same but for the file"
    );
}

#[test]
fn the_report_draws_the_labelled_chart_and_the_json_reads_the_check_as_before() {
    let quint = Quint::new();
    let run = quint.earnest(&["report", "labels"]);
    run.exits(0);
    assert!(run.stdout.contains("```mermaid"), "{run}");
    assert!(run.stdout.contains("state \"nothing placed\" as"), "{run}");
    assert!(!run.stdout.contains("base = Set()"), "{run}");
    let json = quint.earnest(&["verify", "labels", "--json"]);
    json.exits(0);
    let report = json.json();
    assert_eq!(model(&report, "shop")["status"], "pass", "{report}");
    assert_eq!(
        rule(model(&report, "shop"), "shownWasAccepted")["says"],
        "The screen shows only orders the server accepted."
    );
}

#[test]
fn a_check_sees_a_label_its_imports_bring_and_one_of_its_own() {
    let quint = Quint::new();
    let small = quint.earnest(&["chart", "imported", "--model", "Small"]);
    small.exits(0);
    assert_eq!(
        states_drawn(&small.stdout),
        ["start", "between", "top"],
        "{small}"
    );
    let named = quint.earnest(&["chart", "imported", "--model", "Named"]);
    named.exits(0);
    assert_eq!(
        groups_drawn(&named.stdout),
        ["low half", "high half"],
        "{named}"
    );
    assert_eq!(
        states_drawn(&named.stdout),
        ["start", "between", "top"],
        "`S::stage` labels the states of `Named`, which imports `Spec` as `S`: {named}"
    );
}

#[test]
fn a_view_mark_that_labels_nothing_is_unreadable_at_its_line() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "refused", "--json"]);
    run.exits(1);
    let report = run.json();
    let found = problems(&report);
    let expected: [(u64, &str); 13] = [
        (
            3,
            "`n` `n` is a `var`, which gives no value to label the chart with",
        ),
        (
            12,
            "`titled` the doc comment of `titled` holds `@view title`, which is not a mark",
        ),
        (
            16,
            "`bare` `@view` says what part of the chart `bare` labels",
        ),
        (
            19,
            "`named` `@view state` stands alone above the `val` it labels the chart with, \
              and here `, n` follows it",
        ),
        (22, "`moved` `moved` is an `action`, which gives no value"),
        (
            25,
            "`twice` `twice` is a definition with parameters, which gives no value",
        ),
        (28, "`staysSmall` `staysSmall` is a `temporal` definition"),
        (31, "`playTest` `playTest` is a `run`"),
        (
            34,
            "`unspaced` the doc comment holds `view state`, which is how Quint hands over \
              `///@view state`",
        ),
        (38, "`both` the doc comment holds a second `@view` mark"),
        (
            43,
            "`module moduled` the doc comment of the module `moduled` holds `@view`, and a \
              module labels no chart: write `/// @view state` or `/// @view group` on the line \
              above the `val`",
        ),
        (55, "`K` `K` is a `const`"),
        (
            63,
            "`label` `label` is marked `@view state`, and no check that searches sees it",
        ),
    ];
    for (line, start) in expected {
        let said = problem_at(&found, line);
        assert!(said.starts_with(start), "line {line}: {said}");
    }
    assert_eq!(found.len(), 13, "{found:#?}");
    assert_eq!(
        model(&report, "marks")["status"],
        "pass",
        "a mark that labels nothing leaves the check as it is"
    );
    assert_eq!(
        model(&report, "marks")["rules"].as_array().map(Vec::len),
        Some(2)
    );
}

#[test]
fn the_labels_of_a_check_win_over_those_of_a_module_it_imports_plainly() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "twice", "--json"]);
    run.exits(0);
    let report = run.json();
    assert!(problems(&report).is_empty(), "{report}");
    let own = quint.earnest(&["chart", "twice", "--model", "twice"]);
    assert_eq!(groups_drawn(&own.stdout), ["small", "big"], "{own}");
    assert_eq!(states_drawn(&own.stdout), ["0", "1", "2", "3"], "{own}");
    let chart = quint.earnest(&["chart", "twice", "--model", "once"]);
    assert_eq!(groups_drawn(&chart.stdout), ["zero", "above"], "{chart}");
    assert_eq!(
        states_drawn(&chart.stdout),
        ["even", "odd", "even"],
        "a check with no label of its own uses the one of the module it imports: {chart}"
    );
}

#[test]
fn a_label_that_reads_a_variable_the_search_does_not_hold_is_refused_at_its_mark() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "outside", "--json"]);
    run.exits(1);
    let found = problems(&run.json());
    assert_eq!(
        problem_at(&found, 12),
        "`shown` the chart label `shown` reads `unread`, which no part of the check reads: the \
         search holds only the variables that `init`, `step` and the checked properties read, \
         so the label would change what is searched. Label the states with variables the check \
         reads"
    );
}

#[test]
fn a_label_the_reader_can_not_read_gives_the_readers_problem_at_its_mark() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "unread", "--json"]);
    run.exits(1);
    let found = problems(&run.json());
    let said = problem_at(&found, 11);
    assert!(
        said.starts_with(
            "`shown` the chart label `shown` can't be read: `chooseSome`, whose choice Quint's \
             backends make differently, is not read"
        ),
        "{said}"
    );
    assert!(
        said.ends_with("or a record or a tuple of parts of the state"),
        "{said}"
    );
}

#[test]
fn a_property_may_also_label_the_chart_and_a_check_that_only_plays_runs_draws_none() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "checked", "--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        rule(model(&report, "checked"), "belowThree")["says"],
        "The counter stays below three.",
        "the mark is not part of the rule's sentence"
    );
    assert_eq!(model(&report, "checked")["status"], "pass");
    assert_eq!(
        model(&report, "checked")["rules"].as_array().map(Vec::len),
        Some(1),
        "the property that is also the label is judged once, though it is compiled twice"
    );
    assert_eq!(model(&report, "played")["status"], "pass");
    let found = problems(&report);
    assert!(
        problem_at(&found, 30).starts_with(
            "`shown` `shown` is marked `@view state`, and no check that searches sees it"
        ),
        "{found:?}"
    );
    let chart = quint.earnest(&["chart", "checked", "--model", "checked"]);
    assert_eq!(
        states_drawn(&chart.stdout),
        ["true", "true", "false"],
        "{chart}"
    );
    assert_eq!(groups_drawn(&chart.stdout), ["low", "high"], "{chart}");
}

#[test]
fn adding_editing_and_removing_a_view_mark_changes_the_chart_on_the_next_run() {
    let quint = Quint::new();
    let chart = |version: &str| {
        quint.use_version(version);
        let run = quint.earnest(&["chart", "cache"]);
        run.exits(0);
        (states_drawn(&run.stdout), groups_drawn(&run.stdout))
    };
    assert_eq!(
        chart("cache1"),
        (vec!["low".to_owned(), "high".to_owned()], vec![]),
        "the mark labels the states"
    );
    assert_eq!(
        chart("cache2"),
        (vec!["start".to_owned(), "running".to_owned()], vec![]),
        "the `val` under the mark was edited"
    );
    assert_eq!(
        chart("cache3").0,
        ["n = 0", "n = 1", "n = 2", "n = 3"],
        "the mark was removed"
    );
    assert_eq!(
        chart("cache4"),
        (
            vec![
                "n = 0".to_owned(),
                "n = 2".to_owned(),
                "n = 1".to_owned(),
                "n = 3".to_owned()
            ],
            vec!["even".to_owned(), "odd".to_owned()]
        ),
        "a group mark was added to another `val`"
    );
    assert_eq!(
        chart("cache1"),
        (vec!["low".to_owned(), "high".to_owned()], vec![]),
        "back to the first text"
    );
    let verify = quint.earnest(&["verify", "cache"]);
    verify.exits(0);
    assert_eq!(
        fields(&verify.stdout).first().map(String::as_str),
        Some("pass"),
        "{verify}"
    );
}

#[test]
fn the_export_and_quints_own_run_carry_no_view() {
    let quint = Quint::new();
    let marked = quint.earnest(&["tla", "--out", "marked-tla", "labels"]);
    let bare = quint.earnest(&["tla", "--out", "bare-tla", "bare"]);
    marked.exits(0);
    bare.exits(0);
    let read = |folder: &str| {
        let mut files: Vec<(String, String)> = fs::read_dir(quint.sandbox.path().join(folder))
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
        files.sort();
        files
    };
    let exported = read("marked-tla");
    assert!(!exported.is_empty(), "{marked}");
    assert_eq!(exported, read("bare-tla"), "the TLA+ export is the same");
    let confirmed = quint.earnest(&["confirm", "--quint", "labels"]);
    let plain = quint.earnest(&["confirm", "--quint", "bare"]);
    assert_eq!(confirmed.stdout, plain.stdout, "{confirmed}\n{plain}");
    assert_eq!(confirmed.code, plain.code);
    assert!(
        confirmed.stdout.contains("same") && confirmed.stdout.contains("1000 states"),
        "{confirmed}"
    );
    let not_compiled: Vec<String> = quint
        .calls()
        .into_iter()
        .filter(|call| !call.starts_with("compile"))
        .filter(|call| call.contains("seen") || call.contains("retried"))
        .collect();
    assert!(
        not_compiled.is_empty(),
        "only the compile that earnest reads names a label: {not_compiled:?}"
    );
}

#[test]
fn a_label_keeps_every_number_of_the_compiled_module_as_quint_wrote_it() {
    let quint = Quint::new();
    let marked = quint.earnest(&["verify", "numbers", "--json"]);
    let bare = quint.earnest(&["verify", "numbers_bare", "--json"]);
    marked.exits(1);
    bare.exits(1);
    let keys = ["milliseconds", "cached", "file", "line"];
    assert_eq!(
        without(&marked.json(), &keys),
        without(&bare.json(), &keys),
        "the same verdicts and the same faults, as  and not a float"
    );
    let faults = |run: &Run| -> Vec<(String, String)> {
        problems(&run.json())
            .into_iter()
            .map(|(_, _, feature, message)| (feature, message))
            .collect()
    };
    assert!(
        faults(&marked).is_empty(),
        "a Quint `int` has no size, so `wider`'s literals past 64 bits are read: {marked}"
    );
    assert_eq!(faults(&marked), faults(&bare));
    let wider = quint.earnest(&["verify", "numbers", "--no-cache", "--model", "wider"]);
    assert!(
        wider.stdout.contains("broken  below")
            && wider.stdout.contains("start  m = 18446744073709551617"),
        "`m < 9223372036854775808` is false for m = 2^64 + 1, as `quint test` says\n{wider}"
    );
    let text = quint.earnest(&["verify", "numbers", "--no-cache"]);
    let plain = quint.earnest(&["verify", "numbers_bare", "--no-cache"]);
    assert_eq!(result_line(&text, "wide"), result_line(&plain, "wide"));
    assert_eq!(
        result_line(&text, "wide"),
        ["pass", "wide", "3 states", "1 rule, 0 runs"]
    );
    let chart = quint.earnest(&["chart", "numbers", "--model", "wide"]);
    assert_eq!(
        states_drawn(&chart.stdout).first().map(String::as_str),
        Some("#123; n#58; 9007199254740993, top#58; 9223372036854775807 #125;"),
        "{chart}"
    );
}

#[test]
fn a_def_with_no_parameters_labels_the_chart_whether_its_check_is_built_from_the_typecheck_or_compiled()
 {
    let quint = Quint::new();
    let marked = quint.earnest(&["verify", "defs", "--no-cache"]);
    marked.exits(0);
    assert!(
        quint.calls().is_empty(),
        "no `quint` is started: {:?}",
        quint.calls()
    );
    let bare = quint.earnest(&["verify", "defs_bare", "--no-cache"]);
    bare.exits(0);
    for name in ["Small", "plain"] {
        assert_eq!(
            result_line(&marked, name),
            result_line(&bare, name),
            "{marked}\n{bare}"
        );
    }
    assert_eq!(
        result_line(&marked, "Small"),
        ["pass", "Small", "4 states", "1 rule, 1 run"],
        "the spec's run is played"
    );
    let small = quint.earnest(&["chart", "defs", "--model", "Small"]);
    assert_eq!(states_drawn(&small.stdout), ["start", "running"], "{small}");
    let plain = quint.earnest(&["chart", "defs", "--model", "plain"]);
    assert_eq!(states_drawn(&plain.stdout), ["zero", "more"], "{plain}");
    assert_eq!(
        groups_drawn(&plain.stdout),
        ["counter"],
        "a `pure def` labels the box: {plain}"
    );
}

#[test]
fn a_label_leaves_a_setup_built_from_the_typecheck_and_its_imported_runs_played() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "imported"]);
    run.exits(0);
    assert_eq!(
        result_line(&run, "Small"),
        ["pass", "Small", "4 states", "1 rule, 1 run"]
    );
    assert_eq!(
        result_line(&run, "Named"),
        ["pass", "Named", "3 states", "2 rules, 1 run"]
    );
    assert!(
        quint.calls().is_empty(),
        "both setups, with their labels, are built from the typecheck, and so play the spec's \
         run, with no `quint` started: {:?}",
        quint.calls()
    );
}

#[test]
fn a_label_of_the_checks_own_module_wins_over_the_labels_it_imports() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "override", "--json"]);
    run.exits(1);
    let report = run.json();
    for name in ["Inherits", "Overrides", "Chooses", "Once"] {
        assert_eq!(model(&report, name)["status"], "pass", "{name}: {report}");
    }
    let chart = |name: &str| {
        let run = quint.earnest(&["chart", "override", "--model", name]);
        (states_drawn(&run.stdout), groups_drawn(&run.stdout))
    };
    assert_eq!(
        chart("Inherits"),
        (
            vec!["0".to_owned(), "1".to_owned(), "2".to_owned()],
            vec!["early".to_owned(), "late".to_owned()]
        ),
        "a setup with no label of its own takes the spec's"
    );
    assert_eq!(
        chart("Overrides"),
        (
            vec!["zero".to_owned(), "more".to_owned(), "more".to_owned()],
            vec!["early".to_owned(), "late".to_owned()]
        ),
        "its own state label wins over the spec's, and the spec's group is still used"
    );
    let (states, groups) = chart("Chooses");
    assert!(states.contains(&"(0, 2)".to_owned()), "{states:?}");
    assert_eq!(groups, ["level", "apart"], "its own labels choose");
    assert_eq!(
        chart("Once").0,
        ["none", "some"],
        "one module brought whole, under a name and exported is one label"
    );
    let found = problems(&report);
    let two: Vec<String> = found
        .iter()
        .filter(|each| each.1 == 35)
        .map(|each| format!("{} {}", each.2, each.3))
        .collect();
    assert_eq!(
        two,
        [
            "`module Two` the check `Two` sees 2 definitions marked `@view state` in the modules \
             it imports, `S::specView` and `T::specView`, and a chart takes one: mark a `val` of \
             `Two` itself `/// @view state` to choose its label, and the imported ones are not \
             used",
            "`module Two` the check `Two` sees 2 definitions marked `@view group` in the modules \
             it imports, `S::specGroup` and `T::specGroup`, and a chart takes one: mark a `val` \
             of `Two` itself `/// @view group` to choose its label, and the imported ones are not \
             used",
        ],
        "two instances of one spec bring two labels: {found:?}"
    );
    assert_eq!(
        problem_at(&found, 71),
        "`second` the check `TwoOwn` sees two definitions marked `@view state`, `first` and \
         `second`, and a chart takes one: keep the mark on one of them"
    );
    assert!(
        quint.calls().is_empty(),
        "no `quint` is started: {:?}",
        quint.calls()
    );
}

#[test]
fn a_view_mark_in_another_case_or_in_a_comment_quint_drops_is_unreadable_at_its_line() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "near", "--json"]);
    run.exits(1);
    let report = run.json();
    let found = problems(&report);
    assert_eq!(
        problem_at(&found, 11),
        "`@view` the doc comment holds `@View state`, which is not a mark: a mark is written in \
         small letters, `/// @view state`, or it is read as a sentence and the chart is not \
         labelled"
    );
    assert!(
        problem_at(&found, 14).contains("`@VIEW group`, which is not a mark"),
        "{found:?}"
    );
    assert_eq!(
        problem_at(&found, 17),
        "`@view` `// @view state` is a comment that `quint parse` does not hand over, so the \
         chart is not labelled: write the mark in a doc comment, with three slashes, `/// @view \
         state`, on the line above the `val`"
    );
    assert_eq!(
        problem_at(&found, 20),
        "`@view` `@view group` is in a `/* */` comment, which `quint parse` does not hand over, \
         so the chart is not labelled: write the mark in a doc comment, with three slashes, `/// \
         @view group`, on the line above the `val`"
    );
    assert!(
        problem_at(&found, 24).contains("`/// @view state`"),
        "{found:?}"
    );
    assert_eq!(
        problem_at(&found, 2),
        "`x` `x` is a `nondet` definition, which gives no value to label the chart with: mark a \
         `val`, or a `def` with no parameters, `/// @view state`"
    );
    assert_eq!(
        found.len(),
        6,
        "`@vew` is a sentence, and a comment with a blank line under it is above nothing: \
         {found:?}"
    );
    assert_eq!(model(&report, "near")["status"], "pass");
}

#[test]
fn a_label_quint_itself_refuses_is_named_in_quints_error() {
    let quint = Quint::new();
    let run = quint.earnest(&["verify", "quintrefuses", "--json"]);
    run.exits(1);
    let found = problems(&run.json());
    assert_eq!(
        found
            .iter()
            .map(|each| (each.1, each.2.clone(), each.3.clone()))
            .collect::<Vec<_>>(),
        [(
            12,
            "`quint compile --main refused`".to_owned(),
            "`quint compile --main refused` fails in the chart label `shown`: Error [QNT203]: \
             'oneOf' must be used inside a nondet definition"
                .to_owned()
        )]
    );
}

#[test]
fn the_invariant_a_label_is_compiled_into_is_one_quint_typecheck_accepts() {
    let read = |name: &str| -> Value {
        let path = views().join(format!("fixtures/typechecked/typed/{name}.json"));
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()));
        serde_json::from_str(&text).expect("quint typecheck wrote JSON")
    };
    let written = |name: &str| {
        fs::read_to_string(views().join(format!("typed/{name}.qnt")))
            .expect("the typed model can be read")
    };
    assert!(written("welltyped").contains(
        "val synthesized = and(shownWasAccepted, acceptedWasPlaced, seen == seen, retried == \
         retried)"
    ));
    assert_eq!(
        read("welltyped")["errors"],
        serde_json::json!([]),
        "quint 0.33.1 typechecks the invariants with each label as `name == name`"
    );
    assert!(
        written("illtyped")
            .contains("val synthesized = and(shownWasAccepted, acceptedWasPlaced, seen, retried)")
    );
    let refused = read("illtyped")["errors"][0]["explanation"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(
        refused.starts_with("[QNT000] Couldn't unify bool and str"),
        "a label written bare beside the invariants is ill-typed: {refused}"
    );
}
