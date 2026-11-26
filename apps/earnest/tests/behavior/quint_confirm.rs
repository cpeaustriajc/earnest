#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::support::{Run, Sandbox, repository};

const FAKE: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
main=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "--main" ]; then main="$argument"; fi
  previous="$argument"
done
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  compile)
    cat "$here/compiled/$main.json"
    ;;
  parse)
    for argument in "$@"; do
      if [ "$previous" = "--out" ]; then out="$argument"; fi
      if [ "$previous" = "--source-map" ]; then map="$argument"; fi
      previous="$argument"
    done
    echo '{"modules":[]}' > "$out"
    echo '{"sourceIndex":{},"map":{}}' > "$map"
    ;;
  verify)
    temporal=""
    invariants=""
    listing=""
    previous=""
    for argument in "$@"; do
      case "$argument" in --*) listing="" ;; esac
      if [ -n "$listing" ]; then invariants="$invariants+$argument"; fi
      if [ "$previous" = "--temporal" ]; then temporal="@$argument"; fi
      if [ "$argument" = "--invariants" ]; then listing="yes"; fi
      previous="$argument"
    done
    answer="$here/verify/$main$invariants$temporal"
    if [ ! -f "$answer" ]; then
      echo "error: the fake has no answer $answer" >&2
      exit 2
    fi
    cat "$answer"
    if [ -f "$answer.stderr" ]; then cat "$answer.stderr" >&2; fi
    if grep -q '^\[ok\]' "$answer"; then exit 0; fi
    exit 1
    ;;
  test)
    cat "$here/test/$main"
    if grep -q 'failed after' "$here/test/$main"; then exit 1; fi
    ;;
esac
"#;

const CHECKS: &str = r#"{ "checks": [
  { "file": "confirm.qnt", "main": "lift", "invariants": ["inside"], "temporal": ["reachesTop"], "runs": true, "expectCounterexample": ["low"] },
  { "file": "confirm.qnt", "main": "big", "invariants": ["belowBig"] },
  { "file": "confirm.qnt", "main": "halves", "invariants": ["halfAbove"] },
  { "file": "confirm.qnt", "main": "ranges", "runs": true },
  { "file": "confirm.qnt", "main": "words", "invariants": ["counted"] }
] }"#;

const RUNS_PLAYED: &str = "\n  lift\n    ok upTest passed 1 test(s)\n    1) pickTest failed after 3 test(s)\n\n  1 passing (5ms)\n  1 failed\nerror: Tests failed\n";

fn holds(states: u64) -> String {
    format!(
        "[TLC] Running TLC model checker...\n\
         Model checking completed. No error has been found.\n\
         {} states generated, {states} distinct states found, 0 states left on queue.\n\
         Finished in 00s at (2026-11-26 10:00:00)\n\n\
         [ok] No violation found (692ms).\n",
        states.saturating_mul(2)
    )
}

fn violated(states: u64) -> String {
    format!(
        "[TLC] Running TLC model checker...\n\
         Error: Invariant q_inv is violated.\n\
         {states} states generated, {states} distinct states found, 1 states left on queue.\n\n\
         [violation] Found an issue (812ms).\nerror: found a counterexample\n"
    )
}

struct Quint {
    sandbox: Sandbox,
}

