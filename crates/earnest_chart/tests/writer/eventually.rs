use crate::support::{named, outcome, rule, step};
use earnest_chart::{Section, detail_lines, report};
use earnest_check::{Change, Outcome, Path, RuleKind, RuleOutcome, Shown, Status, Verdict};
use serde_json::json;

const STAYS: &str = "nothing changes the state from here, so the run can stay in it forever";

fn flag(value: &str) -> Shown {
    Shown {
        text: value.to_owned(),
        json: json!(value == "true"),
    }
}

fn flips(action: &str, field: &str, before: &str, after: &str) -> earnest_check::Step {
    step(
        action,
        Vec::new(),
        vec![Change {
            field: field.to_owned(),
            before: flag(before),
            after: flag(after),
        }],
    )
}

fn about_runs(
    id: &str,
    says: &str,
    path: Path,
    repeats_from: Option<usize>,
    reason: &str,
) -> RuleOutcome {
    RuleOutcome {
        verdict: Verdict::Broken,
        kind: RuleKind::Eventually,
        reason: Some(reason.to_owned()),
        repeats_from,
        ..rule(id, says, Some(path))
    }
}

fn worker() -> Outcome {
    let start = vec![named("busy", flag("false")), named("done", flag("false"))];
    let looping = Path {
        start: start.clone(),
        steps: vec![
            flips("fidget", "busy", "false", "true"),
            flips("fidget", "busy", "true", "false"),
        ],
    };
    let staying = Path {
        start,
        steps: vec![flips("finish", "done", "false", "true")],
    };
    Outcome {
        rules: vec![
            rule("sane", "The worker is sane", None),
            about_runs(
                "finishes",
                "The job gets done",
                looping,
                Some(0),
                "steps 1 to 2 can repeat forever",
            ),
            about_runs("rests", "The worker rests", staying, None, STAYS),
        ],
        ..outcome(Status::Fail, 4)
    }
}

#[test]
fn a_broken_rule_about_runs_ends_its_path_with_a_why_line_for_a_loop_and_for_a_stay() {
    assert_eq!(
        detail_lines(&worker()),
        [
            "broken  finishes  \"The job gets done\"",
            "  start  busy = false, done = false",
            "  1. fidget  busy: false -> true",
            "  2. fidget  busy: true -> false",
            "  end    busy = false, done = false",
            "  why    steps 1 to 2 can repeat forever",
            "broken  rests  \"The worker rests\"",
            "  start  busy = false, done = false",
            "  1. finish  done: false -> true",
            "  end    busy = false, done = true",
            "  why    nothing changes the state from here, so the run can stay in it forever",
        ],
        "each broken rule about runs, its path and why the run goes on forever"
    );
}

#[test]
fn a_rule_about_runs_that_holds_prints_nothing() {
    let found = Outcome {
        rules: vec![RuleOutcome {
            kind: RuleKind::Eventually,
            ..rule("finishes", "The job gets done", None)
        }],
        ..outcome(Status::Pass, 4)
    };
    assert_eq!(
        detail_lines(&found),
        Vec::<String>::new(),
        "no detail for a rule that holds"
    );
}

#[test]
fn the_report_shows_the_why_line_of_a_broken_rule_about_runs() {
    let worker = worker();
    let sections = [Section {
        name: "Worker (lazy)",
        file: "models/worker.model.ts",
        outcome: &worker,
    }];
    let expected = r#"## Worker \(lazy\)

**fail** · Worker \(lazy\) · 4 states · `models/worker.model.ts`

```text
broken  finishes  "The job gets done"
  start  busy = false, done = false
  1. fidget  busy: false -> true
  2. fidget  busy: true -> false
  end    busy = false, done = false
  why    steps 1 to 2 can repeat forever
broken  rests  "The worker rests"
  start  busy = false, done = false
  1. finish  done: false -> true
  end    busy = false, done = true
  why    nothing changes the state from here, so the run can stay in it forever
```"#;
    assert_eq!(
        report(&sections, &[], None),
        expected,
        "the detail block of the report, under the name of a run in a setup"
    );
}
