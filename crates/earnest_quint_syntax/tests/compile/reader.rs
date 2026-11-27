use crate::compared::{Outcome, compared, compiled, fixtures, listed, root};
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
fn the_reader_reads_each_flattened_output_that_is_quint_s_byte_for_byte() {
    if crate::ran_on_a_stack_of(
        earnest_check::WORKER_STACK,
        the_reader_reads_each_flattened_output_that_is_quint_s_byte_for_byte,
    ) {
        return;
    }
    let recorded = listed(&fixtures()).expect("the recordings are read");
    let mut read = 0;
    let mut searched = 0;
    let mut field_names_searched = 0;
    for each in &recorded {
        if !matches!(compared(each, &root()), Ok(Outcome::Equal)) {
            continue;
        }
        let Ok(Ok(ours)) = compiled(&each.check, &root()) else {
            continue;
        };
        read += 1;
        let outcome = checked(&each.check.file, &ours.output().to_string());
        if !outcome.starts_with("unreadable") {
            searched += 1;
            if each.check.file.ends_with("fieldnames.qnt")
                || each.check.file.ends_with("records.qnt")
            {
                field_names_searched += 1;
            }
        }
    }
    println!("{read} read, {searched} searched, the others unreadable");
    assert!(searched > 100, "only {searched} searched");
    assert!(
        field_names_searched >= 3,
        "the fieldNames samples were searched {field_names_searched} times"
    );
}
