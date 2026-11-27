#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::{Report, Sandbox, binary, rule};

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
    if [ -f "$here/parse.json" ]; then
      cp "$here/parse.json" "$out"
    else
      cp "$here/typecheck.json" "$out"
    fi
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

const FAKE_JAVA: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
previous=""
for argument in "$@"; do
  if [ "$previous" = "-config" ]; then config="$argument"; fi
  previous="$argument"
done
cat "$here/$(basename "$config" .cfg).tlc.txt"
"#;

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

fn faults(report: &Report, model: &str) -> Vec<String> {
    report.model(model)["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding["kind"] == "fault")
        .filter_map(|finding| finding["message"].as_str().map(str::to_owned))
        .collect()
}

fn program(folder: &Path, name: &str, text: &str) {
    let path = folder.join(name);
    fs::write(&path, text).expect("the fake program can be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .expect("the fake program can be made a program");
}

#[test]
fn a_lambda_inside_a_temporal_property_is_judged_as_tlc_judges_it() {
    let report = compiled(
        "temporal_lambda",
        include_str!("../../models/quint_second/temporal_lambda.json"),
    );
    let model = report.model("temporal_lambda");
    assert!(
        faults(&report, "temporal_lambda").is_empty() && rule(model, "small")["verdict"] == "holds",
        "`always(s.forall(e => e < 9))` holds in both states (`Set()` and `Set(5)`): `quint \
         verify --backend tlc --temporal small` prints `[ok] No violation found`; the same \
         condition as an invariant holds in earnest, so a lambda in a temporal property is no \
         fault\n{report}"
    );
}

#[test]
fn a_nondet_inside_a_match_branch_is_picked_only_where_the_branch_matches() {
    let report = compiled(
        "match_nondet",
        include_str!("../../models/quint_second/match_nondet.json"),
    );
    let model = report.model("match_nondet");
    assert!(
        faults(&report, "match_nondet").is_empty()
            && model["states"] == 4
            && rule(model, "small")["verdict"] == "holds",
        "`| B(s) => all {{ nondet w = s.oneOf() ... }}` picks from `s` only where `t` is a `B`: \
         from `t = A(1)` only the `A` branch can happen; `quint verify --backend tlc` finds 4 \
         distinct states and `[ok]` for `small`, and `quint run --backend typescript` finds no \
         violation\n{report}"
    );
}

#[test]
fn a_nondet_behind_a_guard_or_an_if_is_picked_only_where_the_guard_holds() {
    let report = compiled(
        "guarded_nondet",
        include_str!("../../models/quint_second/guarded_nondet.json"),
    );
    let model = report.model("guarded_nondet");
    assert!(
        faults(&report, "guarded_nondet").is_empty()
            && model["states"] == 3
            && rule(model, "small")["verdict"] == "holds",
        "`all {{ i < q.length(), nondet w = q[i].oneOf() ... }}` and `if (i < q.length()) \
         {{ nondet w = q[i].oneOf() ... }}` pick from `q[i]` only where `i` is in range: \
         `quint verify --backend tlc` finds 3 distinct states and `[ok]` for `small`, and \
         `quint run --backend typescript` finds no violation\n{report}"
    );
}

#[test]
fn a_nondet_in_a_match_branch_of_init_that_a_constant_rules_out_is_not_read() {
    let report = compiled(
        "init_match_nondet",
        include_str!("../../models/quint_second/init_match_nondet.json"),
    );
    let model = report.model("init_match_nondet");
    assert!(
        !report.is_unreadable()
            && model["states"] == 1
            && rule(model, "small")["verdict"] == "holds",
        "with `Mode = Fixed(1)`, `init`'s `Range(s)` branch never happens, so its `nondet v = \
         s.oneOf()` is not worked out: `quint verify --backend tlc --main init_match_nondet` \
         finds 1 distinct state and `[ok]`\n{report}"
    );
}

#[test]
fn a_fold_over_tuples_of_ten_items_takes_them_in_the_order_tlc_takes_them() {
    let report = compiled(
        "fold_tuple10",
        include_str!("../../models/quint_second/fold_tuple10.json"),
    );
    assert_eq!(
        rule(report.model("fold_tuple10"), "lastFirst")["verdict"],
        "holds",
        "TLC compares two tuples item by item from the first, so `(0, ..., 0, 1)` comes before \
         `(0, 1, 0, ..., 0)`: `quint verify --backend tlc --invariant lastFirst` and `quint \
         run --backend typescript` both find no violation (the Rust simulator finds one); the \
         docs promise the order TLC takes\n{report}"
    );
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

fn verified_with_recorded_quint(fixtures: &QuintFixtures<'_>) -> Value {
    RecordedQuint::new(fixtures).verify(&["--no-cache"])
}

fn model_named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == name))
        .unwrap_or(&Value::Null)
}

