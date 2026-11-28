use crate::cache::folder_in;
use crate::checks::{CHECKS_FILE, Check};
use crate::discovery::Listed;
use crate::marks::how_to_mark;
use crate::migrate::migration;
use crate::places::{Places, place_all};
use crate::plan::{Labels, Plan, Spot, Viewing, planned};
use crate::reading::{Content, Declared, ModelFile, Readable};
use crate::run::whole_of;
use crate::typecheck::typechecked;
use crate::workers::{run_each, stack_of_this_worker};
use earnest_diagnostics::{Place, Problem};
use earnest_ir::{Code, Expr, Op};
use earnest_quint::{Located, Runs, Setup, Typechecked};
use earnest_quint_syntax::{Parsed, Target, on_a_stack_of, on_large_stack, read};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

mod labels;

use labels::{at_marks, blamed, label_names, label_unbuilt, labelled, without_labels};

const CACHE: &str = "quint";
const HERE: &str = ".";
const TOP_OF_FILE: Place = Place { line: 1, column: 1 };
const QUALIFIED: &str = "::";
const PARSE: &str = "parsing as Quint";
const NOT_PARSED: &str = "the Quint file can't be parsed";
const NOT_READ: &str = "unreadable";
/// The feature of a problem the reading of a compiled module meets outside the reader itself,
/// such as a thread the system refuses. `reading.rs` names it so for a `compiled.json`.
pub(crate) const READER: &str = "the reader";
const MISSING_MODULE: &str = "QNT405";
const FLATTENER_STOPPED: &str = "Internal error while flattening";

#[derive(Debug)]
struct Candidate {
    shown: PathBuf,
    real: PathBuf,
    named: bool,
}

#[derive(Debug)]
struct Job {
    shown: String,
    real: PathBuf,
    check: Check,
    expected: Vec<String>,
    reachable: Vec<String>,
    said: Vec<(String, String)>,
    labels: Labels,
}

impl Job {
    fn model_name(&self) -> &str {
        &self.check.main
    }
}

#[derive(Clone, Debug)]
pub(crate) struct QuintProperty {
    pub(crate) id: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug)]
pub(crate) struct QuintCheck {
    pub(crate) file: PathBuf,
    pub(crate) main: String,
    pub(crate) init: String,
    pub(crate) step: String,
    pub(crate) invariants: Vec<QuintProperty>,
    pub(crate) temporal: Vec<QuintProperty>,
    pub(crate) runs: Runs,
    pub(crate) searches: bool,
}

fn as_asked(job: &Job) -> QuintCheck {
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
        &format!("no module of the Quint file is checked: {}", how_to_mark()),
    )
}

fn found(listed: &[Listed]) -> (Vec<PathBuf>, Vec<Candidate>) {
    let mut old: Vec<PathBuf> = Vec::new();
    let mut seen_old: BTreeSet<PathBuf> = BTreeSet::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    for each in listed {
        if (each.only.is_none() || each.checks.is_file()) && seen_old.insert(real(&each.checks)) {
            old.push(each.checks.clone());
        }
        let Some(qnt) = &each.only else {
            continue;
        };
        let real_file = real(qnt);
        if let Some(kept) = candidates.iter_mut().find(|kept| kept.real == real_file) {
            kept.named |= each.named;
            continue;
        }
        candidates.push(Candidate {
            shown: qnt.clone(),
            real: real_file,
            named: each.named,
        });
    }
    candidates.sort_by(|first, second| first.shown.cmp(&second.shown));
    (old, candidates)
}

pub(crate) fn cache_path(name: &str) -> PathBuf {
    folder_in(Path::new(HERE), CACHE).join(name)
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

fn normalized(path: &Path) -> String {
    let whole = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut parts: Vec<Component<'_>> = Vec::new();
    for part in whole.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(parts.last(), Some(Component::Normal(_))) {
                    parts.pop();
                }
            }
            other => parts.push(other),
        }
    }
    shown_string(&parts.iter().collect::<PathBuf>()).replace('\\', "/")
}