impl Quint {
    fn new() -> Self {
        let sandbox = Sandbox::empty();
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(
            sandbox.path().join("fake/quint"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the fake can be made a program");
        let samples = repository().join("tasks/quint/confirm");
        let read = |path: PathBuf| {
            fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
        };
        sandbox.write("specs/confirm.qnt", &read(samples.join("confirm.qnt")));
        sandbox.write("specs/earnest.quint.json", CHECKS);
        for main in ["lift", "big", "halves", "ranges", "words"] {
            sandbox.write(
                &format!("fake/compiled/{main}.json"),
                &read(samples.join(format!("fixtures/{main}.json"))),
            );
        }
        sandbox.write("fake/calls", "");
        sandbox.write("java/java", "#!/bin/sh\nexit 0\n");
        fs::set_permissions(
            sandbox.path().join("java/java"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the stand-in for Java can be made a program");
        fs::create_dir_all(sandbox.path().join("no-programs")).expect("an empty folder");
        let quint = Self { sandbox };
        quint.answer("lift+inside@reachesTop", &violated(3));
        quint.answer("lift+inside", &holds(4));
        quint.answer("lift@reachesTop", &violated(4));
        quint.answer("lift+low", &violated(3));
        quint.answer("big+belowBig", &holds(3));
        quint.answer("halves+halfAbove", &holds(4));
        quint.sandbox.write("fake/test/lift", RUNS_PLAYED);
        quint.sandbox.write(
            "fake/test/ranges",
            "\n  ranges\n    1) emptyTest failed after 1 test(s)\n\n  1 failed\nerror: Tests failed\n",
        );
        quint
    }

    fn answer(&self, key: &str, said: &str) {
        self.sandbox.write(&format!("fake/verify/{key}"), said);
    }

    fn run(&self, path: &Path, model: &str) -> Run {
        Run::of(
            self.sandbox
                .command()
                .env("EARNEST_QUINT", self.sandbox.path().join("fake/quint"))
                .env("PATH", path)
                .args(["confirm", "--quint", "specs", "--model", model]),
        )
    }

    fn confirm(&self, model: &str) -> Run {
        self.run(&self.sandbox.path().join("java"), model)
    }

    fn without_java(&self, model: &str) -> Run {
        self.run(&self.sandbox.path().join("no-programs"), model)
    }

    fn calls(&self, command: &str) -> Vec<String> {
        let file = self
            .sandbox
            .path()
            .canonicalize()
            .expect("the sandbox has a real path")
            .join("specs/confirm.qnt");
        self.sandbox
            .read("fake/calls")
            .lines()
            .filter(|call| call.starts_with(&format!("{command} ")))
            .map(|call| call.replace(&format!(" {}", file.display()), ""))
            .collect()
    }
}

#[test]
fn a_check_whose_properties_all_hold_is_the_same_after_one_verify_job() {
    let quint = Quint::new();
    let run = quint.confirm("big");
    run.exits(0);
    assert_eq!(
        run.stdout.trim_end(),
        "same     big  3 states  pass  (Quint)",
        "{run}"
    );
    assert_eq!(
        quint.calls("verify"),
        ["verify --main big --backend tlc --init init --step step --invariants belowBig"],
        "one job gives the count and every verdict"
    );
    assert!(quint.calls("test").is_empty(), "the check plays no runs");
}

#[test]
fn each_property_gets_a_job_of_its_own_only_when_the_job_of_all_finds_a_violation() {
    let quint = Quint::new();
    let run = quint.confirm("lift");
    run.exits(0);
    assert_eq!(
        run.stdout.trim_end(),
        "same     lift  4 states  fail  (Quint)",
        "{run}"
    );
    assert_eq!(
        quint.calls("verify"),
        [
            "verify --main lift --backend tlc --init init --step step --temporal reachesTop \
             --invariants inside",
            "verify --main lift --backend tlc --init init --step step --invariants inside",
            "verify --main lift --backend tlc --init init --step step --temporal reachesTop",
            "verify --main lift --backend tlc --init init --step step --invariants low",
        ],
        "all together, then each, and the expected counterexample alone from the start"
    );
    assert_eq!(
        quint.calls("test"),
        ["test --main lift --backend typescript --match Test$"],
        "the runs whose name ends in `Test`, as `quint test` picks them"
    );
}

#[test]
fn an_expected_counterexample_is_the_same_when_quint_finds_a_violation_and_differs_when_not() {
    let quint = Quint::new();
    quint.confirm("lift").exits(0);
    quint.answer("lift+low", &holds(4));
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "differs  lift  4 states  fail  (Quint)",
            "  property low: the checker says broken, `quint verify` says holds",
        ],
        "{run}"
    );
}

#[test]
fn another_verdict_or_count_from_quint_verify_differs() {
    let quint = Quint::new();
    quint.answer("lift@reachesTop", &holds(5));
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "differs  lift  4 states  fail  (Quint)",
            "  states: the checker found 4, `quint verify` found 5",
            "  property reachesTop: the checker says broken, `quint verify` says holds",
        ],
        "{run}"
    );
}

