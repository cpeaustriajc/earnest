use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::Value;
use tempfile::TempDir;

use super::{TLC_JAR_VARIABLE, earnest, repository, tlc_jar};

static ONE_TLC_AT_A_TIME: Mutex<()> = Mutex::new(());

const SEEDS: std::ops::RangeInclusive<u64> = 1..=40;

fn one_at_a_time() -> MutexGuard<'static, ()> {
    ONE_TLC_AT_A_TIME
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn jar() -> String {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    jar.to_string_lossy().into_owned()
}

struct Said {
    stdout: String,
    stderr: String,
    success: bool,
}

fn run(folder: &Path, arguments: &[&str]) -> Said {
    let output = Command::new(earnest())
        .args(arguments)
        .current_dir(folder)
        .output()
        .expect("earnest starts");
    Said {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    }
}

struct Confirmed {
    runs: usize,
    verdicts: Vec<String>,
}

fn confirmed(name: &str, source: &str, jar: &str) -> Confirmed {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(folder.path().join(name), source).expect("the model can be written");
    let checked = run(folder.path(), &["verify", "--json", "--no-cache", name]);
    let report: Value = serde_json::from_str(&checked.stdout).unwrap_or_else(|error| {
        panic!(
            "earnest did not print JSON for {name} ({error}):\n{}{}\n{source}",
            checked.stdout, checked.stderr
        )
    });
    assert_eq!(
        report["unreadable"],
        Value::Array(Vec::new()),
        "{name} is readable\n{source}"
    );
    let models = report["models"].as_array().cloned().unwrap_or_default();
    let verdicts = models
        .iter()
        .flat_map(|model| model["rules"].as_array().cloned().unwrap_or_default())
        .filter(|rule| rule["kind"] == "eventually")
        .map(|rule| rule["verdict"].as_str().unwrap_or_default().to_owned())
        .collect();
    let _tlc = one_at_a_time();
    let said = run(folder.path(), &["confirm", "--tlc", jar, name]);
    let results: Vec<&str> = said
        .stdout
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .collect();
    assert!(
        said.success
            && results.len() == models.len()
            && results.iter().all(|line| line.starts_with("same ")),
        "TLC must find the state count and every verdict earnest finds, for the model and for \
         each setup ({} runs) of {name}:\n{}{}\n{source}",
        models.len(),
        said.stdout,
        said.stderr
    );
    Confirmed {
        runs: models.len(),
        verdicts,
    }
}

fn conformance_models() -> Vec<PathBuf> {
    let folder = repository().join("tasks/conformance/runs");
    let mut files: Vec<PathBuf> = fs::read_dir(&folder)
        .unwrap_or_else(|error| panic!("{} can't be listed: {error}", folder.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().ends_with(".model.ts"))
        .collect();
    files.sort();
    files
}

#[test]
fn tlc_confirms_every_model_and_setup_of_the_runs_conformance_models() {
    let jar = jar();
    let mut runs = 0_usize;
    let mut verdicts = Vec::new();
    for file in conformance_models() {
        let name = file
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned());
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", file.display()));
        let found = confirmed(&name, &source, &jar);
        runs += found.runs;
        verdicts.extend(found.verdicts);
    }
    assert_eq!(
        runs, 12,
        "the README counts 10 models and 2 setups in tasks/conformance/runs"
    );
    let broken = verdicts
        .iter()
        .filter(|verdict| *verdict == "broken")
        .count();
    let holds = verdicts
        .iter()
        .filter(|verdict| *verdict == "holds")
        .count();
    assert_eq!(
        (holds, broken),
        (9, 6),
        "the README lists 15 judged rules about runs: 9 hold and 6 are broken"
    );
}

struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }

    fn percent(&mut self, chance: u64) -> bool {
        self.below(100) < chance
    }

    fn pick<'a, T>(&mut self, all: &'a [T]) -> &'a T {
        let count = u64::try_from(all.len()).unwrap_or(u64::MAX);
        let position = usize::try_from(self.below(count)).unwrap_or_default();
        all.get(position)
            .expect("a pick from a list that is not empty")
    }
}

const COMPARISONS: [&str; 6] = ["===", "!==", "<", "<=", ">", ">="];

