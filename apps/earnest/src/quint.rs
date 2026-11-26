use crate::cache::{ENGINE, fingerprint_of, folder_in, write_whole};
use crate::checks::{CHECKS_FILE, Check, EXPECT_COUNTEREXAMPLE, read_entries};
use crate::discovery::Listed;
use crate::marks::{Kind, how_to_mark, is_a_mark, mark_of, misplaced_mark, unknown_mark};
use crate::places::{Documented, Places, place_all};
use crate::reading::{Content, Declared, ModelFile, Readable};
use crate::run::whole_of;
use crate::typecheck::typechecked;
use crate::workers::run_each;
use earnest_diagnostics::{Place, Problem};
use earnest_ir::{Code, Expr, Op};
use earnest_quint::{Located, Runs, Setup, Typechecked};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

const PROGRAM_VARIABLE: &str = "EARNEST_QUINT";
const PROGRAM: &str = "quint";
const LOCAL_PROGRAMS: &str = "node_modules/.bin";
const QUINT_EXTENSION: &str = "qnt";
const CACHE: &str = "quint";
const HERE: &str = ".";
const SKIPPED_FOLDERS: [&str; 4] = ["node_modules", "target", "dist", "build"];
const TOP_OF_FILE: Place = Place { line: 1, column: 1 };
const TESTS_END_WITH: &str = "Test";
const QUALIFIED: &str = "::";
const PROPERTY_QUALIFIERS: [&str; 4] = ["val", "def", "pureval", "puredef"];
const TEMPORAL: &str = "temporal";
const NOT_FOUND: &str = "`quint` was not found: install it with `npm install --save-dev \
                         @informalsystems/quint`, or set `EARNEST_QUINT`";

static SCRATCH: AtomicU64 = AtomicU64::new(0);

#[cfg(windows)]
const PROGRAM_FILES: [&str; 3] = ["quint.cmd", "quint.exe", "quint"];
#[cfg(not(windows))]
const PROGRAM_FILES: [&str; 1] = [PROGRAM];

#[derive(Debug)]
struct Asked {
    shown: PathBuf,
    only: Option<BTreeSet<PathBuf>>,
    named: Vec<(PathBuf, bool)>,
}

#[derive(Debug)]
struct Candidate {
    checks: String,
    shown: PathBuf,
    real: PathBuf,
    named: bool,
    read: u64,
    quint: Result<Quint, String>,
}

#[derive(Debug)]
struct Job {
    checks: String,
    shown: String,
    real: PathBuf,
    check: Check,
    name: Option<String>,
    expected: Vec<String>,
    reachable: Vec<String>,
    said: Vec<(String, String)>,
    read: u64,
    quint: Result<Quint, String>,
}

impl Job {
    fn model_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.check.main)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct QuintProperty {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug)]
pub(crate) struct QuintCheck {
    pub(crate) program: PathBuf,
    pub(crate) file: PathBuf,
    pub(crate) main: String,
    pub(crate) init: String,
    pub(crate) step: String,
    pub(crate) invariants: Vec<QuintProperty>,
    pub(crate) temporal: Vec<QuintProperty>,
    pub(crate) runs: Runs,
    pub(crate) searches: bool,
}

