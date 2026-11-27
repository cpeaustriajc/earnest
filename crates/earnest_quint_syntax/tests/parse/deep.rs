use crate::corpus::{DEEP, Equal, recorded, root};
use crate::effects::{Outcome, fnv};
use crate::typecheck::{compared, their_output};
use crate::types::{Typed, compared_types, recorded_types, their_table};
use earnest_quint_syntax::Json;
use std::env;
use std::fs;
use std::path::PathBuf;

const LARGE: u64 = 1024 * 1024;

fn deep_samples() -> Vec<String> {
    let mut deep = Vec::new();
    let base = root();
    let Ok(entries) = fs::read_dir(base.join(DEEP)) else {
        return deep;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "qnt") {
            let name = entry.file_name().to_string_lossy().into_owned();
            deep.push(format!("{DEEP}/{name}"));
        }
    }
    deep.sort();
    deep
}

fn on_a_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        earnest_threads::start(scope, Some(1024 * 1024 * 1024), work)
            .expect("a thread of 1 GiB starts")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
    })
}

fn hashed(text: &str) -> Json {
    Json::Object(vec![
        ("length".to_owned(), Json::Number(text.len().to_string())),
        ("fnv".to_owned(), Json::Text(fnv(text))),
    ])
}

#[test]
#[ignore = "types files nested hundreds deep, for minutes in an unoptimized build: cargo test --release -p earnest_quint_syntax --test parse deep:: -- --ignored"]
fn every_deep_sample_quint_answers_is_read_typed_and_checked_as_quint_writes_it() {
    let (summary, different) = on_a_large_stack(every_deep_sample_compared);
    println!("{summary}");
    assert!(different.is_empty(), "{}", different.join("\n"));
}

fn every_deep_sample_compared() -> (String, Vec<String>) {
    let base = root();
    let found = deep_samples();
    assert!(found.len() >= 10, "{found:?}");
    let mut different = Vec::new();
    for sample in &found {
        match crate::corpus::compared(sample, &recorded(), &base) {
            Ok(Equal::Fully) => {}
            other => different.push(format!("{sample}: parse: {other:?}")),
        }
        match compared_types(sample, &recorded(), &base) {
            Ok(Typed::Equal) => {}
            other => different.push(format!("{sample}: types: {other:?}")),
        }
        match crate::effects::compared(sample, &recorded(), &base) {
            Ok(Outcome::Equal) => {}
            other => different.push(format!("{sample}: effects: {other:?}")),
        }
        match compared(sample, &base, &recorded()) {
            Ok(Outcome::Equal) => {}
            other => different.push(format!("{sample}: typecheck: {other:?}")),
        }
    }
    let summary = format!(
        "{} deep samples read, typed and checked as Quint 0.33.1 writes them, {} differ",
        found.len(),
        different.len()
    );
    (summary, different)
}

#[test]
#[ignore = "writes the length and hash of each large types recording: DEEP_RECORDED=<a folder record.sh and record_types.sh wrote> cargo test -p earnest_quint_syntax --test parse deep::large -- --ignored"]
fn large_type_recordings_are_kept_as_their_length_and_hash() {
    for kept in on_a_large_stack(large_type_recordings_hashed) {
        println!("{kept}");
    }
}

fn large_type_recordings_hashed() -> Vec<String> {
    let mut written = Vec::new();
    let full =
        PathBuf::from(env::var("DEEP_RECORDED").expect("DEEP_RECORDED names the recordings"));
    for sample in deep_samples() {
        let whole = full.join(&sample);
        let path = PathBuf::from(format!("{}.types.json", whole.display()));
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() < LARGE {
            continue;
        }
        let types = recorded_types(&whole)
            .expect("recorded")
            .expect("a types recording");
        let table = their_table(&types).expect("a types table");
        let output = their_output(&whole)
            .expect("a typecheck output")
            .expect("effects recorded");
        let kept = Json::Object(vec![
            ("errors".to_owned(), Json::List(Vec::new())),
            (
                "hashed".to_owned(),
                Json::Object(vec![
                    ("table".to_owned(), hashed(&table.to_string())),
                    ("typecheck".to_owned(), hashed(&output.to_string())),
                ]),
            ),
        ]);
        let target = PathBuf::from(format!("{}.types.json", recorded().join(&sample).display()));
        fs::write(&target, format!("{kept}\n")).expect("written");
        written.push(format!("{sample}: {} bytes kept as {kept}", metadata.len()));
    }
    written
}
