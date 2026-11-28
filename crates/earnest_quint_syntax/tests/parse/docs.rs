use crate::corpus::root;
use earnest_quint_syntax::{Json, read};
use std::fs;

fn module(name: &str) -> Json {
    let parsed = read(
        "crates/earnest_quint_syntax/tests/samples/docs.qnt",
        |path| fs::read_to_string(root().join(path)).ok(),
    )
    .expect("the doc samples are read");
    parsed
        .modules()
        .iter()
        .find(|module| module.get("name") == Some(&Json::Text(name.to_owned())))
        .cloned()
        .expect("the module is there")
}

fn doc_of(module: &Json, name: &str) -> Option<String> {
    let Some(Json::List(declarations)) = module.get("declarations") else {
        return None;
    };
    let declaration = declarations
        .iter()
        .find(|declaration| {
            declaration.get("name") == Some(&Json::Text(name.to_owned()))
                || declaration.get("qualifiedName") == Some(&Json::Text(name.to_owned()))
        })
        .expect("the declaration is there");
    match declaration.get("doc") {
        Some(Json::Text(doc)) => Some(doc.clone()),
        _ => None,
    }
}

#[test]
fn a_module_s_doc_is_its_lines_without_the_slashes() {
    let doc = match module("Docs").get("doc") {
        Some(Json::Text(doc)) => doc.clone(),
        _ => String::new(),
    };
    assert_eq!(doc, "A module's doc.\nOver two lines.", "the module's doc");
}

#[test]
fn an_at_check_written_against_the_slashes_arrives_as_check() {
    assert_eq!(
        doc_of(&module("Docs"), "checked").as_deref(),
        Some("check"),
        "///@check loses its first four characters"
    );
}

#[test]
fn a_blank_doc_line_ends_the_doc_with_a_newline() {
    assert_eq!(
        doc_of(&module("Docs"), "blankEnds").as_deref(),
        Some("@check\n"),
        "the blank line adds a newline"
    );
}

#[test]
fn a_line_comment_or_a_block_comment_gives_no_doc() {
    let docs = module("Docs");
    assert_eq!(doc_of(&docs, "lineComment"), None, "// is not a doc");
    assert_eq!(doc_of(&docs, "blockComment"), None, "/* */ is not a doc");
}

#[test]
fn a_doc_of_one_blank_line_is_no_doc() {
    assert_eq!(
        doc_of(&module("Docs"), "onlyBlank"),
        None,
        "/// alone gives nothing"
    );
}

#[test]
fn a_definition_an_instance_a_constant_and_a_run_keep_their_docs() {
    let docs = module("Docs");
    assert_eq!(
        doc_of(&docs, "C").as_deref(),
        Some("on a const"),
        "the constant's doc"
    );
    assert_eq!(
        doc_of(&docs, "v").as_deref(),
        Some("on a var"),
        "the variable's doc"
    );
    assert_eq!(
        doc_of(&docs, "aRun").as_deref(),
        Some("on a run"),
        "the run's doc"
    );
    assert_eq!(
        doc_of(&module("Other"), "D").as_deref(),
        Some("on an instance"),
        "the instance's doc"
    );
}

#[test]
fn a_sum_type_s_doc_lands_on_its_last_constructor() {
    let docs = module("Docs");
    assert_eq!(doc_of(&docs, "Answer"), None, "the type itself has none");
    assert_eq!(
        doc_of(&docs, "No").as_deref(),
        Some("on a sum type: it lands on the last constructor"),
        "the last constructor has it"
    );
}
