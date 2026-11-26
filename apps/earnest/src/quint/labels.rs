use super::{Job, TOP_OF_FILE, compile};
use crate::checks::Check;
use crate::plan::Labels;
use earnest_diagnostics::Problem;
use earnest_quint::{Labelled, Located, Views, split_views};

pub(super) fn labelled(job: &Job) -> Check {
    let mut check = job.check.clone();
    check
        .invariants
        .extend(job.labels.names().map(str::to_owned));
    check
}

fn names(job: &Job) -> String {
    let named: Vec<String> = job.labels.names().map(|name| format!("`{name}`")).collect();
    named.join(" and ")
}

fn at_label(job: &Job, message: &str) -> Problem {
    let at = job.labels.state.as_ref().or(job.labels.group.as_ref());
    match at {
        Some(mark) => Problem {
            message: message.to_owned(),
            ..mark.at.clone()
        },
        None => Problem::new(&job.shown, TOP_OF_FILE, "the chart labels", message),
    }
}

pub(super) fn without_labels(job: &Job, json: &str) -> Result<(String, Views), Vec<Located>> {
    let labelled = Labelled {
        state: job.labels.state.as_ref().map(|each| each.name.clone()),
        group: job.labels.group.as_ref().map(|each| each.name.clone()),
    };
    split_views(json, &labelled).ok_or_else(|| {
        let message = format!(
            "the module compiled for the check `{}` does not end its invariants with the chart \
             label {}, which it was compiled with, so the label can't be told from the \
             invariants and the check is not judged: compile it again with `earnest verify \
             --no-cache`, or remove the `@view` mark",
            job.check.main,
            names(job)
        );
        vec![Located::new(at_label(job, &message), None, None)]
    })
}

pub(super) fn label_unbuilt(job: &Job) -> Problem {
    at_label(
        job,
        &format!(
            "the check `{}` is built from what `quint typecheck` wrote, and it can't be built \
             with the chart label {}: the label would make earnest compile the check instead, \
             which changes the runs it plays. Write the label as a `val` that reads what the \
             check's own definitions read",
            job.check.main,
            names(job)
        ),
    )
}

fn unlabelled(job: &Job) -> Job {
    Job {
        shown: job.shown.clone(),
        real: job.real.clone(),
        check: job.check.clone(),
        expected: job.expected.clone(),
        reachable: job.reachable.clone(),
        said: job.said.clone(),
        read: job.read,
        quint: job.quint.clone(),
        labels: Labels::default(),
    }
}

fn in_label(job: &Job, problem: Problem) -> Problem {
    let inside = [job.labels.state.as_ref(), job.labels.group.as_ref()]
        .into_iter()
        .flatten()
        .find(|label| {
            label.written.as_ref().is_some_and(|(file, first, last)| {
                *file == problem.file && (*first..=*last).contains(&problem.line)
            })
        });
    match inside {
        Some(label) => Problem {
            message: format!(
                "`quint compile --main {}` fails in the chart label `{}`: {}",
                job.check.main, label.name, problem.message
            ),
            ..problem
        },
        None => problem,
    }
}

pub(super) fn blamed(
    checks: &[Job],
    compiled: Vec<Result<String, Problem>>,
) -> Vec<Result<String, Problem>> {
    checks
        .iter()
        .zip(compiled)
        .map(|(job, compiled)| match compiled {
            Err(problem) if job.labels.names().next().is_some() => {
                match compile(&unlabelled(job)) {
                    Ok(_) => Err(at_label(
                        job,
                        &format!(
                            "`quint compile --main {}` fails when the chart label {} follows the \
                             invariants, and compiles the check without it, so the label is \
                             not one Quint takes there. Quint said: {}",
                            job.check.main,
                            names(job),
                            problem.message
                        ),
                    )),
                    Err(_) => Err(in_label(job, problem)),
                }
            }
            other => other,
        })
        .collect()
}

pub(super) fn at_marks(job: &Job, views: &Views, found: Vec<Located>) -> Vec<Located> {
    let marks = [
        (views.state.as_deref(), job.labels.state.as_ref()),
        (views.group.as_deref(), job.labels.group.as_ref()),
    ];
    found
        .into_iter()
        .map(|each| {
            let mark = marks.iter().find_map(|(compiled, mark)| {
                (compiled.is_some() && *compiled == each.definition.as_deref())
                    .then_some(*mark)
                    .flatten()
            });
            match mark {
                Some(mark) => Located::new(
                    Problem {
                        message: each.problem.message,
                        ..mark.at.clone()
                    },
                    None,
                    None,
                ),
                None => each,
            }
        })
        .collect()
}
