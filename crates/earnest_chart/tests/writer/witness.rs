use crate::support::{named, outcome, rule, step};
use earnest_chart::{Section, detail_lines, report};
use earnest_check::{Change, Outcome, Path, RuleKind, RuleOutcome, Shown, Status, Verdict};
use serde_json::json;

fn number(value: i64) -> Shown {
    Shown {
        text: value.to_string(),
        json: json!(value),
    }
}

fn shut() -> Shown {
    Shown {
        text: "false".to_owned(),
        json: json!(false),
    }
}

fn witness(id: &str, says: &str, path: Path, reason: &str) -> RuleOutcome {
    RuleOutcome {
        verdict: Verdict::Broken,
        kind: RuleKind::Witness,
        reason: Some(reason.to_owned()),
        ..rule(id, says, Some(path))
    }
}

fn one_floor_up() -> Path {
    Path {
        start: vec![named("floor", number(1)), named("doors", shut())],
        steps: vec![step(
            "up",
            Vec::new(),
            vec![Change {
                field: "floor".to_owned(),
                before: number(1),
                after: number(2),
            }],
        )],
    }
}

fn lift() -> Outcome {
    let at_start = Path {
        start: vec![named("floor", number(1)), named("doors", shut())],
        steps: Vec::new(),
    };
    Outcome {
        rules: vec![
            rule("low", "The lift stays in the building", None),
            witness(
                "climbs",
                "Two floors up reaches the top floor",
                one_floor_up(),
                "step 2, `up`, can't happen here",
            ),
            witness(
                "grounded",
                "The lift starts on the ground floor",
                at_start,
                "what the witness expects is false here",
            ),
        ],
        ..outcome(Status::Fail, 6)
    }
}

#[test]
fn a_broken_witness_ends_its_path_with_a_why_line_as_deep_as_the_path() {
    assert_eq!(
        detail_lines(&lift()),
        [
            "broken  climbs  \"Two floors up reaches the top floor\"",
            "  start  floor = 1, doors = false",
            "  1. up  floor: 1 -> 2",
            "  end    floor = 2, doors = false",
            "  why    step 2, `up`, can't happen here",
            "broken  grounded  \"The lift starts on the ground floor\"",
            "  start  floor = 1, doors = false",
            "  why    what the witness expects is false here",
        ],
        "each broken witness, its path and its reason"
    );
}

#[test]
fn a_reason_over_several_lines_is_printed_on_one() {
    let found = Outcome {
        rules: vec![witness(
            "climbs",
            "Two floors up",
            one_floor_up(),
            "step 2, `up`,\ncan't happen here",
        )],
        ..outcome(Status::Fail, 6)
    };
    let lines = detail_lines(&found);
    assert_eq!(
        lines.last().map(String::as_str),
        Some("  why    step 2, `up`, can't happen here"),
        "the line break is a space: {lines:?}"
    );
}

#[test]
fn a_broken_rule_has_no_why_line() {
    let found = Outcome {
        rules: vec![rule("low", "The lift stays low", Some(one_floor_up()))],
        ..outcome(Status::Fail, 6)
    };
    let lines = detail_lines(&found);
    assert!(
        lines
            .iter()
            .all(|line| !line.trim_start().starts_with("why")),
        "a rule that must be true in every state needs no reason: {lines:?}"
    );
}

#[test]
fn the_report_shows_the_why_line_of_a_broken_witness() {
    let lift = lift();
    let sections = [Section {
        name: "Lift",
        file: "models/lift.model.ts",
        outcome: &lift,
    }];
    let expected = r#"## Lift

**fail** · Lift · 6 states · `models/lift.model.ts`

```text
broken  climbs  "Two floors up reaches the top floor"
  start  floor = 1, doors = false
  1. up  floor: 1 -> 2
  end    floor = 2, doors = false
  why    step 2, `up`, can't happen here
broken  grounded  "The lift starts on the ground floor"
  start  floor = 1, doors = false
  why    what the witness expects is false here
```"#;
    assert_eq!(
        report(&sections, &[], None),
        expected,
        "the detail block of the report"
    );
}
