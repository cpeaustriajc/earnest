use crate::corpus::{recorded, root};
use crate::types::compared_types;

fn as_quint_types(sample: &str) -> Result<(), String> {
    let path = format!("crates/earnest_quint_syntax/tests/samples/review/typing/{sample}.qnt");
    compared_types(&path, &recorded(), &root()).map(drop)
}

#[test]
fn a_row_variable_inside_a_field_of_its_own_record_is_unreadable_as_quint_overflows() {
    assert_eq!(as_quint_types("row_inside_its_own_field"), Ok(()));
}

#[test]
fn a_record_set_as_a_field_of_itself_after_another_with_is_unreadable_as_quint_overflows() {
    assert_eq!(as_quint_types("with_of_itself_after_with"), Ok(()));
}
