use crate::arguments::Verify;
use crate::cache::{Cache, Key};
use crate::changes::{Compare, check_base, rule_changes};
use crate::commands::{Exit, Gathered, gather, limits_of, refusing, say_what_is_unread};
use crate::print::{Since, Verified, to_stderr, to_stdout};
use crate::reading::ModelFile;
use crate::run::{Bounds, Checked, Expecting, Graphs, Search, Stops, checked, explore_expecting};
use crate::select::{Target, select_with_removed};
use earnest_check::stopped_by_time_limit;
use std::path::Path;
use std::ptr;

const HERE: &str = ".";

#[derive(Debug)]
struct Entry<'a> {
    file: &'a ModelFile,
    path: &'a str,
    key: Key,
    stored: Vec<Checked>,
    fresh: Vec<Checked>,
}

fn is_result_of(stored: &Checked, target: &Target<'_>) -> bool {
    stored.is_of(target.readable)
}

fn served(stored: &Checked) -> Checked {
    let mut served = stored.clone();
    served.result.cached = true;
    served.result.milliseconds = 0;
    served
}

fn entries<'a>(
    targets: &[Target<'a>],
    arguments: &Verify,
    bounds: Bounds,
    cache: &Cache,
) -> Vec<Entry<'a>> {
    let rule = arguments.selection.rule.as_deref();
    let mut entries: Vec<Entry<'a>> = Vec::new();
    for target in targets {
        let path = target.file.path.as_str();
        if entries
            .last()
            .is_some_and(|entry| ptr::eq(entry.file, target.file))
        {
            continue;
        }
        let key = Key::new(&cached_as(target.file), &target.file.source, bounds, rule);
        let stored = cache.load(&key);
        entries.push(Entry {
            file: target.file,
            path,
            key,
            stored,
            fresh: Vec::new(),
        });
    }
    entries
}

fn cached_as(file: &ModelFile) -> String {
    match file.quint_check() {
        Some(check) => match file.models().first() {
            Some(readable) => {
                let judged: Vec<&str> = readable.model.rule_ids().collect();
                format!(
                    "{}\n{check}\njudged:{}\nexpected:{}",
                    file.path,
                    judged.join(","),
                    readable.expected.join(",")
                )
            }
            None => format!("{}\n{check}", file.path),
        },
        None => file.path.clone(),
    }
}

fn keep(entry: &Entry<'_>, cache: &Cache) {
    if entry.fresh.is_empty() {
        return;
    }
    let lasting = |checked: &&Checked| {
        !(entry.key.has_time_limit()
            && checked
                .result
                .reason
                .as_deref()
                .is_some_and(stopped_by_time_limit))
    };
    let still_current = |old: &&Checked| !entry.fresh.iter().any(|new| new.is_the_run_of(old));
    let models: Vec<Checked> = entry
        .stored
        .iter()
        .filter(still_current)
        .chain(entry.fresh.iter().filter(lasting))
        .cloned()
        .collect();
    if let Err(error) = cache.store(&entry.key, &models) {
        to_stderr(&format!(
            "warning: the result of {} was not cached: {error}",
            entry.path
        ));
    }
}

fn check_all(
    targets: &[Target<'_>],
    arguments: &Verify,
    bounds: Bounds,
) -> Result<Vec<Checked>, String> {
    let cache = Cache::under(Path::new(HERE));
    let mut entries = entries(targets, arguments, bounds, &cache);
    let entry_of = |entries: &[Entry<'_>], target: &Target<'_>| {
        entries
            .iter()
            .position(|entry| ptr::eq(entry.file, target.file))
    };
    let mut results: Vec<Option<Checked>> = targets
        .iter()
        .map(|target| {
            let entry = entries.get(entry_of(&entries, target)?)?;
            entry
                .stored
                .iter()
                .find(|stored| is_result_of(stored, target))
                .filter(|_| !arguments.no_cache)
                .map(served)
        })
        .collect();
    let missing: Vec<usize> = (0..targets.len())
        .filter(|position| results.get(*position).is_some_and(Option::is_none))
        .collect();
    let rule = arguments.selection.rule.as_deref();
    let searches: Vec<Expecting<'_>> = missing
        .iter()
        .filter_map(|position| targets.get(*position))
        .map(|target| Expecting {
            search: Search {
                model: &target.readable.model,
                only_rule: rule,
            },
            expected: &target.readable.expected,
            stops: Stops::Early,
        })
        .collect();
    let ran = explore_expecting(&searches, bounds, Graphs::Dropped);
    let checked_targets = missing
        .iter()
        .filter_map(|position| Some((*position, targets.get(*position)?)));
    for ((position, target), ran) in checked_targets.zip(&ran) {
        let fresh = checked(target.readable, ran)?;
        if let Some(entry) = entry_of(&entries, target).and_then(|at| entries.get_mut(at)) {
            entry.fresh.push(fresh.clone());
        }
        if let Some(slot) = results.get_mut(position) {
            *slot = Some(fresh);
        }
    }
    for entry in &entries {
        keep(entry, &cache);
    }
    Ok(results.into_iter().flatten().collect())
}

pub(crate) fn verify(arguments: &Verify) -> Result<Exit, String> {
    let bounds = limits_of(&arguments.bounds);
    if let Some(base) = &arguments.base {
        check_base(base)?;
    }
    let Gathered {
        files,
        skipped,
        no_model,
    } = gather(&arguments.selection.models, bounds.jobs)?;
    let model = arguments.selection.models.model.as_deref();
    let rule = arguments.selection.rule.as_deref();
    let selected = select_with_removed(&files, model, rule, arguments.base.as_deref())
        .map_err(|error| refusing(&skipped, &no_model, model, rule, error))?;
    let targets = selected.targets;
    say_what_is_unread(&files, model, rule, &targets);
    let checked = match check_all(&targets, arguments, bounds) {
        Ok(checked) => checked,
        Err(error) => {
            to_stderr(&format!("error: {error}"));
            return Ok(Exit::Failed);
        }
    };
    let compared = arguments.base.as_deref().map(|base| {
        let compare = Compare {
            base,
            roots: &arguments.selection.models.paths,
            files: &files,
            model: arguments.selection.models.model.as_deref(),
            rule: arguments.selection.rule.as_deref(),
            bounds,
        };
        (base, rule_changes(&compare, &targets))
    });
    if let Some((base, compared)) = &compared {
        to_stderr(&compared.warnings(base).join("\n"));
    }
    if let Some(error) = selected.unmatched
        && compared
            .as_ref()
            .is_none_or(|(_, compared)| compared.lists_nothing())
    {
        return Err(refusing(&skipped, &no_model, model, rule, error));
    }
    let verified = Verified {
        files: &files,
        skipped: &skipped,
        no_model: &no_model,
        checked: &checked,
        since: compared.as_ref().map(|(base, compared)| Since {
            base,
            changes: &compared.changes,
            not_compared: &compared.not_compared,
        }),
    };
    if arguments.json {
        match verified.json() {
            Ok(json) => to_stdout(&json),
            Err(error) => {
                to_stderr(&format!("error: {error}"));
                return Ok(Exit::Failed);
            }
        }
    } else {
        to_stdout(&verified.text());
    }
    Ok(if verified.summary().everything_passed() {
        Exit::Passed
    } else {
        Exit::Failed
    })
}
