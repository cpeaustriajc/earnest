use crate::{Labelled, Views, split_views};

const COMPILED: &str = r#"{"stage":"compiling", "modules": [{"name":"m","declarations":[
  {"kind":"def","name":"big","qualifier":"val","expr":{"id":1,"kind":"int","value":18446744073709551616}},
  {"kind":"def","name":"q::inv","qualifier":"val","expr":{"id":4,"kind":"app","opcode":"and","args":[
    {"id":2,"kind":"name","name":"ok"} , {"id":3,"kind":"name","name":"m::A::shown"}]}}
]}], "table": {}}"#;

fn labelled(state: Option<&str>, group: Option<&str>) -> Labelled {
    Labelled {
        state: state.map(str::to_owned),
        group: group.map(str::to_owned),
    }
}

#[test]
fn the_labels_are_cut_out_of_the_text_and_every_other_byte_is_kept() {
    let (text, views) = split_views(COMPILED, &labelled(Some("A::shown"), None))
        .expect("the invariants end with the label");
    assert_eq!(
        views,
        Views {
            state: Some("m::A::shown".to_owned()),
            group: None
        }
    );
    assert_eq!(
        text,
        COMPILED.replace(r#" , {"id":3,"kind":"name","name":"m::A::shown"}"#, ""),
        "the number beyond 64 bits and the spacing are as Quint wrote them"
    );
}

#[test]
fn invariants_made_only_of_labels_are_taken_out_whole() {
    let only = COMPILED.replace(r#"{"id":2,"kind":"name","name":"ok"} , "#, "");
    let (text, _) = split_views(&only, &labelled(Some("shown"), None)).expect("a label");
    assert!(!text.contains("q::inv"), "{text}");
    assert!(text.contains("18446744073709551616"), "{text}");
    assert!(
        serde_json::from_str::<serde_json::Value>(&text).is_ok(),
        "{text}"
    );
}

#[test]
fn invariants_that_do_not_end_with_the_labels_are_not_split() {
    assert_eq!(split_views(COMPILED, &labelled(Some("other"), None)), None);
    assert_eq!(
        split_views(COMPILED, &labelled(Some("ok"), Some("other"))),
        None
    );
    assert_eq!(split_views(COMPILED, &labelled(Some("a"), Some("b"))), None);
    let unlabelled = split_views(COMPILED, &Labelled::default());
    assert_eq!(unlabelled.map(|(text, _)| text), Some(COMPILED.to_owned()));
}
