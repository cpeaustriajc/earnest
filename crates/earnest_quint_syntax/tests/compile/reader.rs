use crate::compared::{compiled, fixtures, listed, root};
use earnest_check::{Limits, explore};

const MOST_STATES: usize = 20_000;

fn checked(path: &str, source: &str) -> String {
    match earnest_quint::read(path, source) {
        Ok(model) => {
            let limits = Limits {
                max_states: MOST_STATES,
                ..Limits::default()
            };
            format!("{:?}", explore(&model, &limits))
        }
        Err(problems) => format!("unreadable: {problems:?}"),
    }
}

#[test]
fn the_reader_given_the_crate_s_output_checks_the_model_it_checks_given_quint_s() {
    let recorded = listed(&fixtures()).expect("the recordings are read");
    let mut same = 0;
    let mut searched = 0;
    let mut differences = Vec::new();
    let mut field_names_searched = 0;
    for each in &recorded {
        let Ok(theirs) = &each.quint else {
            continue;
        };
        let Ok(Ok(ours)) = compiled(&each.check, &root()) else {
            differences.push(format!("{}: not flattened", each.check.args));
            continue;
        };
        let their_text = theirs.to_string();
        let our_text = ours.output().to_string();
        let their_outcome = checked(&each.check.file, &their_text);
        let our_outcome = checked(&each.check.file, &our_text);
        if their_outcome == our_outcome {
            same += 1;
            if !their_outcome.starts_with("unreadable") {
                searched += 1;
                if each.check.file.ends_with("fieldnames.qnt") {
                    field_names_searched += 1;
                }
            }
        } else {
            differences.push(format!(
                "{}: given Quint's, {}; given ours, {}",
                each.check.args,
                their_outcome.chars().take(400).collect::<String>(),
                our_outcome.chars().take(400).collect::<String>()
            ));
        }
    }
    println!(
        "{same} the same ({searched} searched, the others unreadable both ways), {} differ",
        differences.len()
    );
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert!(searched > 100, "only {searched} searched");
    assert!(
        field_names_searched >= 2,
        "the fieldNames sample was searched {field_names_searched} times"
    );
}
