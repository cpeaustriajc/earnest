use crate::corpus::{recorded, root};
use crate::types::compared_types;
use std::time::{Duration, Instant};

const QUINT_S_TIME: Duration = Duration::from_secs(2);

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

#[test]
fn a_hundred_nested_vals_of_ever_deeper_sets_are_typed_in_no_more_time_than_quint_takes() {
    let started = Instant::now();
    assert_eq!(as_quint_types("hundred_nested_vals_of_sets"), Ok(()));
    assert!(started.elapsed() < QUINT_S_TIME, "{:?}", started.elapsed());
}

#[test]
fn a_set_nested_a_hundred_deep_is_typed_in_no_more_time_than_quint_takes() {
    let started = Instant::now();
    assert_eq!(as_quint_types("set_nested_a_hundred_deep"), Ok(()));
    assert!(started.elapsed() < QUINT_S_TIME, "{:?}", started.elapsed());
}
