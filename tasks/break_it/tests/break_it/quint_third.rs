#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Report, Sandbox, binary};

const FAKE_QUINT: &str = r##"#!/bin/sh
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
    sed "s#\"0\":\"#\"0\":\"$folder/#" "$here/sourcemap.json" > "$map"
    ;;
  typecheck)
    cp "$here/typecheck.json" "$out"
    ;;
  compile)
    cat "$here/compiled.json"
    ;;
esac
"##;

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

fn program(folder: &Path, name: &str, text: &str) {
    let path = folder.join(name);
    fs::write(&path, text).expect("the fake program can be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
}

struct QuintFixtures<'a> {
    file: &'a str,
    source: &'a str,
    typecheck: &'a str,
    sourcemap: &'a str,
    compiled: &'a str,
}

struct RecordedQuint {
    folder: TempDir,
    quint: PathBuf,
}

impl RecordedQuint {
    fn new(fixtures: &QuintFixtures<'_>) -> Self {
        let folder = TempDir::new().expect("a temporary folder can be made");
        let fake = folder.path().join("fake");
        fs::create_dir_all(&fake).expect("the fake's folder can be made");
        program(&fake, "quint", FAKE_QUINT);
        for (name, text) in [
            ("typecheck.json", fixtures.typecheck),
            ("sourcemap.json", fixtures.sourcemap),
            ("compiled.json", fixtures.compiled),
        ] {
            fs::write(fake.join(name), text).expect("a fixture can be written");
        }
        let recorded = Self {
            folder,
            quint: fake.join("quint"),
        };
        recorded.write(fixtures.file, fixtures.source);
        recorded
    }

    fn write(&self, name: &str, text: &str) {
        fs::write(self.folder.path().join(name), text).expect("a file can be written");
    }

    fn verify(&self, arguments: &[&str]) -> Value {
        let output = Command::new(binary())
            .args(["verify", "--json"])
            .args(arguments)
            .arg(".")
            .env("EARNEST_QUINT", &self.quint)
            .current_dir(self.folder.path())
            .output()
            .expect("earnest starts");
        serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON")
    }
}

fn model_named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == name))
        .unwrap_or(&Value::Null)
}

#[test]
fn a_temporal_property_that_is_a_forall_over_an_empty_set_is_still_a_rule() {
    let report = compiled(
        "empty_forall",
        include_str!("../../models/quint_third/empty_forall.json"),
    );
    if report.is_unreadable() {
        return;
    }
    let rules = report.model("empty_forall")["rules"].clone();
    let listed = rules
        .as_array()
        .into_iter()
        .flatten()
        .find(|found| found["id"] == "everyonePays");
    assert_eq!(
        listed.map(|found| found["verdict"].clone()),
        Some(Value::from("holds")),
        "`everyonePays` is `Buyers.forall(b => eventually(...))` over the empty set: an `and` \
         of no parts, true in every run. The check lists it, so it is a rule that holds (or \
         the file is refused naming it); it is never left out of the result, where `--rule \
         everyonePays` then says the model has no such rule and `expectCounterexample` that the \
         module has no such property\n{report}"
    );
}

#[test]
fn the_note_of_runs_not_played_names_only_runs_quint_test_plays() {
    let recorded = RecordedQuint::new(&QuintFixtures {
        file: "one_name.qnt",
        source: include_str!("../../models/quint_third/one_name/one_name.qnt"),
        typecheck: include_str!("../../models/quint_third/one_name/typecheck.json"),
        sourcemap: include_str!("../../models/quint_third/one_name/sourcemap.json"),
        compiled: include_str!("../../models/quint_third/one_name/compiled.json"),
    });
    let report = recorded.verify(&["--no-cache"]);
    let model = model_named(&report, "OneName");
    let named: Vec<&str> = model["runsNotPlayed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        !named.contains(&"specOverTest"),
        "`OneName` imports `Spec.specTest` alone (with `init`, `step` and `v`), so `quint test \
         --main OneName` plays `specTest` and `ownTest` and never `specOverTest`; the note says \
         it did not play `specOverTest`, which no Quint command plays here\n{report:#}"
    );
}

