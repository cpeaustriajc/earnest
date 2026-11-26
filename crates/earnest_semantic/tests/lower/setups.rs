use crate::build::{
    FILE, HERE, action, draft, equal, field, here, int, lowered, one_of, range, record, starting,
    text, the_problem,
};
use earnest_ast::{Draft, SetupDraft, Term, WitnessDraft};
use earnest_diagnostics::{Place, Problem};
use earnest_value::Value;

const NAME_PLACE: Place = Place {
    line: 26,
    column: 30,
};

fn setup(id: &str, starts: Vec<(&str, Term)>) -> SetupDraft {
    SetupDraft {
        id: id.to_owned(),
        says: format!("The clock is {id}"),
        place: Place {
            line: 25,
            column: 1,
        },
        starts: starts
            .into_iter()
            .map(|(name, value)| (name.to_owned(), HERE, value))
            .collect(),
        text: String::new(),
    }
}

fn witness(id: &str, setup: Option<&str>) -> WitnessDraft {
    WitnessDraft {
        id: id.to_owned(),
        says: format!("{id} is played"),
        place: Place {
            line: 41,
            column: 1,
        },
        setup: setup.map(|named| (named.to_owned(), NAME_PLACE)),
        steps: Vec::new(),
        state_name: Some("s".to_owned()),
        expect: equal(field("hand"), int(0)),
        text: String::new(),
    }
}

fn clock() -> Draft {
    let mut source = draft(vec![
        ("skew", starting(range(-10, 10), int(0))),
        ("hand", starting(range(0, 2), int(0))),
        ("mode", starting(one_of(&["auto", "manual"]), text("auto"))),
    ]);
    source.actions = vec![action("tick", None, record(vec![("hand", int(1))]))];
    source
}

fn with_setups(setups: Vec<SetupDraft>) -> Draft {
    let mut source = clock();
    source.runs.setups = setups;
    source
}

fn problem_of(setups: Vec<SetupDraft>) -> Problem {
    let problem = the_problem(&with_setups(setups));
    assert_eq!(problem.file, FILE, "the problem names the draft's file");
    problem
}

fn starts_of(model: &earnest_ir::Model) -> Vec<Vec<String>> {
    model
        .fields
        .iter()
        .map(|each| {
            each.starts
                .iter()
                .map(|value| model.symbols.show(value))
                .collect()
        })
        .collect()
}

#[test]
fn a_setup_keeps_its_id_sentence_line_and_values_by_field_position() {
    let model = lowered(&with_setups(vec![setup(
        "manual",
        vec![("mode", text("manual")), ("skew", int(-10))],
    )]));
    let found = model.runs.setups.first().expect("one setup");
    assert_eq!(found.id, "manual", "the id");
    assert_eq!(found.says, "The clock is manual", "the sentence");
    assert_eq!(found.line, 25, "the line of the declaration");
    let starts: Vec<(usize, String)> = found
        .starts
        .iter()
        .map(|(position, value)| (*position, model.symbols.show(value)))
        .collect();
    assert_eq!(
        starts,
        [(2, "\"manual\"".to_owned()), (0, "-10".to_owned())],
        "each field by its position, in the order written"
    );
    assert_eq!(model.runs.chosen, None, "the model as declared");
    assert_eq!(model.setup(), None, "no setup is chosen");
    assert_eq!(model.run_name(), "Cart", "the model's own name");
}

#[test]
fn a_model_in_a_setup_starts_the_named_fields_at_the_setup_values() {
    let model = lowered(&with_setups(vec![
        setup("behind", vec![("skew", int(-10))]),
        setup("ahead", vec![("skew", int(10))]),
    ]));
    let ahead = model.in_setup(1).expect("the second setup");
    assert_eq!(ahead.runs.chosen, Some(1), "the setup's position");
    assert_eq!(
        ahead.setup().map(|chosen| chosen.id.as_str()),
        Some("ahead"),
        "the setup it starts in"
    );
    assert_eq!(
        ahead.run_name(),
        "Cart (ahead)",
        "the name for a result line"
    );
    assert_eq!(
        starts_of(&ahead),
        [vec!["10"], vec!["0"], vec!["\"auto\""]],
        "`skew` starts at the setup's value, the others where the model declares"
    );
    assert_eq!(
        starts_of(&model),
        [vec!["0"], vec!["0"], vec!["\"auto\""]],
        "the model as declared is unchanged"
    );
    assert!(model.in_setup(2).is_none(), "there is no third setup");
}

