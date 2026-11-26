use super::{TLC_DEADLINE, TLC_JAR_VARIABLE, run_for, tlc_jar};

const SAMPLE: &str = include_str!("../../../../crates/earnest_quint/fixtures/reads.json");

#[test]
fn tlc_confirms_the_sample_of_every_quint_construct_read_since_issue_64() {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    let jar = jar.to_string_lossy().into_owned();
    let confirmed = run_for(
        &[("reads.json", SAMPLE)],
        &["confirm", "--tlc", &jar, "reads.json"],
        TLC_DEADLINE,
    )
    .expect("TLC checks a model of 8 states in minutes");
    assert!(
        confirmed.stdout.starts_with("same") && confirmed.stdout.contains("8 states"),
        "TLC finds the 8 states and the broken `small` that the checker finds\n{}",
        confirmed.stdout
    );
}
