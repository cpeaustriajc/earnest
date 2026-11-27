use crate::corpus::{compared, recorded, root};

fn as_quint_reads(sample: &str) -> Result<(), String> {
    let path = format!("crates/earnest_quint_syntax/tests/samples/review/{sample}.qnt");
    compared(&path, &recorded(), &root()).map(drop)
}

#[test]
fn a_tupled_lambda_of_one_name_is_refused_as_quint_refuses_it() {
    assert_eq!(as_quint_reads("tupled_lambda_of_one"), Ok(()));
}

#[test]
fn set_applied_to_two_types_is_read_as_quint_s_phases_read_it() {
    assert_eq!(as_quint_reads("set_applied_to_two"), Ok(()));
}
