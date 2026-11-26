use crate::{only_draft, search};
use earnest_ast::{HelperKind, Term, TermKind};
use earnest_check::{RuleKind, Verdict};
use earnest_rs::read;
use earnest_semantic::lower;

const GATE: &str = include_str!("../../../../tasks/conformance/runs/gate.rs");
const KETTLE: &str = include_str!("../../../../tasks/conformance/runs/kettle.rs");
const CLOCK: &str = include_str!("../../../../tasks/conformance/runs/clock.rs");

fn with(source: &str, from: &str, to: &str) -> String {
    assert!(source.contains(from), "{from} is in the model");
    source.replacen(from, to, 1)
}

fn refused(source: &str) -> Vec<(String, String)> {
    let reading = read("src/runs.rs", source);
    assert_eq!(
        reading.drafts,
        [],
        "no model of an unreadable file is given"
    );
    reading
        .problems
        .into_iter()
        .map(|problem| {
            (
                problem.feature.trim_matches('`').to_owned(),
                problem.message,
            )
        })
        .collect()
}

fn refused_once(source: &str, feature: &str, words: &str) {
    let found = refused(source);
    assert_eq!(found.len(), 1, "one problem: {found:?}");
    assert_eq!(found[0].0, feature, "the feature: {found:?}");
    assert!(found[0].1.contains(words), "{words} in {found:?}");
}

fn verdicts(source: &str) -> Vec<(String, RuleKind, Verdict)> {
    search(&only_draft("runs.rs", source))
        .rules
        .into_iter()
        .map(|rule| (rule.id, rule.kind, rule.verdict))
        .collect()
}

#[test]
fn fair_marks_an_action_and_after_calls_a_helper_method_on_the_state() {
    let draft = only_draft("gate.rs", GATE);
    let fair: Vec<(&str, bool)> = draft
        .actions
        .iter()
        .map(|action| (action.id.as_str(), action.fair))
        .collect();
    assert_eq!(
        fair,
        [
            ("blink", false),
            ("pay", true),
            ("push", false),
            ("enter", false)
        ],
        "only `pay` is fair"
    );
    let ids: Vec<&str> = draft
        .runs
        .eventually
        .iter()
        .map(|rule| rule.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["opens", "opens_once_paid", "lit_and_paid"],
        "the rules about runs in file order"
    );
    assert_eq!(draft.runs.eventually[0].after, None, "no `after`");
    let reaches = &draft.runs.eventually[0].reaches;
    assert_eq!(reaches.state_name.as_deref(), Some("self"), "the state");
    assert_eq!(reaches.text, "self.door == Door::Open", "the body's text");
    let after = draft.runs.eventually[1]
        .after
        .as_ref()
        .expect("`after = paid_up`");
    assert_eq!(after.text, "paid_up", "the helper's name");
    assert!(
        matches!(&after.term.kind, TermKind::Helper { kind: HelperKind::Method, name, arguments }
            if name == "paid_up"
                && matches!(arguments.as_slice(), [Term { kind: TermKind::Name(state), .. }] if state == "self")),
        "a call of the helper on the state: {:?}",
        after.term.kind
    );
    assert!(
        draft.helpers.iter().any(|helper| helper.name == "paid_up"),
        "the helper is a helper"
    );
}

#[test]
fn a_witness_is_a_chain_of_actions_from_self_and_a_condition_on_the_name_bound() {
    let draft = only_draft("gate.rs", GATE);
    let witness = &draft.runs.witnesses[0];
    assert_eq!(witness.id, "pay_then_push", "the id is the method's name");
    assert_eq!(
        witness.says, "Pay, then push: the gate opens",
        "the sentence"
    );
    let steps: Vec<&str> = witness
        .steps
        .iter()
        .map(|step| step.action.as_str())
        .collect();
    assert_eq!(steps, ["pay", "push"], "first to last");
    assert_eq!(
        witness.state_name.as_deref(),
        Some("after"),
        "the bound name"
    );
    assert_eq!(witness.setup, None, "the model's own start states");
    let kettle = only_draft("kettle.rs", KETTLE);
    let bare = &kettle.runs.witnesses[0];
    assert_eq!(bare.steps, [], "`Some(condition)` alone has no steps");
    assert_eq!(bare.state_name.as_deref(), Some("self"), "on `self`");
}