const SEEDED: [(&str, &str); 10] = [
    (
        "R6",
        include_str!("../../models/quint_third/random/R6.json"),
    ),
    (
        "R9",
        include_str!("../../models/quint_third/random/R9.json"),
    ),
    (
        "R13",
        include_str!("../../models/quint_third/random/R13.json"),
    ),
    (
        "R14",
        include_str!("../../models/quint_third/random/R14.json"),
    ),
    (
        "R15",
        include_str!("../../models/quint_third/random/R15.json"),
    ),
    (
        "R16",
        include_str!("../../models/quint_third/random/R16.json"),
    ),
    (
        "R25",
        include_str!("../../models/quint_third/random/R25.json"),
    ),
    (
        "R30",
        include_str!("../../models/quint_third/random/R30.json"),
    ),
    (
        "R35",
        include_str!("../../models/quint_third/random/R35.json"),
    ),
    (
        "R36",
        include_str!("../../models/quint_third/random/R36.json"),
    ),
];

fn whole_verdicts(model: &Value) -> serde_json::Map<String, Value> {
    let mut verdicts = serde_json::Map::new();
    for found in model["rules"].as_array().into_iter().flatten() {
        let id = found["id"].as_str().unwrap_or_default();
        let base = id.split('#').next().unwrap_or(id).to_owned();
        let verdict = found["verdict"].as_str().unwrap_or_default();
        let earlier = verdicts.get(&base).and_then(Value::as_str);
        let joined = [Some("broken"), Some("unknown")]
            .into_iter()
            .flatten()
            .find(|worst| earlier == Some(*worst) || verdict == *worst)
            .unwrap_or(verdict)
            .to_owned();
        verdicts.insert(base, Value::from(joined));
    }
    verdicts
}

#[test]
fn every_seeded_model_of_buyers_has_the_states_and_verdicts_of_quint_s_tlc_run_and_quint_test() {
    let expected: Value = serde_json::from_str(include_str!(
        "../../models/quint_third/random/expected.json"
    ))
    .expect("the expected answers are JSON");
    for (name, json) in SEEDED {
        for jobs in ["1", "4"] {
            let file = format!("{name}.json");
            let report = Sandbox::with(&[(file.as_str(), json)]).verify(&[&file, "--jobs", jobs]);
            let model = report.model(name);
            let wanted = &expected[name];
            assert_eq!(
                (&model["states"], Value::Object(whole_verdicts(model))),
                (&wanted["states"], wanted["verdicts"].clone()),
                "`{name}` with {jobs} jobs: the states `quint verify --backend tlc` counts, \
                 each invariant and temporal property as TLC judges it (a property in parts \
                 broken when one part is), and each run as `quint test` judges it\n{report}"
            );
        }
    }
}

