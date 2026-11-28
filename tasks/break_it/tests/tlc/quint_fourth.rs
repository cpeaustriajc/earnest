use super::{TLC_DEADLINE, TLC_JAR_VARIABLE, run_for, tlc_jar};

const GUARDED: [(&str, &str); 3] = [
    (
        "guarded_step.json",
        include_str!("../../models/quint_fourth/guarded_fair/guarded_step.json"),
    ),
    (
        "guarded_branch.json",
        include_str!("../../models/quint_fourth/guarded_fair/guarded_branch.json"),
    ),
    (
        "guarded_fair.json",
        include_str!("../../models/quint_fourth/guarded_fair/guarded_fair.json"),
    ),
];

#[test]
fn tlc_judges_the_export_of_fairness_of_a_guarded_action_as_the_checker_does() {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    let jar = jar.to_string_lossy().into_owned();
    for (file, json) in GUARDED {
        let confirmed = run_for(
            &[(file, json)],
            &["confirm", "--tlc", &jar, file],
            TLC_DEADLINE,
        )
        .expect("TLC checks a model of at most 8 states in minutes");
        assert!(
            confirmed.stdout.starts_with("same") && confirmed.stdout.trim_end().ends_with("pass"),
            "`{file}`: the export states `WF_e(Fair_pay_1)` with `Fair_pay_1` the action `pay` \
             alone, so TLC finds the property holds, as `quint verify --backend tlc` does; the \
             checker says so too. Before, the export wrote the step's guarded `pay` and the \
             checker said broken, and the two agreed on the wrong answer\n{}",
            confirmed.stdout
        );
    }
}
