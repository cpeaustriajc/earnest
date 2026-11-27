use crate::corpus::{compared, recorded, root};
use earnest_quint_syntax::{Problem, read};

fn as_quint_reads(sample: &str) -> Result<(), String> {
    let path = format!("crates/earnest_quint_syntax/tests/samples/readings/{sample}.qnt");
    compared(&path, &recorded(), &root()).map(drop)
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
fn the_two_files_known_to_be_read_where_quint_reads_a_keyword_before_a_parenthesis_as_a_name_are_still_read()
 {
    let known = [
        "module G{val w0 def f()def g=val l59877=def l59878={{false}}implies(if(w0)f else{()})val l59880=val l59881={\"s\"}()and(def l59882()f)val v1}",
        "module G{var y:int val w0 val w1 def g=nondet l59412=nondet l59413={y}-if(w0)w0 else{w1}match match and({()}){_=>y}[if(\"s\")w1 else false]{_=>match{1}{_=>false}}}",
    ];
    for text in known {
        assert!(
            read("/m/a.qnt", |_| Some(text.to_owned())).is_ok(),
            "Quint refuses it with QNT008; the README lists it as read, so a change here moves that count: {text}"
        );
    }
}