#[test]
fn a_step_gives_its_inputs_by_parameter_name_and_a_witness_names_its_setup() {
    let draft = only_draft("clock.rs", CLOCK);
    let first = &draft.runs.witnesses[0].steps[0];
    assert_eq!(first.action, "drift", "the action");
    assert_eq!(first.inputs.len(), 1, "one input");
    assert_eq!(first.inputs[0].0, "by", "by its parameter's name");
    assert!(
        matches!(first.inputs[0].1.kind, TermKind::Int(1)),
        "{:?}",
        first.inputs[0].1.kind
    );
    let catches_up = &draft.runs.witnesses[1];
    assert_eq!(
        catches_up.setup.as_ref().map(|(id, _)| id.as_str()),
        Some("behind"),
        "`setup = behind`"
    );
    let steps: Vec<&str> = catches_up
        .steps
        .iter()
        .map(|step| step.action.as_str())
        .collect();
    assert_eq!(steps, ["drift", "sync"], "the chain");
    let setups: Vec<(&str, Vec<&str>)> = draft
        .runs
        .setups
        .iter()
        .map(|setup| {
            (
                setup.id.as_str(),
                setup
                    .starts
                    .iter()
                    .map(|(field, _, _)| field.as_str())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        setups,
        [("behind", vec!["skew"]), ("synced_already", vec!["synced"])],
        "each setup and the fields it writes"
    );
    assert!(
        matches!(draft.runs.setups[0].starts[0].2.kind, TermKind::Int(-2)),
        "{:?}",
        draft.runs.setups[0].starts[0].2.kind
    );
}

#[test]
fn the_checker_judges_the_rust_forms_as_written() {
    assert_eq!(
        verdicts(GATE),
        [
            ("opens".to_owned(), RuleKind::Eventually, Verdict::Holds),
            (
                "opens_once_paid".to_owned(),
                RuleKind::Eventually,
                Verdict::Holds
            ),
            (
                "lit_and_paid".to_owned(),
                RuleKind::Eventually,
                Verdict::Broken
            ),
            (
                "pay_then_push".to_owned(),
                RuleKind::Witness,
                Verdict::Holds
            ),
            ("push_first".to_owned(), RuleKind::Witness, Verdict::Broken),
        ],
        "the gate"
    );
    let unfair = with(GATE, ", fair)", ")");
    assert_eq!(
        verdicts(&unfair)[0],
        ("opens".to_owned(), RuleKind::Eventually, Verdict::Broken),
        "without `fair` the light blinks forever"
    );
    assert_eq!(
        verdicts(KETTLE),
        [
            ("boils".to_owned(), RuleKind::Eventually, Verdict::Broken),
            ("settles".to_owned(), RuleKind::Eventually, Verdict::Holds),
            ("starts_cold".to_owned(), RuleKind::Witness, Verdict::Holds),
            ("hot_at_once".to_owned(), RuleKind::Witness, Verdict::Broken),
        ],
        "the kettle"
    );
}

#[test]
fn a_setup_writes_constants_over_a_clone_of_the_state_and_nothing_else() {
    let behind = "Self {\n            skew: -2,\n            ..self.clone()\n        }";
    refused_once(
        &with(
            CLOCK,
            behind,
            "Self {\n            skew: self.skew,\n            ..self.clone()\n        }",
        ),
        "self.skew",
        "each value is a constant",
    );
    refused_once(
        &with(CLOCK, behind, "Self { ..self.clone() }"),
        "behind",
        "`Self { field: value, ..self.clone() }`",
    );
    refused_once(
        &with(CLOCK, behind, "Self { skew: -2, synced: false }"),
        "behind",
        "`Self { field: value, ..self.clone() }`",
    );
    refused_once(
        &with(
            CLOCK,
            behind,
            "let skew = -2;\n        Self { skew, ..self.clone() }",
        ),
        "behind",
        "`Self { field: value, ..self.clone() }`",
    );
    let found = refused(&with(
        CLOCK,
        "fn behind(&self) -> Self {",
        "fn behind(&self, by: i8) -> Self {",
    ));
    assert!(
        found
            .iter()
            .any(|(_, message)| message.contains("takes `&self` and nothing else")),
        "{found:?}"
    );
    let outside = with(CLOCK, "skew: -2,", "skew: -9,");
    let draft = only_draft("clock.rs", &outside);
    assert!(
        lower(&draft).is_err(),
        "a start the field does not declare is refused when the model is lowered"
    );
}

#[test]
fn a_witness_body_that_is_not_a_chain_and_a_condition_is_refused() {
    let body = "let after = self.drift(1)?;\n        Some(after.skew == 1)";
    for (written, feature, words) in [
        (
            "let after = self.drift(1)?;\n        let again = after.sync()?;\n        Some(again.synced)",
            "drifts_ahead",
            "a witness's body",
        ),
        (
            "let after = self.drift(1)?;\n        Some(after.skew == 1).filter(|held| *held)",
            "Some(after.skew == 1).filter(|held| *held)",
            "a witness's body",
        ),
        (
            "let after = Self { skew: 1, synced: false };\n        Some(after.skew == 1)",
            "Self { skew: 1, synced: false }",
            "a witness's body",
        ),
        (
            "let after = self;\n        Some(after.skew == 1)",
            "self",
            "a witness's body",
        ),
        (
            "let after = self.drift(1)?.fly()?;\n        Some(after.skew == 1)",
            ".fly()",
            "the model's own actions",
        ),
        (
            "let after = self.drift(1);\n        Some(after.is_some())",
            ".drift()",
            "write `?` after it",
        ),
        (
            "let after = self.drift()?;\n        Some(after.skew == 1)",
            ".drift()",
            "E0061",
        ),
        (
            "let after = self.drift(1)?;\n        Some(after.skew)",
            "after.skew",
            "E0308",
        ),
        (
            "let mut after = self.drift(1)?;\n        Some(after.skew == 1)",
            "mut after",
            "a witness's body",
        ),
    ] {
        refused_once(&with(CLOCK, body, written), feature, words);
    }
    refused_once(
        &with(
            CLOCK,
            "fn drifts_ahead(&self) -> Option<bool> {",
            "fn drifts_ahead(&self) -> bool {",
        ),
        "fn drifts_ahead",
        "returns `Option<bool>`",
    );
}

#[test]
fn a_question_mark_after_an_action_that_gives_back_the_state_is_refused() {
    let tick = with(
        KETTLE,
        "    #[end(",
        "    #[action(\"the clock ticks\")]\n    fn tick(&self) -> Self {\n        Self { phase: self.phase }\n    }\n\n    #[end(",
    );
    let chained = with(
        &tick,
        "let after = self.heat()?;",
        "let after = self.tick().heat()?;",
    );
    let draft = only_draft("kettle.rs", &chained);
    let steps: Vec<&str> = draft.runs.witnesses[1]
        .steps
        .iter()
        .map(|step| step.action.as_str())
        .collect();
    assert_eq!(steps, ["tick", "heat"], "no `?` after `tick`");
    refused_once(
        &with(
            &tick,
            "let after = self.heat()?;",
            "let after = self.tick()?;",
        ),
        ".tick()",
        "E0277",
    );
}

#[test]
fn after_names_a_helper_method_that_takes_the_state_and_gives_a_boolean() {
    let after = "after = paid_up)]";
    for (written, feature, words) in [
        ("after = nobody)]", "nobody", "E0599"),
        ("after = pay)]", "pay", "a helper method of the model"),
        ("after = opens)]", "opens", "a helper method of the model"),
    ] {
        refused_once(&with(GATE, after, written), feature, words);
    }
    let counted = with(
        GATE,
        "    fn paid_up(&self) -> bool {\n        self.paid\n    }",
        "    fn paid_up(&self) -> u8 {\n        u8::from(self.paid)\n    }",
    );
    refused_once(&counted, "paid_up", "returns `bool`");
}

#[test]
fn each_attribute_takes_only_its_own_options() {
    for (from, to, feature, words) in [
        (
            "#[action(\"the visitor pushes\")]",
            "#[action(\"the visitor pushes\", fair = yes)]",
            "fair",
            "`#[action(\"...\", fair)]`",
        ),
        (
            "#[action(\"the visitor pays\", fair)]",
            "#[action(\"the visitor pays\", fair, fair)]",
            "fair",
            "given twice",
        ),
        (
            "#[action(\"the visitor pushes\")]",
            "#[action(\"the visitor pushes\", after = paid_up)]",
            "after",
            "`#[action(\"...\", fair)]`",
        ),
        (
            "after = paid_up)]",
            "before = paid_up)]",
            "before",
            "`after = method`",
        ),
        ("after = paid_up)]", "after)]", "after", "`after = method`"),
        (
            "#[witness(\"Push before paying: the gate opens\")]",
            "#[witness(\"Push before paying: the gate opens\", fair)]",
            "fair",
            "`setup = method`",
        ),
    ] {
        refused_once(&with(GATE, from, to), feature, words);
    }
    refused_once(
        &with(
            CLOCK,
            "#[rule(\"A synced clock shows no skew\")]",
            "#[rule(\"A synced clock shows no skew\", fair)]",
        ),
        "fair",
        "takes only its sentence",
    );
    refused_once(
        &with(CLOCK, "setup = behind)]", "setup = drift)]"),
        "drift",
        "`#[setup(\"...\")]`",
    );
    refused_once(
        &with(CLOCK, "#[setup(\"The clock starts synced\")]", "#[setup]"),
        "#[setup]",
        "write the sentence in quotes",
    );
}
