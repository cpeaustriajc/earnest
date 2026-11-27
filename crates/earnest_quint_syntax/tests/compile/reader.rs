use crate::compared::{compiled, fixtures, listed, root};
use earnest_check::{Limits, explore};
use earnest_quint_syntax::Json;

const MOST_STATES: usize = 20_000;

fn with_types(output: &Json, types: Vec<(String, Json)>) -> String {
    let Json::Object(fields) = output else {
        return output.to_string();
    };
    let mut fields = fields.clone();
    let at = fields
        .iter()
        .position(|(key, _)| key == "table")
        .map_or(fields.len(), |at| at + 1);
    fields.insert(at, ("types".to_owned(), Json::Object(types)));
    Json::Object(fields).to_string()
}

fn entries(value: &Json) -> Vec<(String, Json)> {
    match value {
        Json::Object(fields) => fields.clone(),
        _ => Vec::new(),
    }
}

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
    for each in &recorded {
        let Ok(theirs) = &each.quint else {
            continue;
        };
        let Ok(Ok(ours)) = compiled(&each.check, &root()) else {
            differences.push(format!("{}: not flattened", each.check.args));
            continue;
        };
        let mut their_types = entries(&each.variable_types);
        for (id, scheme) in entries(&each.field_names_types) {
            if !their_types.iter().any(|(known, _)| *known == id) {
                their_types.push((id, scheme));
            }
        }
        let their_text = with_types(theirs, their_types);
        let our_text = with_types(&ours.output(), entries(&ours.variable_types()));
        let their_outcome = checked(&each.check.file, &their_text);
        let our_outcome = checked(&each.check.file, &our_text);
        if their_outcome == our_outcome {
            same += 1;
            if !their_outcome.starts_with("unreadable") {
                searched += 1;
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
}
