use super::{TLC_DEADLINE, TLC_JAR_VARIABLE, run_for, tlc_jar};

const VARIANTS: &str = include_str!("../../models/quint_second/confirm/variant_compare.json");

#[test]
fn tlc_compares_variants_whose_values_have_different_types_in_the_export() {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    let jar = jar.to_string_lossy().into_owned();
    let confirmed = run_for(
        &[("variant_compare.json", VARIANTS)],
        &["confirm", "--tlc", &jar, "variant_compare.json"],
        TLC_DEADLINE,
    )
    .expect("TLC checks a model of 2 states in minutes");
    assert!(
        confirmed.stdout.starts_with("same") && confirmed.stdout.contains("2 states"),
        "`holder == Nobody or holder == Held(\"a\")` holds in both states, as the checker and \
         `quint verify --backend tlc` say; the export's `MapSet(map, key, value)` makes TLC meet \
         `value` before `tag`, so TLC compares `\"a\"` with `<<>>` first and stops (`Attempted to \
         check equality of string \"a\" with non-string`)\n{}",
        confirmed.stdout
    );
}
