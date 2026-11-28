//! A tree is plain data: built by hand, compared by value.

use earnest_ast::same_condition_text;
use earnest_ast::{
    HelperKind, Quoting, Reading, Shape, ShapeKind, Term, TermKind, ViewDraft, condition_text,
};
use earnest_diagnostics::Place;
use earnest_value::IntType;

#[test]
fn a_new_term_keeps_its_place_and_kind() {
    let place = Place { line: 4, column: 9 };
    let term = Term::new(place, TermKind::Int(3));
    assert_eq!(term.place, place);
    assert_eq!(term.kind, TermKind::Int(3));
}

#[test]
fn terms_differ_when_only_their_place_differs() {
    let first = Term::new(Place { line: 1, column: 1 }, TermKind::Null);
    let second = Term::new(Place { line: 1, column: 2 }, TermKind::Null);
    assert_ne!(first, second);
}

#[test]
fn a_file_without_models_reads_as_nothing_found_and_nothing_refused() {
    let reading = Reading::default();
    assert!(reading.drafts.is_empty(), "{:?}", reading.drafts);
    assert!(reading.problems.is_empty(), "{:?}", reading.problems);
}

#[test]
fn a_model_without_a_view_has_no_title_labels_steps_or_colors() {
    let view = ViewDraft::default();
    assert_eq!(view.title, None);
    assert_eq!(view.state, None);
    assert_eq!(view.group, None);
    assert!(view.steps.is_empty(), "{:?}", view.steps);
    assert!(view.colors.is_empty(), "{:?}", view.colors);
}

#[test]
fn the_text_of_a_condition_has_one_space_for_each_run_of_white_space() {
    assert_eq!(
        condition_text("  s.n <= 2 &&\n\t\ts.n   >= 0\r\n", Quoting::TypeScript),
        "s.n <= 2 && s.n >= 0"
    );
    assert_eq!(condition_text("s.n<=2", Quoting::Rust), "s.n<=2");
    assert_eq!(condition_text(" \n\t", Quoting::Rust), "");
    assert_eq!(
        condition_text("s.n /* at most */ <= // two\n 2", Quoting::TypeScript),
        "s.n <= 2",
        "a comment is not part of the condition"
    );
    assert_eq!(
        condition_text("s.t === \"a // b\" /* c */", Quoting::TypeScript),
        "s.t === \"a // b\"",
        "text in quotes is kept whole"
    );
    assert_eq!(
        condition_text("self.n /* a /* b */ c */ <= 2", Quoting::Rust),
        "self.n <= 2",
        "a Rust block comment nests"
    );
}

#[test]
fn a_call_of_a_method_is_another_term_than_a_call_of_the_function_of_its_name() {
    let call = |kind: HelperKind| {
        Term::new(
            Place::default(),
            TermKind::Helper {
                kind,
                name: "pick".to_owned(),
                arguments: Vec::new(),
            },
        )
    };
    assert_ne!(call(HelperKind::Function), call(HelperKind::Method));
    assert_eq!(call(HelperKind::Method), call(HelperKind::Method));
}

#[test]
fn a_number_shape_with_an_integer_type_is_another_shape_than_one_without() {
    let number = |ty: Option<IntType>| Shape {
        place: Place::default(),
        kind: ShapeKind::Int { lo: 0, hi: 3, ty },
        initial: None,
    };
    assert_ne!(number(Some(IntType::U8)), number(None));
    assert_ne!(number(Some(IntType::U8)), number(Some(IntType::U16)));
    let declared = Term::new(
        Place::default(),
        TermKind::Declared {
            shape: Box::new(number(Some(IntType::U8))),
            value: Box::new(Term::new(Place::default(), TermKind::Int(1))),
        },
    );
    assert_ne!(
        declared.kind,
        TermKind::Int(1),
        "a value with its type written out is not the bare value"
    );
}

fn same(first: &str, second: &str, quoting: Quoting) -> bool {
    same_condition_text(first, second, quoting)
}

#[test]
fn a_condition_wrapped_before_a_dot_is_the_same_condition() {
    assert!(
        same(
            "s.items.size <= 3",
            "s.items .size <= 3",
            Quoting::TypeScript
        ),
        "a member chain wrapped before `.size`"
    );
    assert!(
        same(
            "(s) => s.items.has(2), }",
            "(s) => s.items .has(2) , }",
            Quoting::TypeScript
        ),
        "a method call wrapped before `.has`"
    );
    assert!(
        same(
            "let after = self.up()?.up()?.up()?;",
            "let after = self .up()? .up()? .up()?;",
            Quoting::Rust
        ),
        "a Rust chain one step a line"
    );
    assert!(
        same("s.n<=2&&s.m>0", "s.n <= 2 && s.m > 0", Quoting::TypeScript),
        "spaces around operators"
    );
    assert!(
        !same("s.n <= 2", "s.n <= 3", Quoting::TypeScript),
        "another number"
    );
}

#[test]
fn a_space_between_two_words_or_in_quoted_text_counts() {
    assert!(
        !same("{ return x; }", "{ returnx; }", Quoting::TypeScript),
        "`return x` is not `returnx`"
    );
    assert!(
        !same("let a = b as u8;", "let a = bas u8;", Quoting::Rust),
        "a space between two words in Rust"
    );
    assert!(
        !same("$a $b", "$a$b", Quoting::TypeScript),
        "`$` is a word letter"
    );
    assert!(
        !same("s.t === \"a b\"", "s.t === \"ab\"", Quoting::TypeScript),
        "a space in a text"
    );
    assert!(
        !same("s.t === 'a .b'", "s.t === 'a.b'", Quoting::TypeScript),
        "a space before a dot in a text"
    );
    assert!(
        !same(
            "s.t === `a ${s.n}`",
            "s.t === `a${s.n}`",
            Quoting::TypeScript
        ),
        "a space in a template literal"
    );
    assert!(
        !same("self.t == \"a b\"", "self.t == \"ab\"", Quoting::Rust),
        "a space in a Rust text"
    );
    assert!(
        !same("self.t == r#\"a b\"#", "self.t == r#\"ab\"#", Quoting::Rust),
        "a space in a Rust raw text"
    );
    assert!(
        !same("self.c == ' '", "self.c == 'x'", Quoting::Rust),
        "a space as a Rust character"
    );
    assert!(
        same(
            "self.t == \"a b\" .len()",
            "self.t == \"a b\".len()",
            Quoting::Rust
        ),
        "a space after a text"
    );
}
