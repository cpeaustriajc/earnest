use crate::build::{
    FILE, HERE, action, declare, draft, equal, field, here, int, lowered, one_of, optional, range,
    record, rule, starting, text, the_problem, yes,
};
use earnest_ast::{Draft, StepDraft, Term, WitnessDraft};
use earnest_diagnostics::{Place, Problem};
use earnest_ir::Frame;
use earnest_value::Value;

const STEP_PLACE: Place = Place {
    line: 40,
    column: 12,
};

fn panel() -> Draft {
    let mut source = draft(vec![
        ("n", starting(range(0, 3), int(0))),
        ("last", optional(one_of(&["key", "mouse"]))),
    ]);
    let bump = action("bump", None, record(vec![("n", int(1))]));
    let mut press = action("press", None, record(vec![("n", int(2))]));
    declare(&mut press, "by", one_of(&["key", "mouse"]));
    declare(&mut press, "times", range(1, 2));
    source.actions = vec![bump, press];
    source
}

fn step(id: &str, inputs: Vec<(&str, Term)>) -> StepDraft {
    StepDraft {
        action: id.to_owned(),
        place: STEP_PLACE,
        inputs: inputs
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    }
}

fn witness(steps: Vec<StepDraft>, expect: Term) -> WitnessDraft {
    WitnessDraft {
        id: "climbs".to_owned(),
        says: "The panel climbs".to_owned(),
        place: Place {
            line: 39,
            column: 1,
        },
        setup: None,
        steps,
        state_name: Some("s".to_owned()),
        expect,
        text: String::new(),
    }
}

fn with_witness(steps: Vec<StepDraft>) -> Draft {
    let mut source = panel();
    source
        .runs
        .witnesses
        .push(witness(steps, equal(field("n"), int(2))));
    source
}

fn problem_of(steps: Vec<StepDraft>) -> Problem {
    let problem = the_problem(&with_witness(steps));
    assert_eq!(problem.file, FILE, "the problem names the draft's file");
    problem
}

#[test]
fn a_witness_keeps_its_id_sentence_line_and_steps_by_action_position() {
    let model = lowered(&with_witness(vec![
        step("press", vec![("times", int(2)), ("by", text("mouse"))]),
        step("bump", Vec::new()),
    ]));
    let found = model.runs.witnesses.first().expect("one witness");
    assert_eq!(found.id, "climbs", "the id");
    assert_eq!(found.says, "The panel climbs", "the sentence");
    assert_eq!(found.line, 39, "the line of the declaration");
    assert_eq!(found.setup, None, "the model's own start states");
    let steps: Vec<(usize, Vec<String>)> = found
        .steps
        .iter()
        .map(|played| {
            let shown = played
                .input
                .iter()
                .map(|value| model.symbols.show(value))
                .collect();
            (played.action, shown)
        })
        .collect();
    assert_eq!(
        steps,
        [
            (1, vec!["\"mouse\"".to_owned(), "2".to_owned()]),
            (0, Vec::new())
        ],
        "each step's action by its position, and its inputs in the order the action declares them"
    );
}

#[test]
fn what_a_witness_expects_reads_the_state() {
    let model = lowered(&with_witness(Vec::new()));
    let found = model.runs.witnesses.first().expect("one witness");
    let start = vec![Value::Int(2), Value::Null];
    assert_eq!(
        found.expect.truth(&mut Frame::new(&start, found.slots)),
        Ok(true),
        "true where the count is 2"
    );
    let other = vec![Value::Int(1), Value::Null];
    assert_eq!(
        found.expect.truth(&mut Frame::new(&other, found.slots)),
        Ok(false),
        "false where the count is 1"
    );
}

#[test]
fn the_witnesses_are_listed_after_the_rules_by_id() {
    let mut source = with_witness(Vec::new());
    source.rules.push(rule("low", equal(yes(), yes())));
    let model = lowered(&source);
    let ids: Vec<&str> = model.rule_ids().collect();
    assert_eq!(ids, ["low", "climbs"], "the rule, then the witness");
}

#[test]
fn a_step_of_an_action_the_model_does_not_have_is_refused() {
    let problem = problem_of(vec![step("jump", Vec::new())]);
    assert_eq!(problem.place(), STEP_PLACE, "where the step is written");
    assert_eq!(problem.feature, "the step `jump`", "the step");
    assert_eq!(
        problem.message, "the model has no action with this id",
        "what is wrong"
    );
}

#[test]
fn an_input_the_action_does_not_declare_is_refused() {
    let problem = problem_of(vec![step(
        "press",
        vec![
            ("by", text("key")),
            ("times", int(1)),
            ("speed", here(int(3))),
        ],
    )]);
    assert_eq!(problem.place(), HERE, "where the value is written");
    assert_eq!(problem.feature, "the input `speed`", "the input");
    assert_eq!(
        problem.message, "the action `press` declares no input with this name",
        "what is wrong"
    );
}

#[test]
fn an_input_given_twice_is_refused() {
    let problem = problem_of(vec![step(
        "press",
        vec![
            ("by", text("key")),
            ("times", int(1)),
            ("by", here(text("mouse"))),
        ],
    )]);
    assert_eq!(problem.place(), HERE, "where the second value is written");
    assert_eq!(problem.feature, "the input `by`", "the input");
    assert_eq!(
        problem.message, "the step gives this input a value twice",
        "what is wrong"
    );
}

#[test]
fn an_input_left_without_a_value_is_refused() {
    let problem = problem_of(vec![step("press", vec![("by", text("key"))])]);
    assert_eq!(problem.place(), STEP_PLACE, "where the step is written");
    assert_eq!(problem.feature, "the step `press`", "the step");
    assert_eq!(
        problem.message, "the action takes the input `times`, and the step gives it no value",
        "the input that has no value"
    );
}

#[test]
fn a_bare_action_that_takes_inputs_is_refused() {
    let problem = problem_of(vec![step("press", Vec::new())]);
    assert_eq!(problem.place(), STEP_PLACE, "where the step is written");
    assert_eq!(problem.feature, "the step `press`", "the step");
    assert_eq!(
        problem.message, "the action takes the input `by`, and the step gives it no value",
        "the first input that has no value"
    );
}

#[test]
fn an_input_value_outside_its_declaration_is_refused() {
    let problem = problem_of(vec![step(
        "press",
        vec![("by", text("key")), ("times", here(int(3)))],
    )]);
    assert_eq!(problem.place(), HERE, "where the value is written");
    assert_eq!(problem.feature, "the input value 3", "the value");
    assert_eq!(
        problem.message, "it is outside the values the input declares",
        "what is wrong"
    );
    let choice = problem_of(vec![step(
        "press",
        vec![("by", here(text("pen"))), ("times", int(1))],
    )]);
    assert_eq!(choice.place(), HERE, "where the choice is written");
    assert!(
        choice.feature.contains("pen"),
        "the choice the input does not declare: {choice:?}"
    );
}

#[test]
fn what_a_witness_expects_must_be_a_boolean() {
    let mut source = panel();
    source
        .runs
        .witnesses
        .push(witness(Vec::new(), here(field("n"))));
    let problem = the_problem(&source);
    assert_eq!(problem.file, FILE, "the problem names the draft's file");
    assert_eq!(problem.place(), HERE, "where the condition is written");
}
