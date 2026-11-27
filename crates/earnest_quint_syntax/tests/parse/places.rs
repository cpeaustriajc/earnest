use crate::corpus::{compared, recorded, root};
use earnest_quint_syntax::{Problem, read};
use std::fs;

fn path(sample: &str) -> String {
    format!("crates/earnest_quint_syntax/tests/samples/places/{sample}.qnt")
}

fn refusal(sample: &str) -> Problem {
    assert_eq!(
        compared(&path(sample), &recorded(), &root()).map(drop),
        Ok(()),
        "{sample} is refused as Quint 0.33.1 refuses it"
    );
    match read(&path(sample), |file| {
        fs::read_to_string(root().join(file)).ok()
    }) {
        Ok(_) => panic!("{sample} is refused"),
        Err(problem) => problem,
    }
}

fn place(problem: &Problem) -> (&str, usize, usize) {
    (problem.code.as_str(), problem.line, problem.column)
}

#[test]
fn a_dangling_operator_after_a_nondet_on_one_line_is_refused_at_the_closing_brace() {
    assert_eq!(
        place(&refusal("nondet_then_dangling_operator")),
        ("QNT000", 4, 55)
    );
}

#[test]
fn a_dangling_operator_on_the_line_after_a_nondet_is_refused_at_the_closing_brace() {
    assert_eq!(
        place(&refusal("nondet_then_dangling_operator_on_lines")),
        ("QNT000", 7, 3)
    );
}

#[test]
fn a_stray_doc_line_in_an_all_after_a_nondet_is_refused_at_the_doc_line() {
    assert_eq!(
        place(&refusal("nondet_then_stray_doc_in_all")),
        ("QNT000", 9, 7)
    );
}

#[test]
fn a_dangling_operator_after_a_local_def_keeps_quints_place_and_code() {
    assert_eq!(
        place(&refusal("local_def_then_dangling_operator")),
        ("QNT000", 7, 3)
    );
}

#[test]
fn a_brace_left_open_before_a_def_is_refused_at_the_doc_line_with_quints_code() {
    assert_eq!(
        place(&refusal("brace_left_open_before_a_def")),
        ("QNT000", 8, 3)
    );
}

#[test]
fn a_dangling_operator_after_a_val_is_refused_at_the_closing_brace() {
    assert_eq!(
        place(&refusal("val_then_dangling_operator")),
        ("QNT000", 4, 52)
    );
}

#[test]
fn a_keyword_list_left_open_after_a_pure_val_still_reads_the_keyword_as_a_name_as_quint_does() {
    assert_eq!(
        place(&refusal("pure_val_then_keyword_list_left_open")),
        ("QNT008", 6, 9)
    );
}

#[test]
fn a_doc_comment_named_in_a_syntax_error_is_shown_without_its_line_end() {
    let problem = refusal("brace_left_open_before_a_def");
    assert_eq!(
        problem.message,
        "unexpected '/// @check', expected an expression"
    );
}

#[test]
fn a_minus_before_an_if_whose_else_branch_ends_before_an_and_list_is_read_as_quint_reads_it() {
    assert_eq!(
        compared(
            &path("minus_then_if_whose_else_ends_before_and"),
            &recorded(),
            &root()
        )
        .map(drop),
        Ok(())
    );
}
