use super::{TLC_DEADLINE, TLC_JAR_VARIABLE, run_for, tlc_jar};

const SAMPLES: [(&str, &str, &str); 3] = [
    (
        "allasbool.json",
        include_str!("../../../../crates/earnest_quint/fixtures/allasbool.json"),
        "4 states",
    ),
    (
        "actionargument.json",
        include_str!("../../../../crates/earnest_quint/fixtures/actionargument.json"),
        "4 states",
    ),
    (
        "wider.json",
        include_str!("../../../../crates/earnest_quint/fixtures/wider.json"),
        "9 states",
    ),
];

#[test]
fn tlc_confirms_all_of_conditions_and_actions_given_as_arguments() {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    let jar = jar.to_string_lossy().into_owned();
    for (file, source, states) in SAMPLES {
        let confirmed = run_for(
            &[(file, source)],
            &["confirm", "--tlc", &jar, file],
            TLC_DEADLINE,
        )
        .expect("TLC checks a model of 9 states in minutes");
        assert!(
            confirmed.stdout.starts_with("same") && confirmed.stdout.contains(states),
            "TLC finds the {states} and the verdicts the checker finds in {file}\n{}",
            confirmed.stdout
        );
    }
}