pub(crate) fn quint_text(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

const _: () = assert!(
    earnest_check::WORKER_STACK >= earnest_quint_syntax::STACK_NEEDED,
    "a worker's stack must hold the front end, which runs on it with no thread of its own"
);

pub(crate) fn with_front_end<T>(work: impl FnOnce() -> T) -> T {
    on_a_stack_of(stack_of_this_worker().unwrap_or(0), work)
}

pub(crate) fn front_end(shown: &str) -> Result<Parsed, earnest_quint_syntax::Problem> {
    read(&normalized(Path::new(shown)), |path| quint_text(path).ok())
}

fn placed(
    problem: &earnest_quint_syntax::Problem,
    real_file: &Path,
    shown: &str,
) -> (String, Place) {
    let at = Place {
        line: u32::try_from(problem.line).unwrap_or(u32::MAX).max(1),
        column: u32::try_from(problem.column).unwrap_or(u32::MAX).max(1),
    };
    if problem.source.is_empty() {
        return (shown.to_owned(), TOP_OF_FILE);
    }
    (
        shown_in_folder(Path::new(&problem.source), real_file, shown),
        at,
    )
}

fn parse_failed(shown: &str, real_file: &Path, problem: &earnest_quint_syntax::Problem) -> Problem {
    let (file, place) = placed(problem, real_file, shown);
    let message = if problem.code == NOT_READ {
        problem.message.clone()
    } else {
        format!("[{}] {}", problem.code, problem.message)
    };
    Problem::new(&file, place, PARSE, &message)
}

fn compile_said(problem: &earnest_quint_syntax::Problem) -> (String, bool) {
    let first = problem.message.lines().next().unwrap_or_default();
    if problem.code == NOT_READ {
        (first.to_owned(), !problem.source.is_empty())
    } else if problem.message.starts_with(FLATTENER_STOPPED) {
        (format!("Error: {first}"), false)
    } else if problem.source.is_empty() && problem.code == MISSING_MODULE {
        (format!("error: {first}"), false)
    } else if problem.source.is_empty() {
        (format!("error: [{}] {first}", problem.code), false)
    } else {
        (format!("Error [{}]: {first}", problem.code), true)
    }
}

fn compile_failed(job: &Job, problem: &earnest_quint_syntax::Problem) -> Problem {
    let (said, at_its_place) = compile_said(problem);
    let (file, place) = if at_its_place {
        placed(problem, &job.real, &job.shown)
    } else {
        (job.shown.clone(), TOP_OF_FILE)
    };
    Problem::new(
        &file,
        place,
        &format!("compiling `{}` as Quint", job.check.main),
        &said,
    )
}

fn target_of(check: &Check) -> Target {
    let joined = |names: &[String]| (!names.is_empty()).then(|| names.join(","));
    Target {
        main: check.main.clone(),
        init: check.init.clone(),
        step: check.step.clone(),
        invariant: joined(&check.invariants),
        temporal: joined(&check.temporal),
    }
}

fn compile(job: &Job) -> Result<String, Problem> {
    let target = target_of(&labelled(job));
    #[cfg(test)]
    tests::record(&target, &job.shown);
    with_front_end(|| {
        let parsed = front_end(&job.shown).map_err(|problem| compile_failed(job, &problem))?;
        let compiled = parsed
            .compile(&target)
            .map_err(|problem| compile_failed(job, &problem))?;
        Ok(compiled.output().to_string())
    })
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
            "the check lists only runs, and the module `{}` sees no `{}` and `{}`, which compiling \
             it as Quint needs, nor does it import a module that has an `init` and a `step`: \
             name an `init` and a `step` the module sees in its mark, as `/// @check init = \
             A::init, step = A::step` under `import Spec as A`. Quint's message: {}",
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
            expected: job.expected.clone(),
            reachable: job.reachable.clone(),
            said: job.said.clone(),
            labels: job.labels.clone(),
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
            Problem::new(
                &job.shown,
                TOP_OF_FILE,
                &format!("compiling `{}` as Quint", check.main),
                "nothing was compiled",
            )
        }),
    ))
}

