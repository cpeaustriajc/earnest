use crate::cache::{fingerprint_of, folder_in, write_whole};
use crate::checks::{CHECKS_FILE, Check, example, read_checks};
use crate::discovery::Listed;
use crate::places::{Places, place_all};
use crate::reading::{Content, Declared, ModelFile, Readable};
use crate::typecheck::typechecked;
use crate::workers::run_each;
use earnest_diagnostics::{Place, Problem};
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
    named: Vec<PathBuf>,
}

#[derive(Debug)]
struct Job {
    shown: String,
    real: PathBuf,
    check: Check,
    read: u64,
    quint: Result<Quint, String>,
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

fn not_listed(qnt: &Path, checks: &Path) -> ModelFile {
    let name = qnt.file_name().map_or_else(
        || shown_string(qnt),
        |name| name.to_string_lossy().into_owned(),
    );
    let main = qnt.file_stem().map_or_else(
        || "Main".to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    problem_file(
        &shown_string(qnt),
        &format!("`{name}`"),
        &format!(
            "the Quint file is not listed in `{}`, so the checker does not know which module \
             to check and what holds in it: list it there, for example:\n{}",
            shown_string(checks),
            example(&name, &main)
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
                slot.named.push(qnt.clone());
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

fn runs_key(runs: &Runs) -> String {
    match runs {
        Runs::None => "none".to_owned(),
        Runs::Tests => "tests".to_owned(),
        Runs::Named(names) => format!("named:{}", names.join(",")),
        other => format!("{other:?}"),
    }
}

fn key_of(job: &Job, quint: &Quint, source: &[u8]) -> String {
    let check = &job.check;
    let listed = [
        quint.version.as_str(),
        &check.main,
        &check.init,
        &check.step,
        &check.invariants.join(","),
        &check.temporal.join(","),
        &runs_key(&check.runs),
    ];
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
    let target = cache_path(&format!(
        "{:016x}.json",
        fingerprint_of(&[
            shown_string(&job.real).as_bytes(),
            job.check.main.as_bytes()
        ])
    ));
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
            shown: job.shown.clone(),
            real: job.real.clone(),
            check: Check {
                init,
                step,
                ..check.clone()
            },
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

fn parsed(real_file: &Path, shown: &str, quint: &Quint) -> Option<Places> {
    let source = fs::read(real_file).ok()?;
    let key = format!(
        "{:016x}",
        fingerprint_of(&[quint.version.as_bytes(), &source])
    );
    let target = cache_path(&format!(
        "places-{:016x}.json",
        fingerprint_of(&[shown_string(real_file).as_bytes()])
    ));
    if let Some(text) = kept(&target, &key)
        && let Ok(places) = serde_json::from_str::<Places>(&text)
        && places.read == fingerprint_files(&places.files_read())
    {
        return Some(places);
    }
    let (out, map) = (scratch("parse").ok()?, scratch("map").ok()?);
    let ran = Command::new(&quint.program)
        .arg("parse")
        .arg(shown)
        .arg("--out")
        .arg(&out)
        .arg("--source-map")
        .arg(&map)
        .output();
    let read = || -> Option<Places> {
        if !ran.as_ref().ok()?.status.success() {
            return None;
        }
        Places::from_quint(
            &fs::read_to_string(&out).ok()?,
            &fs::read_to_string(&map).ok()?,
        )
    };
    let places = read();
    fs::remove_file(&out).unwrap_or_default();
    fs::remove_file(&map).unwrap_or_default();
    let mut places = places?;
    places.read = fingerprint_files(&places.files_read());
    if let Ok(text) = serde_json::to_string(&places) {
        store(&target, &format!("{key}\n{text}"));
    }
    Some(places)
}

fn declared(check: &Check) -> Declared {
    Declared {
        name: check.main.clone(),
        rules: check
            .invariants
            .iter()
            .chain(&check.temporal)
            .cloned()
            .collect(),
    }
}

fn jobs_of(asked: BTreeMap<PathBuf, Asked>, files: &mut Vec<ModelFile>) -> Vec<Job> {
    let mut programs: HashMap<PathBuf, Result<Quint, String>> = HashMap::new();
    let mut jobs = Vec::new();
    for (real_checks, asked) in asked {
        let shown_checks = shown_string(&asked.shown);
        let text = match fs::read_to_string(&asked.shown) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound && asked.only.is_some() => {
                files.extend(asked.named.iter().map(|qnt| not_listed(qnt, &asked.shown)));
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
        let checks = match read_checks(&text) {
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
        let mut matched: BTreeSet<PathBuf> = BTreeSet::new();
        for check in checks {
            let shown = folder.join(&check.file);
            let real_file = real(&shown);
            if let Some(only) = &asked.only {
                if !only.contains(&real_file) {
                    continue;
                }
                matched.insert(real_file.clone());
            }
            let quint = programs
                .entry(real_folder.clone())
                .or_insert_with(|| {
                    let program = find_program(&real_folder)?;
                    let version = version_of(&program)?;
                    Ok(Quint { program, version })
                })
                .clone();
            jobs.push(Job {
                shown: shown_string(&shown),
                real: real_file,
                check,
                read: tree,
                quint,
            });
        }
        files.extend(
            asked
                .named
                .iter()
                .filter(|qnt| !matched.contains(&real(qnt)))
                .map(|qnt| not_listed(qnt, &asked.shown)),
        );
    }
    jobs
}

pub(crate) fn checked(listed: &[Listed], jobs: usize) -> Vec<ModelFile> {
    let mut files = Vec::new();
    let mut checks = jobs_of(asked(listed), &mut files);
    let mut to_parse: Vec<(PathBuf, String, Quint)> = Vec::new();
    for job in &checks {
        if let Ok(quint) = &job.quint
            && !to_parse.iter().any(|(file, ..)| *file == job.real)
        {
            to_parse.push((job.real.clone(), job.shown.clone(), quint.clone()));
        }
    }
    let maps = run_each(&to_parse, jobs, |(file, shown, quint), _| {
        parsed(file, shown, quint)
    });
    let places: HashMap<PathBuf, Places> = to_parse
        .into_iter()
        .zip(maps)
        .filter_map(|((file, ..), places)| Some((file, places?)))
        .collect();
    for job in &mut checks {
        if let Some(found) = places.get(&job.real) {
            job.read = found.read;
        }
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
    let compiled = run_each(&checks, jobs, |job, _| {
        filled(job, typechecked.get(&job.real)).map_or_else(
            || {
                if lacks_a_search(&job.check, places.get(&job.real)) {
                    compiled_for_runs(job, places.get(&job.real))
                } else {
                    compile(job)
                }
            },
            Ok,
        )
    });
    let read: Vec<Result<(String, earnest_ir::Model), Vec<Located>>> = checks
        .iter()
        .zip(compiled)
        .map(|(job, compiled)| match compiled {
            Ok(json) if runs_only(&job.check) => {
                earnest_quint::read_runs(&job.shown, &json, &job.check.runs)
                    .map(|model| (json, model))
            }
            Ok(json) => earnest_quint::read_located(&job.shown, &json, &job.check.runs)
                .map(|model| (json, model)),
            Err(problem) => Err(vec![Located::new(problem, None, None)]),
        })
        .collect();
    for (job, read) in checks.iter().zip(read) {
        files.push(match read {
            Ok((json, model)) => ModelFile {
                path: job.shown.clone(),
                source: json,
                declared: vec![Declared {
                    name: model.name.clone(),
                    rules: model.rule_ids().map(str::to_owned).collect(),
                }],
                content: Content::Models(vec![Readable {
                    draft: None,
                    model,
                    position: 0,
                }]),
            },
            Err(located) => {
                let problems = match places.get(&job.real) {
                    Some(found) => {
                        place_all(located, found, &job.check.main, &job.real, &job.shown)
                    }
                    None => located.into_iter().map(|each| each.problem).collect(),
                };
                unreadable(&job.shown, vec![declared(&job.check)], problems)
            }
        });
    }
    files
}