#[test]
fn the_runs_of_a_spec_imported_under_a_name_are_played_as_quint_test_plays_them() {
    let report = verified_with_recorded_quint(&QuintFixtures {
        file: "named_runs.qnt",
        source: include_str!("../../models/quint_second/named_runs/named_runs.qnt"),
        typecheck: include_str!("../../models/quint_second/named_runs/typecheck.json"),
        sourcemap: include_str!("../../models/quint_second/named_runs/named_runs.sourcemap.json"),
        compiled: include_str!("../../models/quint_second/named_runs/Named.json"),
    });
    assert_eq!(
        rule(model_named(&report, "Named"), "upTest")["verdict"],
        "holds",
        "`quint test --main Named` plays `Named::C::upTest` and prints `ok Named::C::upTest \
         passed 1 test(s)`; a check plays the spec's runs whose name ends \
         in `Test`, `C::name` under a name, and the same check beside a second check of the \
         file plays it\n{report:#}"
    );
}

#[test]
fn a_compiled_module_that_drops_an_imported_run_is_unfinished_and_names_it() {
    let fixtures = |typecheck| QuintFixtures {
        file: "runs_not_played.qnt",
        source: include_str!("../../models/quint_second/runs_not_played/runs_not_played.qnt"),
        typecheck,
        sourcemap: include_str!(
            "../../models/quint_second/runs_not_played/runs_not_played.sourcemap.json"
        ),
        compiled: include_str!("../../models/quint_second/runs_not_played/Plays.json"),
    };
    let built = RecordedQuint::new(&fixtures(include_str!(
        "../../models/quint_second/runs_not_played/typecheck.json"
    )))
    .verify(&["--no-cache"]);
    let plays = model_named(&built, "Plays");
    assert_eq!(
        (plays["status"].as_str(), run_ids(plays)),
        (
            Some("pass"),
            vec![
                ("upTest".to_owned(), "holds".to_owned()),
                ("ownTest".to_owned(), "holds".to_owned())
            ]
        ),
        "`quint test --main Plays` plays `S::upTest` and `ownTest`, and the check, built from \
         the typecheck, plays both\n{built:#}"
    );
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(
        folder.path().join("compiled.qnt"),
        include_str!("../../../quint/setups/compiled.qnt"),
    )
    .expect("the setups can be written");
    let verified = |arguments: &[&str]| {
        Command::new(binary())
            .args(["verify", "--no-cache"])
            .args(arguments)
            .arg("compiled.qnt")
            .env_remove("EARNEST_QUINT")
            .current_dir(folder.path())
            .output()
            .expect("earnest starts")
    };
    let report: Value =
        serde_json::from_slice(&verified(&["--json"]).stdout).expect("earnest verify prints JSON");
    let model = model_named(&report, "Single");
    assert_eq!(
        (
            model["status"].as_str(),
            &model["runsNotPlayed"],
            model["reason"].as_str()
        ),
        (
            Some("unfinished"),
            &serde_json::json!(["Single::Base::upTest"]),
            Some(
                "runs not played: Single::Base::upTest: the checker does not build `import \
                 Limit.L` at `compiled.qnt:32` from the file typechecked as Quint"
            )
        ),
        "`Single` imports one name alone, a shape the checker leaves to the compile, which \
         drops `Base`'s `upTest`; `quint test --main Single` plays it, so the check does not \
         pass: it says which run it did not play and what kept it from building the check\n\
         {report:#}"
    );
    assert_eq!(
        (
            model["states"].as_u64(),
            model["rules"][0]["verdict"].as_str()
        ),
        (Some(3), Some("holds")),
        "the check's property is still judged\n{report:#}"
    );
    let renamed = model_named(&report, "Renamed");
    assert_eq!(
        (renamed["status"].as_str(), run_ids(renamed)),
        (
            Some("pass"),
            vec![("upTest".to_owned(), "holds".to_owned())]
        ),
        "beside it, `Renamed` is a shape the checker builds, and it plays the same run\n\
         {report:#}"
    );
    let text = String::from_utf8_lossy(&verified(&[]).stdout).into_owned();
    assert!(
        text.lines()
            .any(|line| line.trim_start().starts_with("note")
                && line.contains("Single::Base::upTest")),
        "the text output names the run under the result line\n{text}"
    );
    let direct = compiled(
        "Named",
        include_str!("../../models/quint_second/named_runs/Named.json"),
    );
    assert!(
        !direct.is_unreadable() && direct.model("Named")["runsNotPlayed"].is_null(),
        "a compiled file read on its own plays the runs it holds, with no note\n{direct}"
    );
}