fn atom(draw: &mut Draw, fields: &[(String, u64)]) -> String {
    let (name, top) = draw.pick(fields).clone();
    let compared = draw.pick(&COMPARISONS);
    format!("s.{name} {compared} {}", draw.below(top + 1))
}

fn condition(draw: &mut Draw, fields: &[(String, u64)]) -> String {
    match draw.below(100) {
        0..50 => atom(draw, fields),
        50..75 => format!("({} && {})", atom(draw, fields), atom(draw, fields)),
        75..95 => format!("({} || {})", atom(draw, fields), atom(draw, fields)),
        _ => (*draw.pick(&["true", "false"])).to_owned(),
    }
}

fn change(draw: &mut Draw, name: &str, top: u64, fields: &[(String, u64)]) -> String {
    match draw.below(100) {
        0..40 => format!("{name}: {}", draw.below(top + 1)),
        40..80 => format!("{name}: m.rem(s.{name} + 1, {})", top + 1),
        _ => {
            let (other, _) = draw.pick(fields);
            format!("{name}: s.{other} > {top} ? {top} : s.{other}")
        }
    }
}

fn generated(seed: u64) -> String {
    let mut draw = Draw(seed);
    let name = format!("M{seed}");
    let fields: Vec<(String, u64)> = (0..=draw.below(3))
        .map(|position| (format!("f{position}"), 1 + draw.below(2)))
        .collect();
    let mut text = String::from("import { m } from \"@earnestjs/model\";\n\n");
    writeln!(text, "export const {name} = m.model(\"{name}\", {{").ok();
    for (field, top) in &fields {
        let start = if draw.percent(25) {
            String::new()
        } else {
            format!(".initial({})", draw.below(top + 1))
        };
        writeln!(text, "\t{field}: m.int(0, {top}){start},").ok();
    }
    text.push_str("});\n");
    for action in 0..=draw.below(5) {
        let mut changes = Vec::new();
        for (field, top) in &fields {
            if draw.percent(50) {
                changes.push(change(&mut draw, field, *top, &fields));
            }
        }
        writeln!(
            text,
            "export const a{action} = m.action({name}, \"action {action}\", {{"
        )
        .ok();
        if draw.percent(40) {
            text.push_str("\tfair: true,\n");
        }
        if draw.percent(80) {
            writeln!(text, "\twhen: (s) => {},", condition(&mut draw, &fields)).ok();
        }
        writeln!(
            text,
            "\tchanges: (s) => ({{ {} }}),\n}});",
            changes.join(", ")
        )
        .ok();
    }
    for rule in 0..=draw.below(3) {
        writeln!(
            text,
            "export const e{rule} = m.eventually({name}, \"eventually {rule}\", {{"
        )
        .ok();
        if draw.percent(60) {
            writeln!(text, "\tafter: (s) => {},", condition(&mut draw, &fields)).ok();
        }
        writeln!(
            text,
            "\treaches: (s) => {},\n}});",
            condition(&mut draw, &fields)
        )
        .ok();
    }
    if draw.percent(50) {
        writeln!(
            text,
            "export const over = m.end({name}, \"over\", (s) => {});",
            condition(&mut draw, &fields)
        )
        .ok();
    }
    text
}

#[test]
fn the_generator_writes_the_same_model_for_the_same_seed() {
    assert_eq!(generated(7), generated(7), "a seed decides the model");
    assert_ne!(generated(7), generated(8), "another seed, another model");
}

#[test]
fn tlc_judges_every_rule_about_runs_of_small_generated_models_as_earnest_does() {
    let jar = jar();
    let mut verdicts = Vec::new();
    let mut fair = 0_usize;
    let mut after = 0_usize;
    for seed in SEEDS {
        let source = generated(seed);
        fair += usize::from(source.contains("fair: true"));
        after += usize::from(source.contains("after:"));
        let found = confirmed(&format!("m{seed}.model.ts"), &source, &jar);
        verdicts.extend(found.verdicts);
    }
    let count = |wanted: &str| verdicts.iter().filter(|verdict| *verdict == wanted).count();
    assert!(
        count("broken") >= 10 && count("holds") >= 10,
        "the seeds give rules that hold and rules that are broken: {} hold, {} broken, {} \
         unknown",
        count("holds"),
        count("broken"),
        count("unknown")
    );
    assert!(
        fair >= 10 && after >= 10,
        "the seeds give models with fair actions ({fair}) and rules with `after` ({after})"
    );
}