#[test]
fn a_compiled_model_changed_by_hand_differs_from_what_quint_reads_in_the_qnt_file() {
    let quint = Quint::new();
    quint.confirm("lift").exits(0);
    let cache = quint.sandbox.path().join(".earnest/quint");
    let compiled: Vec<PathBuf> = fs::read_dir(&cache)
        .expect("the compiled JSON is cached")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|text| text.contains("\"name\":\"lift\""))
        })
        .collect();
    assert_eq!(
        compiled.len(),
        1,
        "one compiled JSON of `lift`: {compiled:?}"
    );
    let text = fs::read_to_string(&compiled[0]).expect("the cache can be read");
    let top = r#"{"id":8,"kind":"int","value":3}"#;
    assert!(text.contains(top), "the bound of `up` is in the JSON");
    fs::write(
        &compiled[0],
        text.replace(top, r#"{"id":8,"kind":"int","value":5}"#),
    )
    .expect("the cache can be written");
    let run = quint.confirm("lift");
    run.exits(1);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("differs  lift  6 states  fail  (Quint)"),
        "{run}"
    );
    assert!(
        lines.contains(&"  states: the checker found 6, `quint verify` found 4"),
        "{run}"
    );
}

#[test]
fn a_number_beyond_32_bits_makes_another_answer_of_quint_verify_cannot() {
    let quint = Quint::new();
    quint.answer("big+belowBig", &violated(1));
    quint.answer("big", &holds(1));
    let run = quint.confirm("big");
    run.exits(1);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "cannot   big  3 states  pass  (Quint)",
            "  states: the checker found 3, `quint verify` found 1",
            "    Quint can't confirm this: the model declares the number 4294967296, and TLC's \
             integers hold 32 bits, from -2147483648 to 2147483647",
            "  property belowBig: the checker says holds, `quint verify` says broken",
            "    Quint can't confirm this: the model declares the number 4294967296, and TLC's \
             integers hold 32 bits, from -2147483648 to 2147483647",
        ],
        "{run}"
    );
}

#[test]
fn a_division_of_a_negative_number_the_checker_could_not_judge_cannot_be_confirmed() {
    let quint = Quint::new();
    let run = quint.confirm("halves");
    run.exits(1);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines,
        [
            "cannot   halves  4 states  fail  (Quint)",
            "  property halfAbove: the checker says unknown, `quint verify` says holds",
            "    Quint can't confirm this: the checker could not judge it",
        ],
        "{run}"
    );
}

#[test]
fn a_run_quint_test_samples_and_passes_cannot_be_confirmed_and_one_it_plays_once_differs() {
    let quint = Quint::new();
    let sampled = RUNS_PLAYED.replace(
        "1) pickTest failed after 3 test(s)",
        "ok pickTest passed 10000 test(s)",
    );
    quint.sandbox.write("fake/test/lift", &sampled);
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   lift  4 states  fail  (Quint)",
            "  run pickTest: the checker says broken, `quint test` says holds after 10000 \
             sample(s)",
            "    Quint can't confirm this: `quint test` tries random choices of a run, and the \
             checker tries every choice",
        ],
        "{run}"
    );
    let once = RUNS_PLAYED.replace(
        "1) pickTest failed after 3 test(s)",
        "ok pickTest passed 1 test(s)",
    );
    quint.sandbox.write("fake/test/lift", &once);
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().take(2).collect::<Vec<_>>(),
        [
            "differs  lift  4 states  fail  (Quint)",
            "  run pickTest: the checker says broken, `quint test` says holds after 1 sample(s)",
        ],
        "a run with no choice to sample is played the same by both\n{run}"
    );
}