fn as_asked(job: &Job, quint: &Quint) -> QuintCheck {
    let check = &job.check;
    let named = |name: &String| QuintProperty {
        id: name.clone(),
        text: if job.reachable.contains(name) {
            format!("not({name})")
        } else {
            name.clone()
        },
    };
    QuintCheck {
        program: quint.program.clone(),
        file: job.real.clone(),
        main: check.main.clone(),
        init: check.init.clone(),
        step: check.step.clone(),
        invariants: check.invariants.iter().map(named).collect(),
        temporal: check.temporal.iter().map(named).collect(),
        runs: check.runs.clone(),
        searches: !runs_only(check),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Quint {
    pub(crate) program: PathBuf,
    pub(crate) version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Version {
    path: String,
    size: u64,
    modified: u128,
    said: String,
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub(crate) fn shown_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn problem_file(path: &str, feature: &str, message: &str) -> ModelFile {
    unreadable(
        path,
        Vec::new(),
        vec![Problem::new(path, TOP_OF_FILE, feature, message)],
    )
}

fn unreadable(path: &str, declared: Vec<Declared>, problems: Vec<Problem>) -> ModelFile {
    ModelFile {
        path: path.to_owned(),
        source: String::new(),
        declared,
        content: Content::Unreadable(problems),
    }
}

fn not_a_model(qnt: &Path) -> ModelFile {
    let name = qnt.file_name().map_or_else(
        || shown_string(qnt),
        |name| name.to_string_lossy().into_owned(),
    );
    problem_file(
        &shown_string(qnt),
        &format!("`{name}`"),
        &format!(
            "no module of the Quint file is checked, and `{CHECKS_FILE}` lists none: {}",
            how_to_mark()
        ),
    )
}

fn asked(listed: &[Listed]) -> BTreeMap<PathBuf, Asked> {
    let mut asked: BTreeMap<PathBuf, Asked> = BTreeMap::new();
    for each in listed {
        let slot = asked.entry(real(&each.checks)).or_insert_with(|| Asked {
            shown: each.checks.clone(),
            only: Some(BTreeSet::new()),
            named: Vec::new(),
        });
        match &each.only {
            None => slot.only = None,
            Some(qnt) => {
                if let Some(only) = &mut slot.only {
                    only.insert(real(qnt));
                }
                slot.named.push((qnt.clone(), each.named));
            }
        }
    }
    asked
}

fn is_skipped(name: &OsStr) -> bool {
    name.to_str()
        .is_none_or(|name| name.starts_with('.') || SKIPPED_FOLDERS.contains(&name))
}

fn quint_files(folder: &Path, below: &Path, found: &mut Vec<(PathBuf, PathBuf)>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let path = entry.path();
        if path.is_dir() {
            if !is_skipped(&name) {
                quint_files(&path, &below.join(&name), found);
            }
        } else if path.extension().and_then(OsStr::to_str) == Some(QUINT_EXTENSION) {
            found.push((below.join(&name), path));
        }
    }
}

fn tree_fingerprint(folder: &Path) -> u64 {
    let mut found = Vec::new();
    quint_files(folder, Path::new(""), &mut found);
    found.sort();
    let mut pieces: Vec<Vec<u8>> = Vec::with_capacity(found.len().saturating_mul(2));
    for (below, path) in found {
        pieces.push(below.to_string_lossy().into_owned().into_bytes());
        pieces.push(fs::read(&path).unwrap_or_default());
    }
    let borrowed: Vec<&[u8]> = pieces.iter().map(Vec::as_slice).collect();
    fingerprint_of(&borrowed)
}

fn local_program(folder: &Path) -> Option<PathBuf> {
    real(folder).ancestors().find_map(|above| {
        PROGRAM_FILES
            .iter()
            .map(|name| above.join(LOCAL_PROGRAMS).join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn program_on_path() -> Option<PathBuf> {
    let listed = env::var_os("PATH")?;
    env::split_paths(&listed).find_map(|folder| {
        PROGRAM_FILES
            .iter()
            .map(|name| folder.join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn find_program(folder: &Path) -> Result<PathBuf, String> {
    if let Some(named) = env::var_os(PROGRAM_VARIABLE).filter(|named| !named.is_empty()) {
        let program = PathBuf::from(&named);
        return if program.is_file() {
            Ok(program)
        } else {
            Err(format!(
                "`{PROGRAM}` was not found: `{PROGRAM_VARIABLE}` names `{}`, which is not a \
                 file",
                program.to_string_lossy()
            ))
        };
    }
    local_program(folder)
        .or_else(program_on_path)
        .ok_or_else(|| NOT_FOUND.to_owned())
}

pub(crate) fn cache_path(name: &str) -> PathBuf {
    folder_in(Path::new(HERE), CACHE).join(name)
}

pub(crate) fn store(target: &Path, text: &str) {
    if let Err(error) = write_whole(target, text) {
        crate::print::to_stderr(&format!(
            "warning: {} was not cached: {error}",
            shown_string(target)
        ));
    }
}

fn first_lines(output: &Output) -> String {
    let said = String::from_utf8_lossy(&output.stderr);
    let first = said
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned);
    first.unwrap_or_else(|| format!("`{PROGRAM}` stopped with {}", output.status))
}

pub(crate) fn scratch(what: &str) -> io::Result<PathBuf> {
    let path = cache_path(&format!(
        ".{what}-{}-{}.json",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    ));
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    Ok(path)
}

fn version_of(program: &Path) -> Result<String, String> {
    let resolved = real(program);
    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("`{}` can't be read: {error}", shown_string(program)))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    let path = shown_string(&resolved);
    let target = cache_path(&format!(
        "version-{:016x}.json",
        fingerprint_of(&[path.as_bytes()])
    ));
    if let Ok(text) = fs::read_to_string(&target)
        && let Ok(kept) = serde_json::from_str::<Version>(&text)
        && kept.path == path
        && kept.size == metadata.len()
        && kept.modified == modified
    {
        return Ok(kept.said);
    }
    let output = Command::new(program)
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "`{} --version` did not start: {error}",
                shown_string(program)
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "`{} --version` failed: {}",
            shown_string(program),
            first_lines(&output)
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let kept = Version {
        path,
        size: metadata.len(),
        modified,
        said: version.clone(),
    };
    if let Ok(text) = serde_json::to_string(&kept) {
        store(&target, &text);
    }
    Ok(version)
}

fn arguments(check: &Check, file: &str) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = [
        "compile",
        "--target",
        "json",
        "--main",
        &check.main,
        "--init",
        &check.init,
        "--step",
        &check.step,
    ]
    .iter()
    .map(OsString::from)
    .collect();
    for (flag, names) in [
        ("--invariant", &check.invariants),
        ("--temporal", &check.temporal),
    ] {
        if !names.is_empty() {
            arguments.push(flag.into());
            arguments.push(names.join(",").into());
        }
    }
    arguments.push(file.into());
    arguments
}

fn compiled_as(check: &Check) -> [String; 5] {
    [
        check.main.clone(),
        check.init.clone(),
        check.step.clone(),
        check.invariants.join(","),
        check.temporal.join(","),
    ]
}

fn key_of(job: &Job, quint: &Quint, source: &[u8]) -> String {
    let compiled = compiled_as(&job.check);
    let mut listed: Vec<&str> = vec![quint.version.as_str()];
    listed.extend(compiled.iter().map(String::as_str));
    let mut pieces: Vec<&[u8]> = listed.iter().map(|each| each.as_bytes()).collect();
    let read = job.read.to_le_bytes();
    pieces.push(&read);
    pieces.push(source);
    format!("{:016x}", fingerprint_of(&pieces))
}

pub(crate) fn kept(target: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(target).ok()?;
    let (first, rest) = text.split_once('\n')?;
    (first == key).then(|| rest.to_owned())
}

fn quint_place(said: &str, job: &Job) -> (String, Place) {
    for line in said.lines().map(str::trim) {
        let Some(rest) = line.strip_prefix("at ") else {
            continue;
        };
        let mut parts = rest.rsplitn(3, ':');
        let (Some(column), Some(number), Some(file)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(column), Ok(number)) = (column.parse::<u32>(), number.parse::<u32>()) else {
            continue;
        };
        return (
            shown_in(Path::new(file), job),
            Place {
                line: number.max(1),
                column: column.max(1),
            },
        );
    }
    (job.shown.clone(), TOP_OF_FILE)
}

pub(crate) fn shown_in_folder(file: &Path, check: &Path, shown: &str) -> String {
    if real(file) == real(check) {
        return shown.to_owned();
    }
    let here = env::current_dir().ok().map(|folder| real(&folder));
    let resolved = real(file);
    here.and_then(|folder| resolved.strip_prefix(folder).ok().map(shown_string))
        .unwrap_or_else(|| shown_string(file))
}

fn shown_in(file: &Path, job: &Job) -> String {
    shown_in_folder(file, &job.real, &job.shown)
}

fn compile(job: &Job) -> Result<String, Problem> {
    let failed = |message: String| Problem::new(&job.shown, TOP_OF_FILE, "`quint`", &message);
    let quint = job.quint.clone().map_err(failed)?;
    let source = fs::read(&job.real).map_err(|error| {
        Problem::new(
            &job.shown,
            TOP_OF_FILE,
            &format!("`{}`", job.check.file),
            &format!("the Quint file that `{CHECKS_FILE}` lists can't be opened: {error}"),
        )
    })?;
    let key = key_of(job, &quint, &source);
    let compiled = compiled_as(&job.check);
    let file = shown_string(&job.real);
    let mut named: Vec<&[u8]> = vec![file.as_bytes()];
    named.extend(compiled.iter().map(String::as_bytes));
    let target = cache_path(&format!("{:016x}.json", fingerprint_of(&named)));
    if let Some(json) = kept(&target, &key) {
        return Ok(json);
    }
    let written = scratch("compiled").map_err(|error| {
        failed(format!(
            "the folder `.earnest/quint` can't be written, so `quint compile` has no place to \
             write to: {error}"
        ))
    })?;
    let ran = File::create(&written).and_then(|file| {
        Command::new(&quint.program)
            .args(arguments(&job.check, &job.shown))
            .stdout(file)
            .stderr(Stdio::piped())
            .output()
    });
    let json = fs::read_to_string(&written);
    fs::remove_file(&written).unwrap_or_default();
    let output = ran.map_err(|error| {
        failed(format!(
            "`{}` did not start: {error}",
            shown_string(&quint.program)
        ))
    })?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        let (file, place) = quint_place(&said, job);
        return Err(Problem::new(
            &file,
            place,
            &format!("`quint compile --main {}`", job.check.main),
            &first_lines(&output),
        ));
    }
    let json = json.map_err(|error| {
        failed(format!(
            "what `quint compile` wrote can't be read back: {error}"
        ))
    })?;
    store(&target, &format!("{key}\n{json}"));
    Ok(json)
}

fn runs_only(check: &Check) -> bool {
    check.invariants.is_empty() && check.temporal.is_empty() && check.runs != Runs::None
}

fn lacks_a_search(check: &Check, places: Option<&Places>) -> bool {
    runs_only(check)
        && places.is_some_and(|found| {
            !found.declares(&check.main, &check.init) || !found.declares(&check.main, &check.step)
        })
}

fn unbuilt(job: &Job, problem: Problem) -> Problem {
    let check = &job.check;
    Problem {
        message: format!(
            "the check lists only runs, and the module `{}` sees no `{}` and `{}` that `quint \
             compile` takes, nor does it import a module that has an `init` and a `step`: name \
             in the check an `init` and a `step` the module sees, such as `A::init` of an \
             `import Spec as A`. Quint said: {}",
            check.main, check.init, check.step, problem.message
        ),
        ..problem
    }
}

fn compiled_for_runs(job: &Job, places: Option<&Places>) -> Result<String, Problem> {
    let check = &job.check;
    let entry = (check.init.clone(), check.step.clone());
    let mut tried: Vec<(String, String)> = Vec::new();
    if places
        .is_none_or(|found| found.sees(&check.main, &entry.0) && found.sees(&check.main, &entry.1))
    {
        tried.push(entry.clone());
    }
    for each in places.map_or_else(Vec::new, |found| found.imported_init_and_step(&check.main)) {
        if !tried.contains(&each) {
            tried.push(each);
        }
    }
    if tried.is_empty() {
        tried.push(entry);
    }
    let mut first = None;
    for (init, step) in tried {
        let attempt = Job {
            checks: job.checks.clone(),
            shown: job.shown.clone(),
            real: job.real.clone(),
            check: Check {
                init,
                step,
                ..check.clone()
            },
            name: job.name.clone(),
            expected: job.expected.clone(),
            reachable: job.reachable.clone(),
            said: job.said.clone(),
            read: job.read,
            quint: job.quint.clone(),
        };
        match compile(&attempt) {
            Ok(json) => return Ok(json),
            Err(problem) => {
                first.get_or_insert(problem);
            }
        }
    }
    Err(unbuilt(
        job,
        first.unwrap_or_else(|| {
            Problem::new(&job.shown, TOP_OF_FILE, "`quint`", "nothing was compiled")
        }),
    ))
}

fn filled(job: &Job, typechecked: Option<&Typechecked>) -> Option<String> {
    let check = &job.check;
    typechecked?.setup(&Setup::new(
        &check.main,
        (&check.init, &check.step),
        (&check.invariants, &check.temporal),
        &check.runs,
    ))
}

fn fingerprint_files(paths: &[&str]) -> u64 {
    let mut sorted = paths.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let contents: Vec<Vec<u8>> = sorted
        .iter()
        .map(|path| fs::read(path).unwrap_or_default())
        .collect();
    let mut pieces: Vec<&[u8]> = Vec::with_capacity(sorted.len().saturating_mul(2));
    for (path, content) in sorted.iter().zip(&contents) {
        pieces.push(path.as_bytes());
        pieces.push(content);
    }
    fingerprint_of(&pieces)
}

fn places_key(engine: &str, quint: &Quint, source: &[u8]) -> String {
    format!(
        "{:016x}",
        fingerprint_of(&[engine.as_bytes(), quint.version.as_bytes(), source])
    )
}

fn parse_failed(shown: &str, real_file: &Path, said: &str) -> Problem {
    let feature = "`quint parse`";
    let first = serde_json::from_str::<serde_json::Value>(said)
        .ok()
        .and_then(|value| value.get("errors")?.as_array()?.first().cloned());
    let Some(first) = first else {
        return Problem::new(
            shown,
            TOP_OF_FILE,
            feature,
            "the Quint file can't be parsed",
        );
    };
    let explanation = first
        .get("explanation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("the Quint file can't be parsed");
    let at = first
        .get("locs")
        .and_then(|locs| locs.get(0))
        .and_then(|loc| {
            let start = loc.get("start")?;
            let line = u32::try_from(start.get("line")?.as_u64()?).ok()?;
            let column = u32::try_from(start.get("col")?.as_u64()?).ok()?;
            let file = loc
                .get("source")
                .and_then(serde_json::Value::as_str)
                .map_or_else(
                    || shown.to_owned(),
                    |source| shown_in_folder(Path::new(source), real_file, shown),
                );
            Some((
                file,
                Place {
                    line: line.saturating_add(1),
                    column: column.saturating_add(1),
                },
            ))
        });
    let (file, place) = at.unwrap_or_else(|| (shown.to_owned(), TOP_OF_FILE));
    Problem::new(&file, place, feature, explanation)
}

fn parsed(real_file: &Path, shown: &str, quint: &Quint) -> Result<Places, Problem> {
    let unread = |error: io::Error| {
        Problem::new(
            shown,
            TOP_OF_FILE,
            "`quint parse`",
            &format!("the Quint file can't be read: {error}"),
        )
    };
    let source = fs::read(real_file).map_err(unread)?;
    let key = places_key(ENGINE, quint, &source);
    let target = cache_path(&format!(
        "places-{:016x}.json",
        fingerprint_of(&[shown_string(real_file).as_bytes()])
    ));
    if let Some(text) = kept(&target, &key)
        && let Ok(places) = serde_json::from_str::<Places>(&text)
        && places.read == fingerprint_files(&places.files_read())
    {
        return Ok(places);
    }
    let (out, map) = (
        scratch("parse").map_err(unread)?,
        scratch("map").map_err(unread)?,
    );
    let ran = Command::new(&quint.program)
        .arg("parse")
        .arg(shown)
        .arg("--out")
        .arg(&out)
        .arg("--source-map")
        .arg(&map)
        .output();
    let written = fs::read_to_string(&out).unwrap_or_default();
    let read = || -> Result<Places, Problem> {
        let output = ran.as_ref().map_err(|error| {
            Problem::new(
                shown,
                TOP_OF_FILE,
                "`quint`",
                &format!("`{}` did not start: {error}", shown_string(&quint.program)),
            )
        })?;
        if !output.status.success() {
            return Err(parse_failed(shown, real_file, &written));
        }
        fs::read_to_string(&map)
            .ok()
            .and_then(|map| Places::from_quint(&written, &map))
            .ok_or_else(|| parse_failed(shown, real_file, &written))
    };
    let places = read();
    fs::remove_file(&out).unwrap_or_default();
    fs::remove_file(&map).unwrap_or_default();
    let mut places = places?;
    places.read = fingerprint_files(&places.files_read());
    if let Ok(text) = serde_json::to_string(&places) {
        store(&target, &format!("{key}\n{text}"));
    }
    Ok(places)
}

fn refused(job: &Job, name: &str, message: &str) -> Problem {
    Problem::new(
        &job.shown,
        TOP_OF_FILE,
        &format!("`{name}`"),
        &format!(
            "`{EXPECT_COUNTEREXAMPLE}` of the check `{}` names `{name}`, {message}",
            job.model_name()
        ),
    )
}

fn widened(job: &mut Job, places: Option<&Places>) -> Result<(), Problem> {
    for name in job.expected.clone() {
        let check = &job.check;
        let listed_run = matches!(&check.runs, Runs::Named(names) if names.contains(&name));
        if check.invariants.contains(&name) || check.temporal.contains(&name) || listed_run {
            continue;
        }
        let qualifier = places.map_or(PROPERTY_QUALIFIERS[0], |found| {
            found.qualifier(&name, &check.main).unwrap_or_else(|| {
                if found.reaches(&name, &check.main) {
                    ""
                } else {
                    PROPERTY_QUALIFIERS[0]
                }
            })
        });
        let check = &mut job.check;
        match qualifier {
            kind if PROPERTY_QUALIFIERS.contains(&kind) => check.invariants.push(name),
            "temporal" => check.temporal.push(name),
            "run" => match &mut check.runs {
                Runs::Named(names) => names.push(name),
                Runs::Tests if name.ends_with(TESTS_END_WITH) => {}
                Runs::Tests => {
                    return Err(refused(
                        job,
                        &name,
                        &format!(
                            "a run that `\"runs\": true` does not play, since its name does not \
                             end in `{TESTS_END_WITH}`: list the runs to play by name in `runs`"
                        ),
                    ));
                }
                _ => check.runs = Runs::Named(vec![name]),
            },
            _ => {
                return Err(refused(
                    job,
                    &name,
                    "which is not an invariant, a temporal property or a run",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn segments_end_alike(first: &str, second: &str) -> bool {
    let first: Vec<&str> = first.split(QUALIFIED).collect();
    let second: Vec<&str> = second.split(QUALIFIED).collect();
    first.ends_with(&second) || second.ends_with(&first)
}

fn whole_ids(model: &earnest_ir::Model) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for id in model.rule_ids().map(whole_of) {
        if !ids.iter().any(|kept| kept == id) {
            ids.push(id.to_owned());
        }
    }
    ids
}

fn id_of<'a>(ids: &'a [String], name: &str) -> Option<&'a String> {
    let exact = ids.iter().find(|id| *id == name);
    let mut alike = ids.iter().filter(|id| segments_end_alike(id, name));
    exact.or_else(|| {
        let first = alike.next()?;
        alike.next().is_none().then_some(first)
    })
}

fn expected_ids(job: &Job, model: &earnest_ir::Model) -> Result<Vec<String>, Problem> {
    let ids = whole_ids(model);
    let mut expected: Vec<String> = Vec::with_capacity(job.expected.len());
    for name in &job.expected {
        let Some(found) = id_of(&ids, name) else {
            return Err(refused(
                job,
                name,
                &format!(
                    "and the module `{}` has no property of that name",
                    job.check.main
                ),
            ));
        };
        if !expected.contains(found) {
            expected.push(found.clone());
        }
    }
    Ok(expected)
}

fn declared(job: &Job) -> Declared {
    let check = &job.check;
    Declared {
        name: job.model_name().to_owned(),
        rules: check
            .invariants
            .iter()
            .chain(&check.temporal)
            .cloned()
            .collect(),
    }
}

fn program_for(
    programs: &mut HashMap<PathBuf, Result<Quint, String>>,
    folder: &Path,
) -> Result<Quint, String> {
    programs
        .entry(folder.to_path_buf())
        .or_insert_with(|| {
            let program = find_program(folder)?;
            let version = version_of(&program)?;
            Ok(Quint { program, version })
        })
        .clone()
}

fn candidates_of(
    unmatched: Vec<(String, PathBuf, bool)>,
    matched: &BTreeSet<PathBuf>,
    programs: &mut HashMap<PathBuf, Result<Quint, String>>,
) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for (checks, shown, named) in unmatched {
        let real_file = real(&shown);
        if matched.contains(&real_file) {
            continue;
        }
        if let Some(kept) = candidates.iter_mut().find(|each| each.real == real_file) {
            kept.named |= named;
            continue;
        }
        let real_folder = real_file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        candidates.push(Candidate {
            checks,
            read: tree_fingerprint(&real_folder),
            quint: program_for(programs, &real_folder),
            shown,
            real: real_file,
            named,
        });
    }
    candidates.sort_by(|first, second| first.shown.cmp(&second.shown));
    candidates
}

fn jobs_of(
    asked: BTreeMap<PathBuf, Asked>,
    files: &mut Vec<ModelFile>,
) -> (Vec<Job>, Vec<Candidate>) {
    let mut programs: HashMap<PathBuf, Result<Quint, String>> = HashMap::new();
    let mut jobs = Vec::new();
    let mut matched: BTreeSet<PathBuf> = BTreeSet::new();
    let mut unmatched: Vec<(String, PathBuf, bool)> = Vec::new();
    for (real_checks, asked) in asked {
        let shown_checks = shown_string(&asked.shown);
        let text = match fs::read_to_string(&asked.shown) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound && asked.only.is_some() => {
                unmatched.extend(
                    asked
                        .named
                        .into_iter()
                        .map(|(qnt, named)| (shown_checks.clone(), qnt, named)),
                );
                continue;
            }
            Err(error) => {
                files.push(problem_file(
                    &shown_checks,
                    &format!("`{CHECKS_FILE}`"),
                    &format!("the list of Quint checks can't be opened: {error}"),
                ));
                continue;
            }
        };
        let checks = match read_entries(&text) {
            Ok(checks) => checks,
            Err(wrong) => {
                files.push(unreadable(
                    &shown_checks,
                    Vec::new(),
                    vec![Problem::new(
                        &shown_checks,
                        wrong.place,
                        &wrong.feature,
                        &wrong.message,
                    )],
                ));
                continue;
            }
        };
        let folder = asked
            .shown
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        let real_folder = real_checks
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf();
        let tree = tree_fingerprint(&real_folder);
        for entry in checks {
            let check = entry.check;
            let shown = folder.join(&check.file);
            let real_file = real(&shown);
            matched.insert(real_file.clone());
            if let Some(only) = &asked.only
                && !only.contains(&real_file)
            {
                continue;
            }
            jobs.push(Job {
                checks: shown_checks.clone(),
                shown: shown_string(&shown),
                real: real_file,
                check,
                name: entry.name,
                expected: entry.expected,
                reachable: Vec::new(),
                said: Vec::new(),
                read: tree,
                quint: program_for(&mut programs, &real_folder),
            });
        }
        unmatched.extend(
            asked
                .named
                .into_iter()
                .filter(|(_, named)| *named)
                .map(|(qnt, named)| (shown_checks.clone(), qnt, named)),
        );
    }
    let candidates = candidates_of(unmatched, &matched, &mut programs);
    (jobs, candidates)
}

fn marked_check(candidate: &Candidate, main: String, places: &Places) -> Job {
    let runs = if places.sees_a_test(&main) {
        Runs::Tests
    } else {
        Runs::None
    };
    Job {
        checks: candidate.checks.clone(),
        shown: shown_string(&candidate.shown),
        real: candidate.real.clone(),
        check: Check {
            file: candidate
                .shown
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            main,
            init: "init".to_owned(),
            step: "step".to_owned(),
            invariants: Vec::new(),
            temporal: Vec::new(),
            runs,
        },
        name: None,
        expected: Vec::new(),
        reachable: Vec::new(),
        said: Vec::new(),
        read: candidate.read,
        quint: candidate.quint.clone(),
    }
}

fn marked_checks(
    candidate: &Candidate,
    places: Result<&Places, &Problem>,
    files: &mut Vec<ModelFile>,
) -> Vec<Job> {
    let places = match places {
        Ok(places) => places,
        Err(problem) => {
            files.push(unreadable(
                &shown_string(&candidate.shown),
                Vec::new(),
                vec![problem.clone()],
            ));
            return Vec::new();
        }
    };
    let jobs: Vec<Job> = places
        .checks_in(&candidate.real)
        .into_iter()
        .filter(|main| {
            places.sees_a_test(main)
                || places
                    .documented(main)
                    .iter()
                    .any(|each| is_a_mark(each.doc))
        })
        .map(|main| marked_check(candidate, main, places))
        .collect();
    if jobs.is_empty() && candidate.named {
        files.push(not_a_model(&candidate.shown));
    }
    jobs
}

fn mark_problem(job: &Job, places: &Places, each: &Documented<'_>, message: &str) -> Problem {
    let feature = format!("`{}`", each.name);
    let located = Located::new(
        Problem::new(&job.shown, TOP_OF_FILE, &feature, message),
        Some(each.id),
        None,
    );
    let found = place_all(
        vec![located],
        places,
        &job.check.main,
        &job.real,
        &job.shown,
    );
    let Some(mut problem) = found.into_iter().next() else {
        return Problem::new(&job.shown, TOP_OF_FILE, &feature, message);
    };
    if problem.place() != TOP_OF_FILE {
        let doc_lines = u32::try_from(each.doc.lines().count()).unwrap_or(0);
        problem.line = problem.line.saturating_sub(doc_lines).max(1);
    }
    problem
}

fn push_new(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|each| each == name) {
        list.push(name.to_owned());
    }
}

fn with_marks(job: &mut Job, places: Option<&Places>) -> Result<(), Problem> {
    let Some(places) = places else {
        return Ok(());
    };
    for each in places.documented(&job.check.main) {
        let Some(mark) = mark_of(each.doc) else {
            continue;
        };
        let mark =
            mark.map_err(|written| mark_problem(job, places, &each, &unknown_mark(&written)))?;
        let qualifier = each.qualifier.unwrap_or_default();
        let is_a_value = PROPERTY_QUALIFIERS.contains(&qualifier);
        let fits = match mark.kind {
            Kind::Reachable => is_a_value,
            Kind::Holds | Kind::Broken => is_a_value || qualifier == TEMPORAL,
        };
        if !fits {
            return Err(mark_problem(
                job,
                places,
                &each,
                &misplaced_mark(mark.kind, &each.name, qualifier),
            ));
        }
        let check = &mut job.check;
        if qualifier == TEMPORAL {
            push_new(&mut check.temporal, &each.name);
        } else {
            push_new(&mut check.invariants, &each.name);
        }
        if mark.kind != Kind::Holds {
            push_new(&mut job.expected, &each.name);
        }
        if mark.kind == Kind::Reachable {
            push_new(&mut job.reachable, &each.name);
        }
        if let Some(says) = mark.says {
            job.said.push((each.name.clone(), says));
        }
    }
    Ok(())
}

struct Compiled {
    json: String,
    model: earnest_ir::Model,
    expected: Vec<String>,
    reachable: Vec<String>,
}

fn compiled_once(
    checks: &[Job],
    built: Vec<Option<Result<String, Problem>>>,
    jobs: usize,
) -> Vec<Result<String, Problem>> {
    let mut alike: Vec<usize> = Vec::with_capacity(checks.len());
    let mut to_compile: Vec<&Job> = Vec::new();
    for (job, built) in checks.iter().zip(&built) {
        let same = |other: &&Job| {
            other.real == job.real && compiled_as(&other.check) == compiled_as(&job.check)
        };
        let at = match to_compile.iter().position(same) {
            Some(at) => at,
            None if built.is_none() => {
                to_compile.push(job);
                to_compile.len().saturating_sub(1)
            }
            None => usize::MAX,
        };
        alike.push(at);
    }
    let once = run_each(&to_compile, jobs, |job, _| compile(job));
    built
        .into_iter()
        .zip(alike)
        .map(|(built, at)| match built {
            Some(done) => done,
            None => once
                .get(at)
                .cloned()
                .unwrap_or_else(|| Err(Problem::new("", TOP_OF_FILE, "`quint`", "no compile ran"))),
        })
        .collect()
}

fn runs_not_played(
    checks: &[Job],
    compiled: &[Result<(String, bool), Problem>],
    places: &HashMap<PathBuf, Places>,
) -> Vec<Vec<String>> {
    checks
        .iter()
        .zip(compiled)
        .map(|(job, compiled)| match (compiled, places.get(&job.real)) {
            (Ok((json, false)), Some(found)) if job.check.runs == Runs::Tests => {
                found.runs_not_played(&job.check.main, json)
            }
            _ => Vec::new(),
        })
        .collect()
}

fn read_check(job: &Job, compiled: Result<String, Problem>) -> Result<Compiled, Vec<Located>> {
    let json = compiled.map_err(|problem| vec![Located::new(problem, None, None)])?;
    let model = if runs_only(&job.check) {
        earnest_quint::read_runs(&job.shown, &json, &job.check.runs)
    } else {
        earnest_quint::read_located(&job.shown, &json, &job.check.runs)
    };
    let mut model = model?;
    job.model_name().clone_into(&mut model.name);
    let expected =
        expected_ids(job, &model).map_err(|problem| vec![Located::new(problem, None, None)])?;
    let reachable = as_marked(job, &mut model);
    Ok(Compiled {
        json,
        model,
        expected,
        reachable,
    })
}

fn as_marked(job: &Job, model: &mut earnest_ir::Model) -> Vec<String> {
    let ids = whole_ids(model);
    let reachable: Vec<String> = job
        .reachable
        .iter()
        .filter_map(|name| id_of(&ids, name).cloned())
        .collect();
    let said: Vec<(&String, &String)> = job
        .said
        .iter()
        .filter_map(|(name, says)| Some((id_of(&ids, name)?, says)))
        .collect();
    for rule in &mut model.rules {
        let id = whole_of(&rule.id);
        if reachable.iter().any(|each| each == id) {
            rule.holds = Code::new(Expr::Apply(Op::Not, vec![rule.holds.expr().clone()]));
        }
        if let Some((_, says)) = said.iter().find(|(each, _)| *each == id) {
            rule.says.clone_from(says);
        }
    }
    for rule in &mut model.runs.eventually {
        if let Some((_, says)) = said.iter().find(|(each, _)| *each == whole_of(&rule.id)) {
            rule.says.clone_from(says);
        }
    }
    reachable
}

fn compiled_all(
    checks: &[Job],
    refusals: &[Option<Problem>],
    (typechecked, places): (&HashMap<PathBuf, Typechecked>, &HashMap<PathBuf, Places>),
    jobs: usize,
) -> Vec<Result<(String, bool), Problem>> {
    let paired: Vec<(&Job, Option<&Problem>)> = checks
        .iter()
        .zip(refusals.iter().map(Option::as_ref))
        .collect();
    let built = run_each(&paired, jobs, |(job, refusal), _| match refusal {
        Some(problem) => (Some(Err((*problem).clone())), false),
        None => match filled(job, typechecked.get(&job.real)) {
            Some(json) => (Some(Ok(json)), true),
            None => (
                lacks_a_search(&job.check, places.get(&job.real))
                    .then(|| compiled_for_runs(job, places.get(&job.real))),
                false,
            ),
        },
    });
    let (built, from_typecheck): (Vec<_>, Vec<bool>) = built.into_iter().unzip();
    compiled_once(checks, built, jobs)
        .into_iter()
        .zip(from_typecheck)
        .map(|(compiled, from)| compiled.map(|json| (json, from)))
        .collect()
}

fn parsed_all(
    checks: &[Job],
    candidates: &[Candidate],
    jobs: usize,
) -> HashMap<PathBuf, Result<Places, Problem>> {
    let mut to_parse: Vec<(PathBuf, String, Quint)> = Vec::new();
    let reals = checks
        .iter()
        .map(|job| (&job.real, job.shown.clone(), &job.quint))
        .chain(
            candidates
                .iter()
                .map(|each| (&each.real, shown_string(&each.shown), &each.quint)),
        );
    for (real_file, shown, quint) in reals {
        if let Ok(quint) = quint
            && !to_parse.iter().any(|(file, ..)| file == real_file)
        {
            to_parse.push((real_file.clone(), shown, quint.clone()));
        }
    }
    let maps = run_each(&to_parse, jobs, |(file, shown, quint), _| {
        parsed(file, shown, quint)
    });
    to_parse
        .into_iter()
        .zip(maps)
        .map(|((file, ..), places)| (file, places))
        .collect()
}

pub(crate) fn checked(listed: &[Listed], jobs: usize) -> Vec<ModelFile> {
    let mut files = Vec::new();
    let (mut checks, candidates) = jobs_of(asked(listed), &mut files);
    for candidate in &candidates {
        if let Err(message) = &candidate.quint {
            files.push(problem_file(
                &shown_string(&candidate.shown),
                "`quint`",
                message,
            ));
        }
    }
    let read = parsed_all(&checks, &candidates, jobs);
    for candidate in &candidates {
        if let Some(places) = read.get(&candidate.real) {
            checks.extend(marked_checks(candidate, places.as_ref(), &mut files));
        }
    }
    let places: HashMap<PathBuf, Places> = read
        .into_iter()
        .filter_map(|(file, places)| Some((file, places.ok()?)))
        .collect();
    let mut refusals: Vec<Option<Problem>> = Vec::with_capacity(checks.len());
    for job in &mut checks {
        if let Some(found) = places.get(&job.real) {
            job.read = found.read;
        }
        let marked = with_marks(job, places.get(&job.real));
        refusals.push(
            marked
                .and_then(|()| widened(job, places.get(&job.real)))
                .err(),
        );
    }
    let mut to_typecheck: Vec<(PathBuf, String, Quint)> = Vec::new();
    for job in &checks {
        if let Ok(quint) = &job.quint
            && places.get(&job.real).is_some_and(|found| {
                found.may_be_a_setup(&job.check.main) || lacks_a_search(&job.check, Some(found))
            })
            && !to_typecheck.iter().any(|(file, ..)| *file == job.real)
        {
            to_typecheck.push((job.real.clone(), job.shown.clone(), quint.clone()));
        }
    }
    let typechecks = run_each(&to_typecheck, jobs, |(file, shown, quint), _| {
        typechecked(file, shown, quint, places.get(file)?)
    });
    let typechecked: HashMap<PathBuf, Typechecked> = to_typecheck
        .into_iter()
        .zip(typechecks)
        .filter_map(|((file, ..), read)| Some((file, read?)))
        .collect();
    let compiled = compiled_all(&checks, &refusals, (&typechecked, &places), jobs);
    let not_played = runs_not_played(&checks, &compiled, &places);
    let compiled = compiled.into_iter().map(|each| each.map(|(json, _)| json));
    let read: Vec<Result<Compiled, Vec<Located>>> = checks
        .iter()
        .zip(compiled)
        .map(|(job, compiled)| read_check(job, compiled))
        .collect();
    for ((job, read), runs_not_played) in checks.iter().zip(read).zip(not_played) {
        files.push(match read {
            Ok(Compiled {
                json,
                model,
                expected,
                reachable,
            }) => ModelFile {
                path: job.shown.clone(),
                source: format!(
                    "{json}\n{:?}\n{:?}\n{:?}",
                    job.check, job.reachable, job.said
                ),
                declared: vec![Declared {
                    name: model.name.clone(),
                    rules: model.rule_ids().map(str::to_owned).collect(),
                }],
                content: Content::Models(vec![Readable {
                    draft: None,
                    model,
                    position: 0,
                    expected,
                    reachable,
                    counted: true,
                    runs_not_played,
                    quint: job.quint.as_ref().ok().map(|quint| as_asked(job, quint)),
                }]),
            },
            Err(located) => {
                let problems = match places.get(&job.real) {
                    Some(found) => {
                        place_all(located, found, &job.check.main, &job.real, &job.shown)
                    }
                    None => located.into_iter().map(|each| each.problem).collect(),
                };
                unreadable(&job.shown, vec![declared(job)], problems)
            }
        });
    }
    files
}

#[cfg(test)]
mod tests {
    use super::{Quint, places_key};
    use crate::cache::ENGINE;
    use std::path::PathBuf;

    #[test]
    fn what_another_build_read_from_a_parse_is_not_kept_for_this_build() {
        let quint = Quint {
            program: PathBuf::from("quint"),
            version: "0.33.1".to_owned(),
        };
        let source = b"module counter { var n: int }";
        let ours = places_key(ENGINE, &quint, source);
        assert_eq!(
            ours,
            places_key(ENGINE, &quint, source),
            "one build, one Quint and one file keep one entry"
        );
        assert_ne!(
            ours,
            places_key("0.1.0+0000000000000000", &quint, source),
            "the entry holds what the checker made of Quint's output, so a build from other \
             sources reads the parse again"
        );
        let newer = Quint {
            version: "0.34.0".to_owned(),
            ..quint.clone()
        };
        assert_ne!(ours, places_key(ENGINE, &newer, source), "another Quint");
        assert_ne!(
            ours,
            places_key(ENGINE, &quint, b"module counter { var m: int }"),
            "another text"
        );
    }
}