fn compiled_runs(found: &Places, job: &Job, json: &str, unplayed: &mut Vec<String>) -> Runs {
    let main = &job.check.main;
    let held = found.runs_held(main, json);
    let (kept, missing): (Vec<String>, Vec<String>) =
        job.check.runs.listed().iter().cloned().partition(|name| {
            held.iter()
                .any(|each| each == name || each.ends_with(&format!("::{name}")))
        });
    for name in missing {
        if !unplayed.contains(&name) {
            unplayed.push(name);
        }
    }
    match &job.check.runs {
        Runs::Tests | Runs::TestsAnd(_) => match found.runs_played(main, json) {
            Some(mut tests) => {
                tests.extend(kept);
                Runs::Named(tests)
            }
            None if kept.is_empty() => Runs::Tests,
            None => Runs::TestsAnd(kept),
        },
        Runs::Named(_) => Runs::Named(kept),
        other => other.clone(),
    }
}

fn not_built(
    job: &Job,
    typechecked: Option<&Typechecked>,
    found: Option<&Places>,
) -> Option<String> {
    let Some(typechecked) = typechecked else {
        return Some("typechecking as Quint gave nothing to build the check from".to_owned());
    };
    let (id, written) = typechecked.not_built(&job.check.main)?;
    let at =
        found
            .and_then(|found| found.spot_of(id))
            .map_or_else(String::new, |(file, line, _)| {
                format!(
                    " at `{}:{}`",
                    shown_in(Path::new(file), job),
                    line.saturating_add(1)
                )
            });
    Some(format!(
        "the checker does not build `{written}`{at} from the file typechecked as Quint"
    ))
}

fn builds(found: &Places, job: &Job) -> bool {
    let main = &job.check.main;
    found.may_be_a_setup(main)
        || lacks_a_search(&job.check, Some(found))
        || (job.check.invariants.is_empty() && job.check.temporal.is_empty())
        || found.plays_an_imported_run(main, &job.check.runs)
        || found.imports_whole(main)
}

fn filled(job: &Job, typechecked: Option<&Typechecked>) -> Option<Result<String, Problem>> {
    let check = &job.check;
    let built = |labels: &[String]| {
        typechecked?.setup(
            &Setup::new(
                &check.main,
                (&check.init, &check.step),
                (&check.invariants, &check.temporal),
                &check.runs,
            )
            .with_labels(labels),
        )
    };
    let labels = label_names(job);
    match built(&labels) {
        Some(json) => Some(Ok(json)),
        None if !labels.is_empty() => built(&[]).map(|_| Err(label_unbuilt(job))),
        None => None,
    }
}