#[test]
fn a_setup_starts_a_field_with_several_start_values_at_one() {
    let mut source = with_setups(vec![setup("fixed", vec![("hand", int(2))])]);
    if let Some(hand) = source.fields.get_mut(1) {
        hand.shape.initial = None;
    }
    let model = lowered(&source);
    assert_eq!(
        starts_of(&model).get(1).map(Vec::len),
        Some(3),
        "the model starts `hand` at 0, 1 and 2"
    );
    let fixed = model.in_setup(0).expect("the setup");
    assert_eq!(
        starts_of(&fixed).get(1),
        Some(&vec!["2".to_owned()]),
        "the setup starts it at 2 alone"
    );
    assert_eq!(
        fixed.fields.get(1).map(|hand| hand.starts.clone()),
        Some(vec![Value::Int(2)]),
        "one start value"
    );
}

#[test]
fn a_witness_is_listed_only_in_the_run_of_its_setup() {
    let mut source = with_setups(vec![
        setup("behind", vec![("skew", int(-10))]),
        setup("ahead", vec![("skew", int(10))]),
    ]);
    source.runs.witnesses = vec![
        witness("own", None),
        witness("late", Some("behind")),
        witness("early", Some("ahead")),
    ];
    let model = lowered(&source);
    let setups: Vec<Option<usize>> = model.runs.witnesses.iter().map(|each| each.setup).collect();
    assert_eq!(
        setups,
        [None, Some(0), Some(1)],
        "each witness names its setup by position"
    );
    let ids = |run: &earnest_ir::Model| run.rule_ids().map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(ids(&model), ["own"], "the model's own run");
    assert_eq!(
        ids(&model.in_setup(0).expect("behind")),
        ["late"],
        "the run behind"
    );
    assert_eq!(
        ids(&model.in_setup(1).expect("ahead")),
        ["early"],
        "the run ahead"
    );
}

#[test]
fn a_witness_setup_the_model_does_not_have_is_refused() {
    let mut source = with_setups(vec![setup("behind", vec![("skew", int(-10))])]);
    source.runs.witnesses = vec![witness("late", Some("later"))];
    let problem = the_problem(&source);
    assert_eq!(problem.place(), NAME_PLACE, "where the setup is named");
    assert_eq!(problem.feature, "the setup `later`", "the setup");
    assert_eq!(
        problem.message, "the model has no setup with this id",
        "what is wrong"
    );
}

#[test]
fn a_setup_naming_a_field_the_model_lacks_is_refused() {
    let problem = problem_of(vec![setup("wrong", vec![("drift", int(1))])]);
    assert_eq!(problem.place(), HERE, "where the field is named");
    assert_eq!(problem.feature, "the field `drift`", "the field");
    assert_eq!(
        problem.message, "the model has no field with this name",
        "what is wrong"
    );
}

#[test]
fn a_setup_naming_a_field_twice_is_refused() {
    let problem = problem_of(vec![setup(
        "twice",
        vec![("skew", int(1)), ("hand", int(1)), ("skew", int(2))],
    )]);
    assert_eq!(problem.place(), HERE, "where the field is named again");
    assert_eq!(problem.feature, "the field `skew`", "the field");
    assert_eq!(
        problem.message, "the setup starts this field twice",
        "what is wrong"
    );
}

#[test]
fn a_setup_value_outside_the_field_declaration_is_refused() {
    let number = problem_of(vec![setup("far", vec![("skew", here(int(11)))])]);
    assert_eq!(number.place(), HERE, "where the value is written");
    assert_eq!(number.feature, "the starting value 11", "the value");
    assert_eq!(
        number.message, "it is outside the values the field declares",
        "what is wrong"
    );
    let choice = problem_of(vec![setup("odd", vec![("mode", here(text("off")))])]);
    assert_eq!(choice.place(), HERE, "where the choice is written");
    assert!(
        choice.feature.contains("off"),
        "the choice the field does not declare: {choice:?}"
    );
}

#[test]
fn a_setup_value_of_the_wrong_type_is_refused() {
    let problem = problem_of(vec![setup("typed", vec![("hand", here(text("two")))])]);
    assert_eq!(problem.place(), HERE, "where the value is written");
}