#[test]
fn a_run_quint_test_breaks_on_a_range_that_starts_above_its_end_cannot_be_confirmed() {
    let quint = Quint::new();
    let run = quint.confirm("ranges");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   ranges  0 states  pass  (Quint)",
            "  run emptyTest: the checker says holds, `quint test` says broken after 1 sample(s)",
            "    Quint can't confirm this: the check's runs use `range`, which `quint test` \
             answers otherwise than Quint's TLC run and the checker when it starts above its end",
        ],
        "{run}"
    );
    assert!(
        quint.calls("verify").is_empty(),
        "a check of runs only searches nothing"
    );
}

#[test]
fn a_fold_over_texts_that_the_check_reaches_makes_another_answer_of_quint_verify_cannot() {
    let quint = Quint::new();
    quint.answer("words+counted", &holds(4));
    let run = quint.confirm("words");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   words  3 states  pass  (Quint)",
            "  states: the checker found 3, `quint verify` found 4",
            "    Quint can't confirm this: the check folds a set of items of the kind `str`, \
             which TLC may take in another order than the checker",
        ],
        "{run}"
    );
}

#[test]
fn a_run_only_quint_test_plays_differs() {
    let quint = Quint::new();
    quint.sandbox.write(
        "fake/test/lift",
        &RUNS_PLAYED.replace(
            "    ok upTest",
            "    ok downTest passed 1 test(s)\n    ok upTest",
        ),
    );
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "differs  lift  4 states  fail  (Quint)",
            "  run downTest: `quint test` played it, and the checker did not",
        ],
        "{run}"
    );
}

#[test]
fn a_run_quint_test_also_plays_under_a_second_name_cannot_be_confirmed() {
    let quint = Quint::new();
    quint.sandbox.write(
        "fake/test/lift",
        &RUNS_PLAYED.replace(
            "    ok upTest",
            "    ok lift::B::upTest passed 1 test(s)\n    ok upTest",
        ),
    );
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   lift  4 states  fail  (Quint)",
            "  run lift::B::upTest: `quint test` played it, and the checker did not",
            "    Quint can't confirm this: Quint also names a run that the checker played as \
             `upTest`, which an `export` makes visible under two names, and the checker can't \
             tell whether it is that run",
        ],
        "{run}"
    );
}

#[test]
fn without_java_the_line_says_so_and_the_command_fails() {
    let quint = Quint::new();
    let run = quint.without_java("big");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   big  3 states  pass  (Quint)",
            "  states and properties: not compared",
            "    Quint can't confirm this: Java was not found on PATH, and `quint verify \
             --backend tlc` needs it to run TLC, so the state count and the properties are not \
             compared",
        ],
        "{run}"
    );
    assert!(quint.calls("verify").is_empty(), "no job is started");
}

#[test]
fn a_quint_that_stops_on_the_file_cannot_confirm_it_and_says_what_quint_said() {
    let quint = Quint::new();
    quint.answer(
        "big+belowBig",
        "[TLC] Compiling to TLA+ (via Apalache)...\n",
    );
    quint.answer(
        "big+belowBig.stderr",
        "/node_modules/@informalsystems/quint/dist/src/flattening/fullFlattener.js:90\n\
         \x20           throw new Error(`Module ${name} not found`);\n\
         \x20           ^\n\n\
         Error: Module A not found\n    at FullFlattener.flatten (fullFlattener.js:90:19)\n",
    );
    quint.answer("big", "[TLC] Compiling to TLA+ (via Apalache)...\n");
    let run = quint.confirm("big");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   big  3 states  pass  (Quint)",
            "  states: the checker found 3, `quint verify` gave no count",
            "    Quint can't confirm this: Quint stopped: Error: Module A not found",
            "  property belowBig: the checker says holds, `quint verify` says unknown",
            "    Quint can't confirm this: Quint stopped: Error: Module A not found",
        ],
        "{run}"
    );
}