pub(crate) fn parsed(real_file: &Path, shown: &str) -> Result<Places, Problem> {
    if let Err(error) = fs::read(real_file) {
        return Err(Problem::new(
            shown,
            TOP_OF_FILE,
            PARSE,
            &format!("the Quint file can't be read: {error}"),
        ));
    }
    let not_parsed = || Problem::new(shown, TOP_OF_FILE, PARSE, NOT_PARSED);
    if !Path::new(shown).is_file() {
        return Err(not_parsed());
    }
    on_large_stack(stack_of_this_worker().unwrap_or(0), || {
        let parsed =
            front_end(shown).map_err(|problem| parse_failed(shown, real_file, &problem))?;
        Places::from_quint(
            &parsed.output().to_string(),
            &parsed.source_map().to_string(),
        )
        .ok_or_else(not_parsed)
    })
    .unwrap_or_else(|problem| Err(parse_failed(shown, real_file, &problem)))
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

fn expected_ids(
    job: &Job,
    model: &earnest_ir::Model,
    unplayed: &[String],
) -> Result<Vec<String>, Problem> {
    let ids = whole_ids(model);
    let mut expected: Vec<String> = Vec::with_capacity(job.expected.len());
    for name in &job.expected {
        let Some(found) = id_of(&ids, name) else {
            if id_of(unplayed, name).is_some() {
                continue;
            }
            return Err(Problem::new(
                &job.shown,
                TOP_OF_FILE,
                &format!("`{name}`"),
                &format!(
                    "the module `{}` has no property `{name}` to judge",
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

fn job_of(candidate: &Candidate, plan: Plan) -> Job {
    Job {
        shown: shown_string(&candidate.shown),
        real: candidate.real.clone(),
        check: Check {
            file: candidate
                .shown
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            main: plan.main,
            init: plan.init,
            step: plan.step,
            invariants: plan.invariants,
            temporal: plan.temporal,
            runs: plan.runs,
        },
        expected: plan.expected,
        reachable: plan.reachable,
        said: plan.said,
        labels: plan.labels,
    }
}

fn unreadable_of(shown: &str, module: Option<String>, problems: Vec<Problem>) -> ModelFile {
    let declared = module.map_or_else(Vec::new, |name| {
        vec![Declared {
            name,
            rules: Vec::new(),
        }]
    });
    unreadable(shown, declared, problems)
}

fn marked_checks(
    candidates: &[Candidate],
    read: &HashMap<PathBuf, Result<Places, Problem>>,
    files: &mut Vec<ModelFile>,
) -> Vec<Job> {
    let mut jobs = Vec::new();
    let mut seen: BTreeSet<Spot> = BTreeSet::new();
    let mut marked: Vec<(String, Spot, Problem)> = Vec::new();
    let mut viewing = Viewing::default();
    for candidate in candidates {
        let shown = shown_string(&candidate.shown);
        let places = match read.get(&candidate.real) {
            Some(Ok(places)) => places,
            Some(Err(problem)) => {
                files.push(unreadable(&shown, Vec::new(), vec![problem.clone()]));
                continue;
            }
            None => continue,
        };
        let mut found = planned(places, &candidate.real, &shown);
        viewing.add(&shown, std::mem::take(&mut found.viewing));
        seen.extend(found.seen);
        marked.extend(
            found
                .marked
                .into_iter()
                .map(|(spot, problem)| (shown.clone(), spot, problem)),
        );
        let mut by_module: Vec<(Option<String>, Vec<Problem>)> = Vec::new();
        for (module, problem) in found.problems {
            match by_module.iter_mut().find(|(each, _)| *each == module) {
                Some((_, problems)) => problems.push(problem),
                None => by_module.push((module, vec![problem])),
            }
        }
        let checks = found.plans.len();
        let refused = by_module
            .iter()
            .filter(|(module, _)| module.is_some())
            .count();
        for (module, mut problems) in by_module {
            problems.sort_by_key(Problem::place);
            files.push(unreadable_of(&shown, module, problems));
        }
        jobs.extend(found.plans.into_iter().map(|plan| job_of(candidate, plan)));
        if checks == 0 && refused == 0 && !found.module_marked && candidate.named {
            files.push(not_a_model(&candidate.shown));
        }
    }
    let unseen: Vec<(String, Problem)> = marked
        .into_iter()
        .filter(|(_, spot, _)| !seen.contains(spot))
        .map(|(shown, _, problem)| (shown, problem))
        .chain(viewing.unseen())
        .collect();
    for (shown, problem) in unseen {
        files.push(unreadable(&shown, Vec::new(), vec![problem]));
    }
    jobs
}

struct Compiled {
    json: String,
    model: earnest_ir::Model,
    expected: Vec<String>,
    reachable: Vec<String>,
    not_judged: Vec<String>,
    unread: Vec<Located>,
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
            other.real == job.real && compiled_as(&labelled(other)) == compiled_as(&labelled(job))
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
            None => once.get(at).cloned().unwrap_or_else(|| {
                Err(Problem::new(
                    "",
                    TOP_OF_FILE,
                    "compiling as Quint",
                    "no compile ran",
                ))
            }),
        })
        .collect()
}

type ReadCheck = (Result<Compiled, Vec<Located>>, Vec<String>);

fn read_compiled(
    checks: &[Job],
    compiled: Vec<Result<(String, bool), Problem>>,
    places: &HashMap<PathBuf, Places>,
) -> Vec<ReadCheck> {
    let read = move || {
        checks
            .iter()
            .zip(compiled)
            .map(|(job, compiled)| read_one(job, compiled, places.get(&job.real)))
            .collect()
    };
    on_large_stack(stack_of_this_worker().unwrap_or(0), read).unwrap_or_else(|problem| {
        checks
            .iter()
            .map(|job| {
                let refused = Problem::new(&job.shown, TOP_OF_FILE, READER, &problem.message);
                (Err(vec![Located::new(refused, None, None)]), Vec::new())
            })
            .collect()
    })
}

fn read_one(
    job: &Job,
    compiled: Result<(String, bool), Problem>,
    found: Option<&Places>,
) -> ReadCheck {
    let mut unplayed = match (&compiled, found) {
        (Ok((json, false)), Some(found))
            if matches!(job.check.runs, Runs::Tests | Runs::TestsAnd(_)) =>
        {
            found.runs_not_played(&job.check.main, json)
        }
        _ => Vec::new(),
    };
    let runs = match (&compiled, found) {
        (Ok((json, false)), Some(found)) => compiled_runs(found, job, json, &mut unplayed),
        _ => job.check.runs.clone(),
    };
    let read = read_check(job, compiled.map(|(json, _)| json), (&runs, &unplayed));
    (read, unplayed)
}

fn read_check(
    job: &Job,
    compiled: Result<String, Problem>,
    (runs, unplayed): (&Runs, &[String]),
) -> Result<Compiled, Vec<Located>> {
    let json = compiled.map_err(|problem| vec![Located::new(problem, None, None)])?;
    let (json, views) = without_labels(job, &json)?;
    let read = if runs_only(&job.check) {
        earnest_quint::read_runs_in_part(&job.shown, &json, runs)
    } else {
        earnest_quint::read_viewed_in_part(&job.shown, &json, runs, &views)
    };
    let read = read.map_err(|found| at_marks(job, &views, found))?;
    let mut model = read.model;
    job.model_name().clone_into(&mut model.name);
    let left_out: Vec<String> = unplayed.iter().chain(&read.not_judged).cloned().collect();
    let expected = expected_ids(job, &model, &left_out)
        .map_err(|problem| vec![Located::new(problem, None, None)])?;
    let reachable = as_marked(job, &mut model);
    Ok(Compiled {
        json,
        model,
        expected,
        reachable,
        not_judged: read.not_judged,
        unread: at_marks(job, &views, read.unread),
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
        None => match filled(
            job,
            typechecked.get(&job.real).filter(|_| {
                places
                    .get(&job.real)
                    .is_some_and(|found| builds(found, job))
            }),
        ) {
            Some(json) => (Some(json), true),
            None => (
                lacks_a_search(&job.check, places.get(&job.real))
                    .then(|| compiled_for_runs(job, places.get(&job.real))),
                false,
            ),
        },
    });
    let (built, from_typecheck): (Vec<_>, Vec<bool>) = built.into_iter().unzip();
    blamed(checks, compiled_once(checks, built, jobs))
        .into_iter()
        .zip(from_typecheck)
        .map(|(compiled, from)| compiled.map(|json| (json, from)))
        .collect()
}

fn parsed_all(candidates: &[Candidate], jobs: usize) -> HashMap<PathBuf, Result<Places, Problem>> {
    let mut to_parse: Vec<(PathBuf, String)> = Vec::new();
    for each in candidates {
        if !to_parse.iter().any(|(file, _)| *file == each.real) {
            to_parse.push((each.real.clone(), shown_string(&each.shown)));
        }
    }
    let maps = run_each(&to_parse, jobs, |(file, shown), _| parsed(file, shown));
    to_parse
        .into_iter()
        .zip(maps)
        .map(|((file, ..), places)| (file, places))
        .collect()
}

fn found_checks(
    candidates: &[Candidate],
    jobs: usize,
    files: &mut Vec<ModelFile>,
) -> (Vec<Job>, HashMap<PathBuf, Places>) {
    let mut checks: Vec<Job> = Vec::new();
    let read = parsed_all(candidates, jobs);
    checks.extend(marked_checks(candidates, &read, files));
    let places = read
        .into_iter()
        .filter_map(|(file, places)| Some((file, places.ok()?)))
        .collect();
    (checks, places)
}

pub(crate) fn checked(listed: &[Listed], jobs: usize) -> Vec<ModelFile> {
    let mut files = Vec::new();
    let (old, candidates) = found(listed);
    let (checks, places) = found_checks(&candidates, jobs, &mut files);
    files.extend(old.iter().map(|checks_file| {
        problem_file(
            &shown_string(checks_file),
            &format!("`{CHECKS_FILE}`"),
            &migration(checks_file, &places),
        )
    }));
    let refusals: Vec<Option<Problem>> = vec![None; checks.len()];
    let mut to_typecheck: Vec<(PathBuf, String)> = Vec::new();
    for job in &checks {
        if places
            .get(&job.real)
            .is_some_and(|found| builds(found, job))
            && !to_typecheck.iter().any(|(file, _)| *file == job.real)
        {
            to_typecheck.push((job.real.clone(), job.shown.clone()));
        }
    }
    let typechecks = run_each(&to_typecheck, jobs, |(_, shown), _| typechecked(shown));
    let typechecked: HashMap<PathBuf, Typechecked> = to_typecheck
        .into_iter()
        .zip(typechecks)
        .filter_map(|((file, ..), read)| Some((file, read?)))
        .collect();
    let compiled = compiled_all(&checks, &refusals, (&typechecked, &places), jobs);
    let (read, not_played): (Vec<_>, Vec<_>) = read_compiled(&checks, compiled, &places)
        .into_iter()
        .unzip();
    for ((job, read), runs_not_played) in checks.iter().zip(read).zip(not_played) {
        files.push(match read {
            Ok(Compiled {
                json,
                model,
                expected,
                reachable,
                not_judged,
                unread,
            }) => {
                let unread = placed_all(unread, places.get(&job.real), job);
                ModelFile {
                    path: job.shown.clone(),
                    source: format!(
                        "{json}\n{:?}\n{:?}\n{:?}\n{:?}\n{not_judged:?}\n{unread:?}",
                        job.check,
                        job.reachable,
                        job.said,
                        job.labels.names().collect::<Vec<_>>()
                    ),
                    declared: vec![Declared {
                        name: model.name.clone(),
                        rules: model
                            .rule_ids()
                            .map(str::to_owned)
                            .chain(not_judged.iter().cloned())
                            .collect(),
                    }],
                    content: Content::Models(vec![Readable {
                        draft: None,
                        model,
                        position: 0,
                        expected,
                        reachable,
                        counted: true,
                        not_built: (!runs_not_played.is_empty())
                            .then(|| {
                                not_built(job, typechecked.get(&job.real), places.get(&job.real))
                            })
                            .flatten(),
                        runs_not_played,
                        not_judged,
                        unread,
                        quint: Some(as_asked(job)),
                    }]),
                }
            }
            Err(located) => unreadable(
                &job.shown,
                vec![declared(job)],
                placed_all(located, places.get(&job.real), job),
            ),
        });
    }
    one_per_file(files)
}

fn placed_all(located: Vec<Located>, places: Option<&Places>, job: &Job) -> Vec<Problem> {
    match places {
        Some(found) => place_all(located, found, &job.check.main, &job.real, &job.shown),
        None => located.into_iter().map(|each| each.problem).collect(),
    }
}

fn one_per_file(files: Vec<ModelFile>) -> Vec<ModelFile> {
    let mut kept: Vec<ModelFile> = Vec::with_capacity(files.len());
    for file in files {
        let alone = file.declared.is_empty();
        let Content::Unreadable(problems) = file.content else {
            kept.push(file);
            continue;
        };
        let earlier = kept.iter_mut().find(|other| {
            alone
                && other.path == file.path
                && other.declared.is_empty()
                && matches!(other.content, Content::Unreadable(_))
        });
        match earlier {
            Some(ModelFile {
                content: Content::Unreadable(found),
                ..
            }) => {
                found.extend(problems);
                found.sort_by_key(Problem::place);
            }
            _ => kept.push(ModelFile {
                content: Content::Unreadable(problems),
                ..file
            }),
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::{compile_said, compiled_all, found, found_checks};
    use crate::discovery::discover;
    use earnest_quint_syntax::{Problem, Target};
    use std::collections::{BTreeSet, HashMap};
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;

    const FOLDER_VARIABLE: &str = "EARNEST_LISTED_FOLDER";
    const LIST_VARIABLE: &str = "EARNEST_LISTED_CHECKS";

    static RECORDED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub(super) fn record(target: &Target, file: &str) {
        let mut words: Vec<&str> = vec![
            "compile",
            "--target",
            "json",
            "--main",
            &target.main,
            "--init",
            &target.init,
            "--step",
            &target.step,
        ];
        for (flag, text) in [
            ("--invariant", &target.invariant),
            ("--temporal", &target.temporal),
        ] {
            if let Some(text) = text {
                words.extend([flag, text.as_str()]);
            }
        }
        words.push(file);
        if let Ok(mut recorded) = RECORDED.lock() {
            recorded.push(words.join("\t"));
        }
    }

    #[test]
    #[ignore = "writes the list of checks to the file named by EARNEST_LISTED_CHECKS"]
    fn the_checks_earnest_compiles_are_listed_as_quint_compile_is_called() {
        let list = env::var(LIST_VARIABLE).expect("EARNEST_LISTED_CHECKS names the list to write");
        let folder = env::var(FOLDER_VARIABLE).unwrap_or_else(|_| "tasks".to_owned());
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("the repository is two folders above the CLI");
        let listed = discover(&[root.join(folder)])
            .expect("the folder can be searched")
            .listed;
        let (_, candidates) = found(&listed);
        let mut files = Vec::new();
        let (checks, places) = found_checks(&candidates, 1, &mut files);
        compiled_all(
            &checks,
            &vec![None; checks.len()],
            (&HashMap::new(), &places),
            1,
        );
        let prefix = format!("{}/", root.to_string_lossy());
        let lines: BTreeSet<String> = RECORDED
            .lock()
            .expect("the list can be read")
            .iter()
            .map(|line| line.replace(&prefix, ""))
            .collect();
        let mut text = lines.into_iter().collect::<Vec<_>>().join("\n");
        text.push('\n');
        fs::write(&list, text).expect("the list can be written");
    }

    fn problem(code: &str, message: &str, source: &str) -> Problem {
        Problem {
            code: code.to_owned(),
            message: message.to_owned(),
            source: source.to_owned(),
            line: 4,
            column: 22,
        }
    }

    #[test]
    fn a_compile_error_is_said_as_the_first_line_quint_prints_for_it() {
        assert_eq!(
            compile_said(&problem(
                "QNT000",
                "Couldn't unify int and bool\nTrying to unify int and bool",
                "/specs/a.qnt"
            )),
            (
                "Error [QNT000]: Couldn't unify int and bool".to_owned(),
                true
            ),
            "an error at a place: its first line, the place after it"
        );
        assert_eq!(
            compile_said(&problem("QNT404", "Name 'nope' not found", "")),
            ("error: [QNT404] Name 'nope' not found".to_owned(), false),
            "an error in what the command line names, with no place"
        );
        assert_eq!(
            compile_said(&problem("QNT405", "module zz does not exist", "")),
            ("error: module zz does not exist".to_owned(), false),
            "a main module that is not there, which Quint prints with no code"
        );
        assert_eq!(
            compile_said(&problem(
                "QNT000",
                "Internal error while flattening [QNT404] Name 'twice' not found",
                ""
            )),
            (
                "Error: Internal error while flattening [QNT404] Name 'twice' not found".to_owned(),
                false
            ),
            "the flattener's internal error, as Node prints what Quint throws"
        );
        assert_eq!(
            compile_said(&problem(
                "unreadable",
                "Quint's type checker throws here",
                "/a.qnt"
            )),
            ("Quint's type checker throws here".to_owned(), true),
            "a file the front end refuses where Quint throws, in its own words"
        );
        assert_eq!(
            compile_said(&problem(
                "unreadable",
                "earnest could not start the thread",
                ""
            )),
            ("earnest could not start the thread".to_owned(), false),
            "a refusal of earnest's own with no place, in its own words and with no code"
        );
    }
}