fn verdicts_of(model: &Value) -> Vec<(String, String)> {
    model["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|found| {
            (
                found["id"].as_str().unwrap_or_default().to_owned(),
                found["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn fairness_of_an_action_given_a_part_of_a_picked_tuple_or_record_is_judged_as_tlc_judges_it() {
    let holds = |id: &str| (id.to_owned(), "holds".to_owned());
    let broken = |id: &str| (id.to_owned(), "broken".to_owned());
    let cases = [
        (
            include_str!("../../models/quint_third/fair_parts/fair_parts_pair.json"),
            4,
            vec![
                holds("payEach"),
                holds("payEach#2"),
                holds("payOne"),
                broken("payOne#2"),
                holds("payOneOnly"),
            ],
        ),
        (
            include_str!("../../models/quint_third/fair_parts/fair_parts_record.json"),
            4,
            vec![
                holds("payEach"),
                holds("payEach#2"),
                holds("payOne"),
                broken("payOne#2"),
                holds("payOneOnly"),
            ],
        ),
        (
            include_str!("../../models/quint_third/fair_parts/fair_parts_both.json"),
            7,
            vec![
                holds("allPay"),
                holds("allPay#2"),
                holds("onePays"),
                broken("onePays#2"),
                holds("oneOnly"),
            ],
        ),
    ];
    for (json, states, expected) in cases {
        let report = compiled("fair_parts", json);
        let model = report.model("fair_parts");
        let mut found = verdicts_of(model);
        found.sort();
        let mut wanted = expected;
        wanted.sort();
        assert_eq!(
            (model["states"].as_u64(), found),
            (Some(states), wanted),
            "`pay(p._1)` with `nondet p` over `tuples(BUYERS, ITEMS)`, `pay(r.who)` over a set \
             of records and `buy(p._1, p._2)`: `weakFair(pay(1), paid)` is fair to the steps \
             whose pick has `1` there, so buyer 2 may never pay; `quint verify --backend tlc` \
             gives these states, `[ok]` for `payEach`, `payOneOnly`, `allPay` and `oneOnly`, \
             and a violation for `payOne` and `onePays`\n{report}"
        );
    }
}

#[test]
fn a_refusal_writes_the_expression_as_quint_does_names_the_caller_and_says_what_to_write() {
    let report = compiled(
        "messages",
        include_str!("../../models/quint_third/messages/messages.json"),
    );
    let messages: Vec<&str> = report.json["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|problem| problem["message"].as_str())
        .collect();
    let wanted = [
        "fairness over `t._2`",
        "`chooseSome`, whose choice Quint's backends make differently, is not read: pick every \
         item with `nondet x = S.oneOf()`, or write `S.getOnlyElement()` for a set of one, in \
         the definition `step`",
        "`x == 0` before `implies` is judged in the start states only, as TLC judges it, and it \
         holds in some start states and not in others",
        "write `always((x == 0) implies eventually(…))`",
    ];
    for part in wanted {
        assert!(
            messages.iter().any(|message| message.contains(part)),
            "`weakFair(step, t._2)` is shown as written, the `chooseSome` in the action given \
             to `wrap` in `step` is in `step`, and `(x == 0).implies(eventually(x == 3))` over \
             start states where `x` is 0 and 1 says what to write: no message has {part:?}\n\
             {report}"
        );
    }
}

#[test]
fn a_condition_implying_a_temporal_formula_is_judged_in_the_start_states_as_tlc_judges_it() {
    let report = compiled(
        "start_implies",
        include_str!("../../models/quint_third/start_implies/start_implies.json"),
    );
    let mut found = verdicts_of(report.model("start_implies"));
    found.sort();
    let expected: Vec<(String, String)> = [
        ("fairFromZero", "holds"),
        ("fromOne", "holds"),
        ("fromZero", "broken"),
    ]
    .iter()
    .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
    .collect();
    assert_eq!(
        found, expected,
        "`x` starts at 0: `(x == 0).implies(eventually(x == 3))` is broken by a run that stays, \
         holds under `weakFair(step, x)`, and `(x == 1).implies(…)` holds for no start state \
         has `x == 1`, as `quint verify --backend tlc` and Apalache judge them\n{report}"
    );
}

#[test]
fn a_step_shows_its_written_argument_a_clashing_run_keeps_its_prefix_and_rule_plays_a_run_alone() {
    let recorded = RecordedQuint::new(&QuintFixtures {
        file: "output.qnt",
        source: include_str!("../../models/quint_third/output/output.qnt"),
        typecheck: include_str!("../../models/quint_third/output/typecheck.json"),
        sourcemap: include_str!("../../models/quint_third/output/sourcemap.json"),
        compiled: "{}",
    });
    let report = recorded.verify(&["--no-cache", "--max-states", "1000", "--model", "Own"]);
    let rules = model_named(&report, "Own")["rules"].clone();
    let ids: Vec<&str> = rules
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|found| found["id"].as_str())
        .collect();
    let steps: Vec<&str> = rules[0]["path"]["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|step| step["action"].as_str())
        .collect();
    assert_eq!(
        (ids, steps),
        (vec!["M::aTest", "aTest"], vec!["add  k = 2"]),
        "`Own` has its own `aTest` beside the named `M::aTest`, so the played run keeps the \
         prefix that tells them apart, and its step `add(N)` with `N = 2` shows the value\n\
         {report:#}"
    );
    let alone = recorded.verify(&[
        "--no-cache",
        "--max-states",
        "1000",
        "--model",
        "endless",
        "--rule",
        "aTest",
    ]);
    let endless = model_named(&alone, "endless");
    assert_eq!(
        (&endless["status"], &endless["states"]),
        (&Value::from("pass"), &Value::from(1)),
        "`--rule aTest` judges a run alone, so the model, which has no end, is not searched: \
         only its start state is counted, and the run passes as `quint test` says\n{alone:#}"
    );
}

#[test]
fn a_variable_quint_compile_writes_twice_is_refused_naming_quint_s_compile() {
    let report = compiled(
        "TwiceImported",
        include_str!("../../models/quint_third/export_twice/TwiceImported.json"),
    );
    let messages: Vec<&str> = report.json["unreadable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|problem| problem["message"].as_str())
        .collect();
    assert!(
        messages.iter().any(|message| message.contains(
            "`quint compile --main TwiceImported` writes this variable twice under one id, as \
             `A::S::v` and `A::v`"
        ) && message
            .contains("put the two modules in separate files")),
        "`quint compile` writes `v` twice under the id 114 when another module of the file \
         exports `Spec`; the refusal says it is Quint's compile and what to do\n{report}"
    );
}