#[test]
fn a_job_of_all_properties_that_stops_is_split_so_the_line_names_the_property_quint_refused() {
    let quint = Quint::new();
    quint.answer(
        "lift+inside@reachesTop",
        "[TLC] Compiling to TLA+ (via Apalache)...\nerror: TLC error (see output above)\n",
    );
    quint.answer(
        "lift@reachesTop",
        "Error: TLC threw an unexpected exception.\nerror: TLC error (see output above)\n",
    );
    let run = quint.confirm("lift");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   lift  4 states  fail  (Quint)",
            "  property reachesTop: the checker says broken, `quint verify` says unknown",
            "    Quint can't confirm this: Quint stopped: TLC threw an unexpected exception.",
        ],
        "`inside` is still compared, and its job gives the count\n{run}"
    );
    assert_eq!(
        quint.calls("verify").len(),
        4,
        "all, then each, then the expected counterexample"
    );
}

#[test]
fn a_model_that_is_not_read_from_a_qnt_file_cannot_be_confirmed_by_quint() {
    let quint = Quint::new();
    quint.sandbox.write(
        "specs/climb.model.ts",
        "import { m } from \"@earnestjs/model\";\n\
         export const Climb = m.model(\"Climb\", { n: m.int(0, 3).initial(0) });\n",
    );
    let run = quint.confirm("Climb");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        [
            "cannot   Climb  1 states  fail  (Quint)",
            "  not compared",
            "    Quint can't confirm this: the model is not read from a `.qnt` file, so Quint \
             has nothing to run",
        ],
        "{run}"
    );
}

const SKIPPED_FAKE: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
echo "$*" >> "$here/calls"
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
    cp "$here/parse.json" "$out"
    sed "s#FOLDER#$folder#" "$here/map.json" > "$map"
    ;;
  typecheck)
    echo "error: typechecking failed" >&2
    exit 1
    ;;
  compile)
    cat "$here/setup.json"
    ;;
  verify)
    printf '4 states generated, 4 distinct states found, 0 states left on queue.\n[ok] No violation found (1ms).\n'
    ;;
  test)
    cat "$here/test"
    if grep -q 'failed after' "$here/test"; then exit 1; fi
    ;;
esac
"#;

const SKIPPED_RUN_FAILS: &str = "\n  setup\n    1) setup::spec::upTest failed after 1 test(s)\n\n  1 failed\nerror: Tests failed\n";

