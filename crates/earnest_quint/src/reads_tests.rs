use crate::read;
use earnest_diagnostics::Problem;
use earnest_ir::{Fault, Frame, Model, Rule};
use earnest_value::Value;

const VALUES: &str = include_str!("../fixtures/values.json");
const READS: &str = include_str!("../fixtures/reads.json");
const ENDLESS: &str = include_str!("../fixtures/endless.json");
const ENDLESS_FOLD: &str = include_str!("../fixtures/endlessfold.json");
const HELPERS: &str = include_str!("../fixtures/helpers.json");
const BIG: &str = include_str!("../fixtures/bigint.json");
const OPEN_FIELDS: &str = include_str!("../fixtures/openfields.json");
const ONLY_FAULT: &str = include_str!("../fixtures/onlyfault.json");

fn model(source: &str) -> Model {
    read("model.json", source).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

fn problems(source: &str) -> Vec<Problem> {
    match read("model.json", source) {
        Ok(found) => panic!("{} was read, but it must be refused", found.name),
        Err(problems) => problems,
    }
}

fn at_the_start<'m>(model: &'m Model, rule: &str) -> (&'m Rule, &'m [Value]) {
    let found = model
        .rules
        .iter()
        .find(|each| each.id == rule)
        .unwrap_or_else(|| panic!("no rule {rule}"));
    let starts = model
        .runs
        .starts
        .as_ref()
        .expect("a Quint model lists its start states");
    (found, starts.first().expect("one start state"))
}

fn evaluated((rule, state): (&Rule, &[Value])) -> Result<Value, Fault> {
    rule.holds.eval(&mut Frame::new(state, rule.slots))
}

fn holds(model: &Model, rule: &str) {
    assert_eq!(
        evaluated(at_the_start(model, rule)),
        Ok(Value::Bool(true)),
        "Quint's TypeScript evaluator gives true for `{rule}`"
    );
}

fn the_problem(source: &str, feature: &str) -> Problem {
    let found = problems(source);
    found
        .iter()
        .find(|problem| problem.feature == feature)
        .cloned()
        .unwrap_or_else(|| panic!("no problem names {feature}: {found:?}"))
}

#[test]
fn bool_is_the_set_of_both_booleans() {
    holds(&model(VALUES), "booleans");
}

#[test]
fn membership_in_nat_and_int_is_a_comparison() {
    holds(&model(VALUES), "natural");
}

#[test]
fn nat_read_as_a_set_is_refused_with_a_message_that_names_it_and_says_it_has_no_end() {
    for (source, name) in [(ENDLESS, "Nat"), (ENDLESS_FOLD, "Int")] {
        let problem = the_problem(source, &format!("`{name}`"));
        assert!(
            problem.message.contains("a set without end")
                && problem.message.contains(&format!("x.in({name})"))
                && !problem.message.contains("does not say what this name is"),
            "the message names {name}, says it has no end and what is read: {problem:?}"
        );
    }
}

#[test]
fn get_only_element_gives_the_item_of_a_set_of_one_and_is_a_fault_on_any_other_set() {
    let values = model(VALUES);
    holds(&values, "only");
    for rule in ["emptyOnly", "twoOnly"] {
        assert_eq!(
            evaluated(at_the_start(&values, rule)),
            Err(Fault::Missing),
            "Quint's TypeScript evaluator stops with QNT505 on `{rule}`"
        );
    }
}

#[test]
fn field_names_gives_the_set_of_the_field_names_as_texts() {
    holds(&model(VALUES), "names");
}

#[test]
fn field_names_of_a_record_whose_type_is_left_open_is_refused_with_what_to_write() {
    let problem = the_problem(OPEN_FIELDS, "`fieldNames`");
    assert!(
        problem.message.contains("give the parameter a record type")
            && problem
                .message
                .contains("in the definition `names`, which `one` uses"),
        "the message says what to write and which check reaches it: {problem:?}"
    );
}

#[test]
fn foldr_folds_from_the_right_with_the_item_first() {
    let values = model(VALUES);
    for rule in ["foldedRight", "foldedRightList", "foldedRightEmpty"] {
        holds(&values, rule);
    }
}

#[test]
fn an_operator_is_given_as_an_argument_as_a_lambda_or_by_name_and_two_levels_down() {
    holds(&model(VALUES), "operators");
}

#[test]
fn q_debug_is_read_as_its_value() {
    holds(&model(VALUES), "debugged");
}

#[test]
fn an_unused_helper_the_reader_does_not_take_does_not_refuse_the_check() {
    let reads = model(READS);
    assert_eq!(
        reads.rules.len(),
        7,
        "`unused` calls `chooseSome` and no check reaches it, so every invariant is read"
    );
}

#[test]
fn a_used_helper_the_reader_does_not_take_is_refused_with_the_names_that_reach_it() {
    let problem = the_problem(HELPERS, "`chooseSome`");
    assert!(
        problem
            .message
            .contains("in the definition `deeper`, which `reached` reaches through `check`"),
        "the problem names the helper and the path from the invariant: {problem:?}"
    );
}

#[test]
fn a_literal_past_64_bits_is_compared_as_the_number_written() {
    holds(&model(BIG), "below");
}

#[test]
fn bool_one_of_in_a_run_and_q_debug_in_an_action_are_read() {
    let faulty = model(ONLY_FAULT);
    assert_eq!(faulty.runs.scripts.len(), 1, "`boolTest` is played");
    assert_eq!(
        faulty.actions[0].inputs[0].values,
        [Value::Bool(false), Value::Bool(true)],
        "`Bool.oneOf()` picks either boolean"
    );
}

const SETUP_FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tasks/quint/setups/fixtures"
);

fn setup_fixture(name: &str) -> String {
    let path = format!("{SETUP_FIXTURES}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path} can't be read: {error}"))
}

#[test]
fn a_setup_built_from_the_typecheck_reads_field_names_bool_nat_foldr_and_operators_as_compiled() {
    let typechecked = crate::Typechecked::read(&setup_fixture("readsetup.typecheck.json"))
        .expect("the typecheck output is read");
    let invariants = vec!["named".to_owned(), "reversed".to_owned()];
    let built = typechecked
        .setup(&crate::Setup::new(
            "ReadSetup",
            ("init", "step"),
            (&invariants, &[]),
            &crate::Runs::None,
        ))
        .expect("the setup is of the plain shape");
    let ours = model(&built);
    let theirs = model(&setup_fixture("readsetup.ReadSetup.json"));
    assert_eq!(ours.runs.starts, theirs.runs.starts);
    assert_eq!(
        ours.rule_ids().collect::<Vec<_>>(),
        theirs.rule_ids().collect::<Vec<_>>()
    );
    for read in [&ours, &theirs] {
        holds(read, "named");
        holds(read, "reversed");
    }
}
