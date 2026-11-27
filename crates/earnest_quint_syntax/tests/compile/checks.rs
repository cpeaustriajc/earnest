use crate::compared::{Outcome, compared, fixtures, listed, root};
use std::env;
use std::path::{Path, PathBuf};

struct Tally {
    equal: usize,
    refused: usize,
    types_left: usize,
    differences: Vec<String>,
}

fn every(folder: &Path, base: &Path) -> Tally {
    let recorded = listed(folder).expect("the recordings are read");
    let mut tally = Tally {
        equal: 0,
        refused: 0,
        types_left: 0,
        differences: Vec::new(),
    };
    for each in &recorded {
        match compared(each, base) {
            Ok(Outcome::Equal) => tally.equal += 1,
            Ok(Outcome::BothRefuse) => tally.refused += 1,
            Ok(Outcome::TypesLeft) => tally.types_left += 1,
            Err(found) => tally
                .differences
                .push(format!("{}: {found}", each.check.args)),
        }
    }
    tally
}

fn said(tally: &Tally) -> String {
    format!(
        "{} equal, {} refused by both, {} refused by Quint's type or effect pass (left to it), {} differ",
        tally.equal,
        tally.refused,
        tally.types_left,
        tally.differences.len()
    )
}

#[test]
fn every_check_of_the_repository_flattens_as_quint_compile_writes_it() {
    let tally = every(&fixtures(), &root());
    println!("{}", said(&tally));
    assert!(
        tally.differences.is_empty(),
        "{}:\n{}",
        said(&tally),
        tally.differences.join("\n")
    );
    assert!(tally.equal >= 200, "{}", said(&tally));
}

#[test]
#[ignore = "needs a folder of recordings made outside the repository"]
fn every_check_of_a_folder_of_recordings_flattens_as_quint_compile_writes_it() {
    let folder =
        PathBuf::from(env::var("QUINT_COMPILED").expect("QUINT_COMPILED names the recordings"));
    let base = env::var("QUINT_COMPILED_BASE").map_or_else(|_| root(), PathBuf::from);
    let tally = every(&folder, &base);
    println!("{}", said(&tally));
    assert!(
        tally.differences.is_empty(),
        "{}",
        tally.differences.join("\n")
    );
}
