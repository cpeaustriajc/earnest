use crate::corpus::{TWO_READINGS, compared, recorded, root, samples};
use earnest_quint_syntax::{Json, Problem, read};
use std::fs;
use std::path::Path;

fn as_quint_reads(sample: &str) -> Result<(), String> {
    let path = format!("crates/earnest_quint_syntax/tests/samples/readings/{sample}.qnt");
    compared(&path, &recorded(), &root()).map(drop)
}

fn first_error(path: &str) -> Option<String> {
    let recording = recorded().join(format!("{path}.phases.json"));
    let text = fs::read_to_string(&recording)
        .unwrap_or_else(|_| panic!("{} is recorded", recording.display()));
    let phases =
        Json::parse(&text).unwrap_or_else(|at| panic!("the recording parses, not at {at}"));
    match phases.get("errors") {
        Some(Json::List(errors)) => errors.first().and_then(|first| match first.get("code") {
            Some(Json::Text(code)) => Some(code.clone()),
            _ => None,
        }),
        _ => None,
    }
}

fn refused(path: &str) -> Problem {
    match read(path, |file| fs::read_to_string(root().join(file)).ok()) {
        Ok(_) => panic!("{path} is refused"),
        Err(problem) => problem,
    }
}

fn refusal(text: &str) -> Problem {
    match read("/m/a.qnt", |_| Some(text.to_owned())) {
        Ok(_) => panic!("the file is refused: {text}"),
        Err(problem) => problem,
    }
}

#[test]
fn values_of_nested_lets_ending_at_two_points_are_read_as_quint_reads_them() {
    assert_eq!(as_quint_reads("values_ending_at_two_points"), Ok(()));
}

#[test]
fn a_keyword_that_quints_grammar_can_also_read_as_a_name_is_refused_with_the_value_to_put_in_parentheses()
 {
    let problem = refusal("module G {\n  val v = def l = { true } iff and() [()]\n}\n");
    assert_eq!(
        (problem.code.as_str(), problem.line, problem.column),
        ("unreadable", 2, 28),
        "at the `iff`: {problem}"
    );
    assert!(
        problem
            .message
            .ends_with("put that definition's value, from 2:19 to before 2:37, in parentheses"),
        "the message says what to write: {}",
        problem.message
    );
}

#[test]
fn the_same_value_in_parentheses_is_read_as_quint_reads_it() {
    assert_eq!(as_quint_reads("keyword_in_a_parenthesized_value"), Ok(()));
}

#[test]
fn an_infix_keyword_quint_reads_as_a_name_is_refused_rather_than_read_another_way() {
    let problem = refusal("module G{val w1 def g=(val{l3173}=1-nondet l3175=1iff\"s\"-{w1})}");
    assert_eq!(
        problem.code, "unreadable",
        "Quint refuses it with QNT008: {problem}"
    );
}

#[test]
fn a_match_quint_reads_as_a_call_of_a_name_is_refused_rather_than_read_another_way() {
    let problem = refusal("module G{val g=val l2518=match({()}){_=>false}-{false}}");
    assert_eq!(
        problem.code, "unreadable",
        "Quint refuses it with QNT008: {problem}"
    );
}

#[test]
fn the_two_files_quint_refuses_where_it_reads_a_keyword_before_a_parenthesis_as_a_name_are_refused()
{
    for sample in [
        "keyword_call_quint_reads_after_an_infix_operator",
        "match_quint_reads_as_a_name_before_an_operator",
    ] {
        let path = format!("{TWO_READINGS}/{sample}.qnt");
        let quint = first_error(&path).unwrap_or_else(|| panic!("Quint refuses {sample}"));
        assert_eq!(quint, "QNT008", "Quint refuses {sample} with QNT008");
        let problem = refused(&path);
        assert!(
            problem.code == "unreadable" && problem.message.contains("two ways"),
            "earnest refuses {sample} as having two readings: {problem}"
        );
    }
}

#[test]
fn a_keyword_call_quint_reads_as_a_call_is_refused_where_it_could_also_continue_the_value_before_it()
 {
    let path = format!("{TWO_READINGS}/keyword_call_quint_reads_as_a_call.qnt");
    assert_eq!(first_error(&path), None, "Quint reads it");
    let problem = refused(&path);
    assert_eq!(
        (problem.code.as_str(), problem.line, problem.column),
        ("unreadable", 1, 147),
        "at the `and`: {problem}"
    );
    assert!(
        problem
            .message
            .ends_with("put that call, from 1:147 to before 1:165, in parentheses"),
        "the message says what to write: {}",
        problem.message
    );
}

#[test]
fn the_same_call_in_parentheses_is_read_as_quint_reads_it() {
    assert_eq!(as_quint_reads("keyword_call_in_parentheses"), Ok(()));
}

#[test]
fn every_file_quint_reads_two_ways_is_refused_as_having_two_readings() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new(TWO_READINGS), &mut found);
    assert!(found.len() >= 3, "the samples are found: {found:?}");
    for path in found {
        let problem = refused(&path);
        assert!(
            problem.code == "unreadable" && problem.message.contains("two ways"),
            "{path} is refused as having two readings: {problem}"
        );
        if let Some(code) = first_error(&path) {
            assert_eq!(
                code, "QNT008",
                "where Quint refuses {path}, it is with QNT008"
            );
        }
    }
}
