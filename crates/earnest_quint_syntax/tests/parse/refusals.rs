use crate::corpus::root;
use earnest_quint_syntax::{Problem, read};
use std::collections::HashMap;
use std::fs;

fn refusal(sample: &str) -> Problem {
    let path = format!("crates/earnest_quint_syntax/tests/samples/refused/{sample}.qnt");
    match read(&path, |path| fs::read_to_string(root().join(path)).ok()) {
        Ok(_) => panic!("{sample} is refused"),
        Err(problem) => problem,
    }
}

fn place(problem: &Problem) -> (&str, usize, usize) {
    (problem.code.as_str(), problem.line, problem.column)
}

#[test]
fn an_unclosed_brace_is_refused_where_quint_refuses_it() {
    assert_eq!(
        place(&refusal("unclosed_brace")),
        ("QNT000", 5, 3),
        "at the `val` the brace swallowed"
    );
    assert_eq!(
        place(&refusal("unclosed_module")),
        ("QNT000", 3, 1),
        "at the end of the file"
    );
}

#[test]
fn a_missing_equals_sign_is_refused_where_quint_refuses_it() {
    assert_eq!(
        place(&refusal("missing_equals")),
        ("QNT000", 2, 9),
        "at the value"
    );
    assert_eq!(
        place(&refusal("missing_equals_def")),
        ("QNT000", 2, 22),
        "at the body"
    );
}

#[test]
fn a_keyword_as_a_name_is_refused_where_quint_refuses_it() {
    assert_eq!(
        place(&refusal("keyword_match")),
        ("QNT008", 2, 13),
        "after the reserved word"
    );
    assert_eq!(
        place(&refusal("keyword_import")),
        ("QNT008", 2, 14),
        "after the reserved word"
    );
    assert_eq!(
        place(&refusal("keyword_all")),
        ("QNT000", 2, 7),
        "at the keyword"
    );
    assert_eq!(
        place(&refusal("keyword_if")),
        ("QNT000", 2, 7),
        "at the keyword"
    );
}

#[test]
fn an_unterminated_string_is_refused_where_quint_refuses_it() {
    let problem = refusal("unterminated_string");
    assert_eq!(place(&problem), ("QNT000", 2, 11), "at the opening quote");
    assert_eq!(
        problem.message, "token recognition error at: '\"abc\\n  val y = 1\\n}\\n'",
        "Quint's message names the rest of the file"
    );
}

#[test]
fn a_stray_character_is_refused_where_quint_refuses_it() {
    let expected: HashMap<&str, (usize, usize)> = HashMap::from([
        ("stray_hash", (2, 13)),
        ("stray_at", (2, 13)),
        ("stray_bang", (2, 11)),
        ("stray_unicode", (2, 13)),
    ]);
    for (sample, (line, column)) in expected {
        assert_eq!(
            place(&refusal(sample)),
            ("QNT000", line, column),
            "{sample}"
        );
    }
}

#[test]
fn a_syntax_error_in_an_imported_file_is_unreadable_rather_than_repaired() {
    let files = HashMap::from([
        (
            "/m/main.qnt",
            "module Main {\n  import Broken.* from \"./broken\"\n}\n",
        ),
        ("/m/broken.qnt", "module Broken {\n  val x = (1, \n}\n"),
    ]);
    let Err(problem) = read("/m/main.qnt", |path| {
        files.get(path).map(|text| (*text).to_owned())
    }) else {
        panic!("the import is refused");
    };
    assert_eq!(
        (
            problem.code.as_str(),
            problem.source.as_str(),
            problem.line,
            problem.column
        ),
        ("unreadable", "/m/broken.qnt", 3, 1),
        "the imported file's own error, named as unreadable"
    );
}

#[test]
fn a_file_that_needs_more_attempts_than_the_cap_is_refused_rather_than_guessed() {
    let lets = "val a = ".repeat(15);
    let bodies = "-1 ".repeat(15);
    let text = format!("module A {{\n  val v = {lets}1 {bodies}\n}}\n");
    let Err(problem) = read("/m/deep.qnt", |_| Some(text.clone())) else {
        panic!("the file is refused");
    };
    assert_eq!(
        problem.code, "unreadable",
        "the cap is a refusal: {problem}"
    );
    assert!(
        problem.message.contains("attempts"),
        "the message says why: {}",
        problem.message
    );
}
