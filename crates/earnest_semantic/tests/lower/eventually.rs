use crate::build::{
    FILE, HERE, action, draft, equal, field, here, int, lowered, range, record, rule, starting,
    the_problem, yes,
};
use earnest_ast::{ConditionDraft, Draft, EventuallyDraft, Term, WitnessDraft};
use earnest_diagnostics::Place;
use earnest_ir::{Code, Frame};
use earnest_value::Value;

fn condition(term: Term) -> ConditionDraft {
    ConditionDraft {
        state_name: Some("s".to_owned()),
        term,
        text: String::new(),
    }
}

fn eventually(id: &str, after: Option<Term>, reaches: Term) -> EventuallyDraft {
    EventuallyDraft {
        id: id.to_owned(),
        says: format!("{id} comes"),
        place: Place {
            line: 33,
            column: 1,
        },
        after: after.map(condition),
        reaches: condition(reaches),
    }
}

fn counter() -> Draft {
    let mut source = draft(vec![("n", starting(range(0, 3), int(0)))]);
    source.actions = vec![
        action("bump", None, record(vec![("n", int(1))])),
        action("reset", None, record(vec![("n", int(0))])),
    ];
    source
}

fn truth(code: &Code, slots: usize, n: i64) -> Result<bool, String> {
    let state = vec![Value::Int(n)];
    code.truth(&mut Frame::new(&state, slots))
        .map_err(earnest_ir::Fault::describe)
}

#[test]
fn a_rule_about_runs_keeps_its_id_sentence_line_and_both_conditions() {
    let mut source = counter();
    source.runs.eventually.push(eventually(
        "settles",
        Some(equal(field("n"), int(1))),
        equal(field("n"), int(3)),
    ));
    let model = lowered(&source);
    let found = model.runs.eventually.first().expect("one rule about runs");
    assert_eq!(found.id, "settles", "the id");
    assert_eq!(found.says, "settles comes", "the sentence");
    assert_eq!(found.line, 33, "the line of the declaration");
    let after = found.after.as_ref().expect("an `after`");
    assert_eq!(
        truth(after, found.slots, 1),
        Ok(true),
        "`after` is true at 1"
    );
    assert_eq!(
        truth(after, found.slots, 2),
        Ok(false),
        "`after` is false at 2"
    );
    assert_eq!(
        truth(&found.reaches, found.slots, 3),
        Ok(true),
        "`reaches` is true at 3"
    );
    assert_eq!(
        truth(&found.reaches, found.slots, 0),
        Ok(false),
        "`reaches` is false at 0"
    );
}

#[test]
fn a_rule_about_runs_without_after_has_none() {
    let mut source = counter();
    source
        .runs
        .eventually
        .push(eventually("tops", None, equal(field("n"), int(3))));
    let model = lowered(&source);
    let found = model.runs.eventually.first().expect("one rule about runs");
    assert!(found.after.is_none(), "it starts in the start states");
}

#[test]
fn the_fair_actions_are_kept_by_their_position() {
    let mut source = counter();
    source
        .actions
        .push(action("hold", None, record(Vec::new())));
    for (position, each) in source.actions.iter_mut().enumerate() {
        each.fair = position != 1;
    }
    let model = lowered(&source);
    assert_eq!(model.runs.fair, [0, 2], "`bump` and `hold`, not `reset`");
}

#[test]
fn a_model_without_fair_actions_has_none() {
    let model = lowered(&counter());
    assert_eq!(model.runs.fair, Vec::<usize>::new(), "no fair action");
    assert!(model.runs.eventually.is_empty(), "no rule about runs");
}

#[test]
fn rules_about_runs_are_listed_after_the_rules_and_before_the_witnesses() {
    let mut source = counter();
    source.runs.witnesses.push(WitnessDraft {
        id: "starts".to_owned(),
        says: "It starts at zero".to_owned(),
        place: Place {
            line: 40,
            column: 1,
        },
        setup: None,
        steps: Vec::new(),
        state_name: Some("s".to_owned()),
        expect: equal(field("n"), int(0)),
        text: String::new(),
    });
    source
        .runs
        .eventually
        .push(eventually("second", None, equal(field("n"), int(3))));
    source
        .runs
        .eventually
        .push(eventually("first", None, equal(field("n"), int(0))));
    source.rules.push(rule("low", equal(yes(), yes())));
    let model = lowered(&source);
    let ids: Vec<&str> = model.rule_ids().collect();
    assert_eq!(
        ids,
        ["low", "second", "first", "starts"],
        "the rule, the rules about runs in their order, the witness"
    );
}

#[test]
fn what_a_rule_about_runs_reaches_must_be_a_boolean() {
    let mut source = counter();
    source
        .runs
        .eventually
        .push(eventually("tops", None, here(field("n"))));
    let problem = the_problem(&source);
    assert_eq!(problem.file, FILE, "the problem names the draft's file");
    assert_eq!(problem.place(), HERE, "where the condition is written");
}

#[test]
fn where_a_rule_about_runs_starts_must_be_a_boolean() {
    let mut source = counter();
    source.runs.eventually.push(eventually(
        "tops",
        Some(here(field("n"))),
        equal(field("n"), int(3)),
    ));
    let problem = the_problem(&source);
    assert_eq!(problem.place(), HERE, "where `after` is written");
}

#[test]
fn a_rule_about_runs_may_read_no_state() {
    let mut source = counter();
    source
        .runs
        .eventually
        .push(eventually("always", None, yes()));
    let model = lowered(&source);
    let found = model.runs.eventually.first().expect("one rule about runs");
    assert_eq!(
        truth(&found.reaches, found.slots, 2),
        Ok(true),
        "true in every state"
    );
}