fn run_ids(model: &Value) -> Vec<(String, String)> {
    model["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|found| {
            found["says"]
                .as_str()
                .is_some_and(|says| says.contains("Quint run"))
        })
        .map(|found| {
            (
                found["id"].as_str().unwrap_or_default().to_owned(),
                found["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn confirmed_with_tlc_as_it_prints_today(
    name: &str,
    json: &str,
    outputs: &[(&str, &str)],
) -> String {
    let folder = TempDir::new().expect("a temporary folder can be made");
    let fake = folder.path().join("fake");
    fs::create_dir_all(&fake).expect("the fake's folder can be made");
    program(&fake, "java", FAKE_JAVA);
    for (config, text) in outputs {
        fs::write(fake.join(format!("{config}.tlc.txt")), text)
            .expect("a TLC output can be written");
    }
    let jar = fake.join("tla2tools.jar");
    fs::write(&jar, "").expect("the jar's place can be written");
    let file = format!("{name}.json");
    fs::write(folder.path().join(&file), json).expect("the module can be written");
    let output = Command::new(binary())
        .arg("confirm")
        .arg("--tlc")
        .arg(&jar)
        .arg(&file)
        .env("PATH", format!("{}:/usr/bin:/bin", fake.display()))
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn confirm_reads_a_temporal_property_that_tlc_names_as_violated_as_broken() {
    let said = confirmed_with_tlc_as_it_prints_today(
        "broken_eventually",
        include_str!("../../models/quint_second/confirm/broken_eventually.json"),
        &[
            (
                "count",
                include_str!("../../models/quint_second/confirm/broken_eventually.count.tlc.txt"),
            ),
            (
                "eventually0",
                include_str!(
                    "../../models/quint_second/confirm/broken_eventually.eventually0.tlc.txt"
                ),
            ),
        ],
    );
    assert!(
        said.starts_with("same "),
        "TLC 2026.11.26 (the `tla2tools.jar` of the v1.8.0 release and the TLC in Apalache \
         0.62.4's jar) prints `Error: Temporal property Eventually_finish was violated.` for \
         the export of `eventually(done)` with no fairness, which earnest judges broken: TLC \
         agrees, so confirm says `same`\n{said}"
    );
}

#[test]
fn confirm_reads_an_invariant_that_tlc_finds_false_without_a_state_as_broken() {
    let said = confirmed_with_tlc_as_it_prints_today(
        "constant_false",
        include_str!("../../models/quint_second/confirm/constant_false.json"),
        &[
            (
                "count",
                include_str!("../../models/quint_second/confirm/constant_false.count.tlc.txt"),
            ),
            (
                "rule0",
                include_str!("../../models/quint_second/confirm/constant_false.rule0.tlc.txt"),
            ),
        ],
    );
    assert!(
        said.starts_with("same "),
        "`val never = Set(1, 2).size() == 3` reads no variable, so TLC judges it before the \
         search and prints `Error: The invariant of Rule_never is equal to FALSE`: the rule is \
         false, as earnest says, so confirm says `same`\n{said}"
    );
}

const SEEDED: [(&str, &str, &str, &str); 10] = [
    (
        "H14",
        include_str!("../../models/quint_second/random/H14.qnt"),
        include_str!("../../models/quint_second/random/H14.typecheck.json"),
        include_str!("../../models/quint_second/random/H14.sourcemap.json"),
    ),
    (
        "H25",
        include_str!("../../models/quint_second/random/H25.qnt"),
        include_str!("../../models/quint_second/random/H25.typecheck.json"),
        include_str!("../../models/quint_second/random/H25.sourcemap.json"),
    ),
    (
        "H37",
        include_str!("../../models/quint_second/random/H37.qnt"),
        include_str!("../../models/quint_second/random/H37.typecheck.json"),
        include_str!("../../models/quint_second/random/H37.sourcemap.json"),
    ),
    (
        "H45",
        include_str!("../../models/quint_second/random/H45.qnt"),
        include_str!("../../models/quint_second/random/H45.typecheck.json"),
        include_str!("../../models/quint_second/random/H45.sourcemap.json"),
    ),
    (
        "H54",
        include_str!("../../models/quint_second/random/H54.qnt"),
        include_str!("../../models/quint_second/random/H54.typecheck.json"),
        include_str!("../../models/quint_second/random/H54.sourcemap.json"),
    ),
    (
        "H63",
        include_str!("../../models/quint_second/random/H63.qnt"),
        include_str!("../../models/quint_second/random/H63.typecheck.json"),
        include_str!("../../models/quint_second/random/H63.sourcemap.json"),
    ),
    (
        "H1002",
        include_str!("../../models/quint_second/random/H1002.qnt"),
        include_str!("../../models/quint_second/random/H1002.typecheck.json"),
        include_str!("../../models/quint_second/random/H1002.sourcemap.json"),
    ),
    (
        "H1006",
        include_str!("../../models/quint_second/random/H1006.qnt"),
        include_str!("../../models/quint_second/random/H1006.typecheck.json"),
        include_str!("../../models/quint_second/random/H1006.sourcemap.json"),
    ),
    (
        "H1010",
        include_str!("../../models/quint_second/random/H1010.qnt"),
        include_str!("../../models/quint_second/random/H1010.typecheck.json"),
        include_str!("../../models/quint_second/random/H1010.sourcemap.json"),
    ),
    (
        "H1013",
        include_str!("../../models/quint_second/random/H1013.qnt"),
        include_str!("../../models/quint_second/random/H1013.typecheck.json"),
        include_str!("../../models/quint_second/random/H1013.sourcemap.json"),
    ),
];

const SEEDED_EXPECTED: &str = include_str!("../../models/quint_second/random/expected.json");

#[test]
fn every_seeded_setup_has_the_states_and_verdicts_of_quint_s_tlc_run_and_quint_test() {
    let expected: Value =
        serde_json::from_str(SEEDED_EXPECTED).expect("the expected answers are JSON");
    let mut wrong = Vec::new();
    for (name, source, typecheck, sourcemap) in SEEDED {
        let answers = &expected[name];
        let file = format!("{name}.qnt");
        let report = verified_with_recorded_quint(&QuintFixtures {
            file: &file,
            source,
            typecheck,
            sourcemap,
            compiled: "",
        });
        let model = model_named(&report, name);
        if model["states"] != answers["states"] {
            wrong.push(format!(
                "{name}: {} states, Quint's TLC run finds {}",
                model["states"], answers["states"]
            ));
        }
        for kind in ["verdicts", "runs"] {
            for (id, verdict) in answers[kind].as_object().into_iter().flatten() {
                let found = &rule(model, id)["verdict"];
                if found != verdict {
                    wrong.push(format!("{name}: `{id}` is {found}, Quint says {verdict}"));
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "each seeded setup checks as `quint verify --backend tlc` (states, invariants, temporal \
         properties) and `quint test` (runs) judge it:\n{}",
        wrong.join("\n")
    );
}
