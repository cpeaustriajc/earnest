use super::{Job, TOP_OF_FILE};
use crate::checks::Check;
use earnest_diagnostics::Problem;
use earnest_quint::{Labelled, Located, Views, split_views};

pub(super) fn labelled(job: &Job) -> Check {
    let mut check = job.check.clone();
    check
        .invariants
        .extend(job.labels.names().map(str::to_owned));
    check
}

pub(super) fn without_labels(job: &Job, json: &str) -> Result<(String, Views), Vec<Located>> {
    let labelled = Labelled {
        state: job.labels.state.is_some(),
        group: job.labels.group.is_some(),
    };
    split_views(json, labelled).ok_or_else(|| {
        vec![Located::new(
            Problem::new(
                &job.shown,
                TOP_OF_FILE,
                "the chart labels",
                "the compiled module does not end its invariants with the chart labels it was \
                 compiled with, so they can't be told from the invariants",
            ),
            None,
            None,
        )]
    })
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