fn a_setup_whose_spec_imports_a_module_with_variables(played: &str) -> Sandbox {
    let sandbox = Sandbox::empty();
    let samples = repository().join("tasks/quint/confirm/fixtures/skipped");
    let read = |name: &str| {
        fs::read_to_string(samples.join(name))
            .unwrap_or_else(|error| panic!("{name} can't be read: {error}"))
    };
    sandbox.write("specs/skipped.qnt", &read("skipped.qnt"));
    sandbox.write(
        "specs/earnest.quint.json",
        r#"{ "checks": [ { "file": "skipped.qnt", "main": "setup", "invariants": ["ok"], "runs": true } ] }"#,
    );
    for name in ["parse.json", "map.json", "setup.json"] {
        sandbox.write(&format!("fake/{name}"), &read(name));
    }
    sandbox.write("fake/quint", SKIPPED_FAKE);
    sandbox.write("fake/test", played);
    sandbox.write("fake/calls", "");
    sandbox.write("java/java", "#!/bin/sh\nexit 0\n");
    for program in ["fake/quint", "java/java"] {
        fs::set_permissions(
            sandbox.path().join(program),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the fakes can be made programs");
    }
    sandbox
}

fn confirm_in(sandbox: &Sandbox, quint: &Path) -> Run {
    Run::of(
        sandbox
            .command()
            .env("EARNEST_QUINT", quint)
            .env("PATH", sandbox.path().join("java"))
            .args(["confirm", "--quint", "specs"]),
    )
}

#[test]
fn a_run_the_checker_did_not_play_and_quint_test_breaks_is_never_the_same() {
    let sandbox = a_setup_whose_spec_imports_a_module_with_variables(SKIPPED_RUN_FAILS);
    let verified = Run::of(
        sandbox
            .command()
            .env("EARNEST_QUINT", sandbox.path().join("fake/quint"))
            .args(["verify", "specs"]),
    );
    assert!(
        verified
            .stdout
            .contains("note  runs not played: setup::spec::upTest"),
        "the checker says it did not play the run\n{verified}"
    );
    let run = confirm_in(&sandbox, &sandbox.path().join("fake/quint"));
    run.exits(1);
    assert!(
        !run.stdout.starts_with("same "),
        "`quint test` broke a run that the checker never played, so nothing was compared\n{run}"
    );
}

#[test]
fn a_run_only_quint_test_plays_differs_even_when_a_run_the_checker_did_not_play_ends_alike() {
    let sandbox = a_setup_whose_spec_imports_a_module_with_variables(
        "\n  setup\n    ok setup::spec::upTest passed 1 test(s)\n    1) upTest failed after 1 test(s)\n\n  1 passing (1ms)\n  1 failed\nerror: Tests failed\n",
    );
    let run = confirm_in(&sandbox, &sandbox.path().join("fake/quint"));
    run.exits(1);
    assert!(
        run.stdout
            .lines()
            .any(|line| line == "  run upTest: `quint test` played it, and the checker did not"),
        "`upTest` of `setup` itself is not the run `setup::spec::upTest` the checker did not play\n{run}"
    );
}

#[test]
fn quint_given_by_a_relative_path_runs_for_confirm_as_it_runs_for_verify() {
    let sandbox =
        a_setup_whose_spec_imports_a_module_with_variables("\n  setup\n\n  0 passing (1ms)\n");
    let run = confirm_in(&sandbox, Path::new("fake/quint"));
    assert!(
        !run.stdout.contains("did not start"),
        "`quint` is found from the folder earnest was started in\n{run}"
    );
}

fn a_difference_in_lift_compiled_as(fixture: &str) -> Run {
    let quint = Quint::new();
    quint.sandbox.write(
        "fake/compiled/lift.json",
        &fs::read_to_string(
            repository()
                .join("tasks/quint/confirm/fixtures")
                .join(fixture),
        )
        .expect("the compiled `lift` can be read"),
    );
    quint.confirm("lift").exits(0);
    let cache = quint.sandbox.path().join(".earnest/quint");
    let compiled: Vec<PathBuf> = fs::read_dir(&cache)
        .expect("the compiled JSON is cached")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            fs::read_to_string(path).is_ok_and(|text| text.contains("\"name\":\"lift\""))
        })
        .collect();
    assert_eq!(
        compiled.len(),
        1,
        "one compiled JSON of `lift`: {compiled:?}"
    );
    let text = fs::read_to_string(&compiled[0]).expect("the cache can be read");
    let top = r#"{"id":8,"kind":"int","value":3}"#;
    assert!(text.contains(top), "the bound of `up` is in the JSON");
    fs::write(
        &compiled[0],
        text.replace(top, r#"{"id":8,"kind":"int","value":5}"#),
    )
    .expect("the cache can be written");
    quint.confirm("lift")
}

#[test]
fn a_fold_over_numbers_that_nothing_calls_does_not_hide_a_model_changed_by_hand() {
    let run = a_difference_in_lift_compiled_as("lift-with-fold.json");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().next(),
        Some("differs  lift  6 states  fail  (Quint)"),
        "`total` folds a set of numbers, whose order changes no answer, and nothing calls it\n{run}"
    );
}

#[test]
fn a_range_in_another_module_of_the_file_does_not_hide_a_model_changed_by_hand() {
    let run = a_difference_in_lift_compiled_as("lift-beside-ranges.json");
    run.exits(1);
    assert_eq!(
        run.stdout.lines().next(),
        Some("differs  lift  6 states  fail  (Quint)"),
        "only the module `ranges` uses `range`, and Quint's compile of `lift` lists it in its table\n{run}"
    );
}
