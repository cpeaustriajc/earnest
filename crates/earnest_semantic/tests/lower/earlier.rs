use crate::build::HERE;
use crate::build::{
    binary, draft, field, first_state, helper, helper_call, here, int, name, range, rule, starting,
    text,
};
use earnest_ast::{Binary, Draft};
use earnest_ast::{ConditionDraft, EventuallyDraft, SetupDraft, Term};
use earnest_ir::{Frame, Model, Rule};
use earnest_semantic::lower_with_earlier;
use earnest_semantic::lower_with_earlier_parts;

fn tank(level: i64) -> Draft {
    draft(vec![("level", starting(range(0, 9), int(level)))])
}

fn capped(max: i64) -> Draft {
    let mut source = tank(3);
    source.constants.push(("MAX".to_owned(), int(max)));
    source.rules.push(rule(
        "capped",
        binary(Binary::LessOrEqual, field("level"), name("MAX")),
    ));
    source
}

fn in_the_start_state(model: &Model, rule: &Rule) -> bool {
    let state = first_state(model);
    rule.holds
        .truth(&mut Frame::new(&state, rule.slots))
        .expect("the rule has a value")
}

#[test]
fn an_earlier_rule_reads_the_constants_of_its_own_draft() {
    let (model, earlier) = lower_with_earlier(&capped(3), &capped(2)).expect("both are readable");
    let was = earlier[0].as_ref().expect("the earlier rule is readable");
    assert!(
        in_the_start_state(&model, &model.rules[0]),
        "level 3 is at most the new MAX of 3"
    );
    assert!(
        !in_the_start_state(&model, was),
        "level 3 is above the earlier MAX of 2"
    );
}

#[test]
fn an_earlier_rule_calls_the_helpers_of_its_own_draft() {
    let allowed = |most: i64| {
        let mut source = tank(3);
        let body = binary(Binary::LessOrEqual, name("level"), int(most));
        source.helpers.push(helper("allowed", &["level"], body));
        source.rules.push(rule(
            "checked",
            helper_call("allowed", vec![field("level")]),
        ));
        source
    };
    let (model, earlier) = lower_with_earlier(&allowed(3), &allowed(2)).expect("readable");
    let was = earlier[0].as_ref().expect("the earlier rule is readable");
    assert!(in_the_start_state(&model, &model.rules[0]), "3 <= 3");
    assert!(!in_the_start_state(&model, was), "3 <= 2 is false");
}

#[test]
fn an_earlier_rule_reads_the_fields_of_the_new_draft_and_shares_its_texts() {
    let moody = |mood: &str| {
        let mut source = draft(vec![(
            "mood",
            starting(crate::build::one_of(&["calm", "cross"]), text("cross")),
        )]);
        source.rules.push(rule(
            "mood",
            binary(Binary::Equal, field("mood"), text(mood)),
        ));
        source
    };
    let (model, earlier) = lower_with_earlier(&moody("cross"), &moody("calm")).expect("readable");
    let was = earlier[0].as_ref().expect("the earlier rule is readable");
    assert!(
        in_the_start_state(&model, &model.rules[0]),
        "the mood is cross"
    );
    assert!(!in_the_start_state(&model, was), "the mood is not calm");
}

#[test]
fn an_earlier_rule_that_cant_be_read_against_the_new_fields_is_none_and_no_error() {
    let mut old = tank(0);
    old.rules.push(rule(
        "deep",
        binary(Binary::Less, here(field("depth")), int(2)),
    ));
    old.rules
        .push(rule("low", binary(Binary::Less, field("level"), int(2))));
    let (_, earlier) = lower_with_earlier(&tank(0), &old).expect("the new draft is readable");
    assert!(earlier[0].is_none(), "the new model has no field `depth`");
    assert!(earlier[1].is_some(), "the other rule is still read");
}

#[test]
fn the_problems_of_the_new_draft_are_the_error() {
    let mut new = tank(0);
    new.rules.push(rule(
        "deep",
        binary(Binary::Less, here(field("depth")), int(2)),
    ));
    let problems = lower_with_earlier(&new, &tank(0)).expect_err("the new draft is unreadable");
    assert_eq!(problems.len(), 1, "one problem: {problems:?}");
    assert_eq!(problems[0].feature, "the state field `depth`");
}

fn reaching(goal: Term) -> EventuallyDraft {
    EventuallyDraft {
        id: "reaches".to_owned(),
        says: "the level reaches the goal".to_owned(),
        place: HERE,
        after: None,
        reaches: ConditionDraft {
            state_name: Some("s".to_owned()),
            term: binary(Binary::Equal, field("level"), goal),
            text: "same".to_owned(),
        },
    }
}

fn starting_at(start: Term) -> SetupDraft {
    SetupDraft {
        id: "high".to_owned(),
        says: "the level starts high".to_owned(),
        place: HERE,
        starts: vec![("level".to_owned(), HERE, start)],
        text: "same".to_owned(),
    }
}

fn goal(value: i64) -> Draft {
    let mut source = tank(0);
    source.constants.push(("GOAL".to_owned(), int(value)));
    source.runs.eventually.push(reaching(name("GOAL")));
    source.runs.setups.push(starting_at(name("GOAL")));
    source
}

#[test]
fn earlier_rules_about_runs_and_setups_read_the_constants_of_their_own_draft() {
    let (model, earlier) = lower_with_earlier_parts(&goal(3), &goal(2)).expect("both are readable");
    let was = earlier.eventually[0]
        .as_ref()
        .expect("the earlier rule about runs is readable");
    let now = &model.runs.eventually[0];
    assert_ne!(
        was.reaches, now.reaches,
        "the earlier GOAL is 2, the new one 3"
    );
    let setup = earlier.setups[0]
        .as_ref()
        .expect("the earlier setup is readable");
    assert_ne!(
        setup.starts, model.runs.setups[0].starts,
        "it starts at the earlier GOAL"
    );
    let (model, same) = lower_with_earlier_parts(&goal(3), &goal(3)).expect("readable");
    let was = same.eventually[0].as_ref().expect("readable");
    assert_eq!(
        was.reaches, model.runs.eventually[0].reaches,
        "the same GOAL"
    );
    assert_eq!(
        same.setups[0].as_ref().map(|setup| &setup.starts),
        Some(&model.runs.setups[0].starts)
    );
}

#[test]
fn an_earlier_setup_of_a_field_the_new_model_lacks_is_none() {
    let mut old = goal(2);
    old.runs.setups[0].starts[0].0 = "depth".to_owned();
    let (_, earlier) = lower_with_earlier_parts(&goal(2), &old).expect("the new draft is readable");
    assert!(
        earlier.setups[0].is_none(),
        "the new model has no field `depth`"
    );
    assert!(
        earlier.eventually[0].is_some(),
        "the rule about runs is still read"
    );
}
