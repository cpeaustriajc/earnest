use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use super::{TLC_JAR_VARIABLE, earnest, tlc_jar};

const EXPECTED: &str = include_str!("../../models/quint_first/random/expected.json");

const COMPILED: [(&str, &str); 10] = [
    (
        "G4.json",
        include_str!("../../models/quint_first/random/G4.json"),
    ),
    (
        "G6.json",
        include_str!("../../models/quint_first/random/G6.json"),
    ),
    (
        "G16.json",
        include_str!("../../models/quint_first/random/G16.json"),
    ),
    (
        "G21.json",
        include_str!("../../models/quint_first/random/G21.json"),
    ),
    (
        "G25.json",
        include_str!("../../models/quint_first/random/G25.json"),
    ),
    (
        "G28.json",
        include_str!("../../models/quint_first/random/G28.json"),
    ),
    (
        "G29.json",
        include_str!("../../models/quint_first/random/G29.json"),
    ),
    (
        "G33.json",
        include_str!("../../models/quint_first/random/G33.json"),
    ),
    (
        "G47.json",
        include_str!("../../models/quint_first/random/G47.json"),
    ),
    (
        "G56.json",
        include_str!("../../models/quint_first/random/G56.json"),
    ),
];

fn said(folder: &TempDir, arguments: &[&str]) -> (String, bool) {
    let output = Command::new(earnest())
        .args(arguments)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (text, output.status.success())
}

fn differences(file: &str, json: &str, expected: &Value, jar: &str) -> Vec<String> {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(folder.path().join(file), json).expect("the compiled module can be written");
    let (checked, _) = said(&folder, &["verify", "--json", "--no-cache", file]);
    let report: Value = serde_json::from_str(&checked).unwrap_or(Value::Null);
    let model = &report["models"][0];
    let mut found = Vec::new();
    if model["states"] != expected["states"] {
        found.push(format!(
            "{file}: {} states, and Quint's TLC run finds {}",
            model["states"], expected["states"]
        ));
    }
    let rules = expected["rules"].as_object().cloned().unwrap_or_default();
    for (id, verdict) in rules {
        let ours = model["rules"]
            .as_array()
            .and_then(|all| all.iter().find(|rule| rule["id"] == id.as_str()))
            .map_or(Value::Null, |rule| rule["verdict"].clone());
        if ours != verdict {
            found.push(format!(
                "{file}: `{id}` is {ours}, and Quint's TLC run says {verdict}"
            ));
        }
    }
    let (confirmed, success) = said(&folder, &["confirm", "--tlc", jar, file]);
    if !success || !confirmed.starts_with("same") {
        found.push(format!("{file}: `earnest confirm --tlc` says\n{confirmed}"));
    }
    found
}

#[test]
fn every_seeded_quint_model_has_the_states_and_verdicts_of_quint_s_tlc_run() {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    let jar = jar.to_string_lossy().into_owned();
    let expected: Value =
        serde_json::from_str(EXPECTED).expect("the answers of Quint's TLC run are JSON");
    let mut found = Vec::new();
    for (file, json) in COMPILED {
        let model = file.trim_end_matches(".json");
        let answer = expected
            .as_array()
            .and_then(|all| all.iter().find(|each| each["model"] == model))
            .unwrap_or_else(|| panic!("expected.json has no answer for {model}"));
        found.extend(differences(file, json, answer, &jar));
    }
    assert!(
        found.is_empty(),
        "`quint verify --backend tlc` gave each state count and verdict in expected.json, and \
         earnest must find the same, and TLC on earnest's own export too:\n{}",
        found.join("\n")
    );
}
