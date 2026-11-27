#![cfg(unix)]

use serde_json::Value;

use crate::quint_integers::{ran_in, recorded_in};
use crate::support::rule;

const FLOORED: &str = "floored";
const SIGNS: &str = "signs";

const INVARIANTS: [&str; 4] = ["rebuilt", "between", "literals", "wideLiterals"];

const TRUNCATES: &str = "    Quint can't confirm this: the check's runs use `/` or `%`, which \
                         `quint test` truncates toward zero, where the checker and Quint's TLC \
                         run take the floor of the quotient: with a negative number they answer \
                         otherwise, as `(-7) / 2` is -3 in `quint test` and -4 in the checker";

const REFUSES: &str = "    Quint can't confirm this: Quint stopped: TLC refuses `%` by a divisor \
                       below 1 (\"The second argument of % should be a positive number\"), where \
                       the checker takes `a % b` as `a - b * (a / b)` for every divisor but zero";

fn verified(case: &str) -> Value {
    let (stdout, stderr) = ran_in(case, &["verify", "--json", "--no-cache", "."]);
    let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!("earnest verify prints JSON ({error}):\n{stdout}\n{stderr}")
    });
    report["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == case))
        .cloned()
        .unwrap_or(Value::Null)
}

fn quint_test_says(case: &str, run: &str) -> &'static str {
    let recorded = recorded_in(case, "test.txt");
    if recorded
        .lines()
        .any(|line| line.trim().starts_with(&format!("ok {run} passed")))
    {
        "holds"
    } else {
        "broken"
    }
}

#[test]
fn division_floors_and_the_remainder_follows_for_all_four_signs_inside_and_past_64_bits() {
    let model = verified(FLOORED);
    for invariant in INVARIANTS {
        assert_eq!(
            rule(&model, invariant)["verdict"],
            "holds",
            "`{invariant}` holds where `/` is the floor of the exact quotient and `%` is \
             `a - b * (a / b)`, as `quint verify --backend tlc` computes `(-7) / 2`\n{model:#}"
        );
    }
    assert_eq!(
        (&model["states"], &model["status"]),
        (&Value::from(8), &Value::from("fail")),
        "eight pairs, and only the run that expects truncation is broken\n{model:#}"
    );
}

#[test]
fn runs_expect_the_floored_answers_where_quint_test_truncates() {
    let model = verified(FLOORED);
    for (run, ours) in [
        ("flooredTest", "holds"),
        ("truncatedTest", "broken"),
        ("wideFlooredTest", "holds"),
        ("positiveTest", "holds"),
    ] {
        assert_eq!(rule(&model, run)["verdict"], ours, "{run}\n{model:#}");
        let theirs = quint_test_says(FLOORED, run);
        assert_eq!(
            ours == theirs,
            run == "positiveTest",
            "`quint test --backend typescript` of Quint 0.33.1 says `{run}` {theirs}: it agrees \
             only where no operand is negative"
        );
    }
}

#[test]
fn confirm_with_quint_names_truncation_in_quint_test_and_not_a_bare_difference() {
    let (stdout, stderr) = ran_in(FLOORED, &["confirm", "--quint", "."]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("cannot   floored  8 states  fail  (Quint)"),
        "{stdout}\n{stderr}"
    );
    for run in ["flooredTest", "truncatedTest", "wideFlooredTest"] {
        let at = lines
            .iter()
            .position(|line| line.starts_with(&format!("  run {run}:")))
            .unwrap_or_else(|| panic!("{run} is compared\n{stdout}"));
        assert_eq!(lines.get(at + 1).copied(), Some(TRUNCATES), "{stdout}");
    }
    assert!(
        !stdout.contains("positiveTest"),
        "the run with no negative operand agrees\n{stdout}"
    );
    assert!(!stdout.contains("differs"), "{stdout}");
}

#[test]
fn confirm_with_quint_names_tlc_s_refusal_of_a_remainder_by_a_negative_number() {
    let (stdout, stderr) = ran_in(SIGNS, &["confirm", "--quint", "."]);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("cannot   signs  4 states  pass  (Quint)"),
        "`quint verify --backend tlc` stops where TLC meets `7 % (-2)`\n{stdout}\n{stderr}"
    );
    let reasons: Vec<&&str> = lines
        .iter()
        .filter(|line| line.starts_with("    "))
        .collect();
    assert!(
        !reasons.is_empty() && reasons.iter().all(|line| **line == REFUSES),
        "each line names TLC's refusal\n{stdout}"
    );
    assert!(!stdout.contains("differs"), "{stdout}");
}

#[test]
fn the_signs_model_holds_in_the_checker() {
    let model = verified(SIGNS);
    for invariant in ["rebuilt", "literals"] {
        assert_eq!(rule(&model, invariant)["verdict"], "holds", "{model:#}");
    }
}
