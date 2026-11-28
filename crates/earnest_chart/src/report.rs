use crate::changes::{NotCompared, RuleChanges, rule_change_lines, rule_change_lines_noting};
use crate::markdown::{code_span, fenced, one_line, prose};
use crate::mermaid::chart_with_limit;
use crate::select::ARROW_LIMIT;
use crate::text::{detail_lines, word};
use earnest_check::{Outcome, Status};

/// What the report says about one model.
#[derive(Clone, Copy, Debug)]
pub struct Section<'a> {
    /// The model's name, for the heading and the result line.
    pub name: &'a str,
    /// The path of the model's file as the report should show it.
    pub file: &'a str,
    /// What the search found. The section has a chart when its `graph` is present.
    pub outcome: &'a Outcome,
}

/// A model file that could not be read, so none of its models is in the report.
#[derive(Clone, Copy, Debug)]
pub struct Unreadable<'a> {
    /// The path of the file as the report should show it.
    pub file: &'a str,
    /// One line for each problem, as the text output of the CLI prints it:
    /// `file:line:column`, the feature and the sentence.
    pub problems: &'a [String],
}

fn result_line(detailed: &Detailed<'_>) -> String {
    let section = &detailed.charted.section;
    let outcome = section.outcome;
    let status = if detailed.read_in_part && outcome.status != Status::Fail {
        UNREADABLE.to_owned()
    } else {
        word(&outcome.status)
    };
    let so_far = if outcome.status == Status::Unfinished {
        " so far"
    } else {
        ""
    };
    let mut parts = vec![
        format!("**{status}**"),
        prose(section.name),
        format!("{} states{so_far}", outcome.states),
    ];
    parts.extend(outcome.reason.as_deref().map(prose));
    parts.push(code_span(section.file));
    parts.join(" · ")
}

const UNREADABLE: &str = "unreadable";

fn unread_lines(file: &Unreadable<'_>) -> Vec<String> {
    let mut lines = vec![format!("{UNREADABLE}  {}", one_line(file.file))];
    lines.extend(
        file.problems
            .iter()
            .map(|problem| format!("    {}", one_line(problem))),
    );
    lines
}

/// The `Skipped files` section for a report: the test files that use the library, which the
/// checker did not read and which did not change the verdict. `None` when there is none.
///
/// Append it to the text of [`report`] after a blank line, so a reader of the report sees that
/// a file was passed over on purpose, and which.
#[must_use]
pub fn skipped_section(skipped: &[String]) -> Option<String> {
    if skipped.is_empty() {
        return None;
    }
    let (noun, verb) = if skipped.len() == 1 {
        ("test file that uses", "was")
    } else {
        ("test files that use", "were")
    };
    let lines: Vec<String> = skipped.iter().map(|path| one_line(path)).collect();
    Some(format!(
        "## Skipped files\n\n{} {noun} the library {verb} not read:\n\n{}",
        skipped.len(),
        fenced(&lines)
    ))
}

/// A model's section with its chart drawn already, for [`report_with_charts`].
#[derive(Clone, Copy, Debug)]
pub struct Charted<'a> {
    /// What the report says about the model.
    pub section: Section<'a>,
    /// The model's chart, as [`chart_with_limit`](crate::chart_with_limit) or
    /// [`chart_outline`](crate::chart_outline) draws it. The section has no chart without it.
    pub chart: Option<&'a str>,
}

/// A model's section with the lines of its fenced block written already, for
/// [`report_with_details`].
#[derive(Clone, Copy, Debug)]
pub struct Detailed<'a> {
    /// The section and its chart.
    pub charted: Charted<'a>,
    /// The lines of the fenced block, in place of the path to each broken rule and each
    /// finding of the outcome. `None` prints those.
    pub detail: Option<&'a [String]>,
    /// Whether the model was read only in part, so that some of what it asks was not judged.
    /// Its result line then says `unreadable`, unless the outcome failed, so the section never
    /// reads as a pass. Say in `detail` what was left out.
    pub read_in_part: bool,
}

fn model(detailed: &Detailed<'_>) -> String {
    let section = &detailed.charted.section;
    let mut blocks = vec![format!("## {}", prose(section.name)), result_line(detailed)];
    blocks.extend(detailed.charted.chart.map(str::to_owned));
    let detail = detailed
        .detail
        .map_or_else(|| detail_lines(section.outcome), <[String]>::to_vec);
    if !detail.is_empty() {
        blocks.push(fenced(&detail));
    }
    blocks.join("\n\n")
}

/// Markdown for a PR body, with no trailing line break: one section per model, in the order
/// given, then an `Unreadable files` section when `unreadable` is not empty, then a `Rule
/// changes` section when `changes` is present.
///
/// The `Unreadable files` section names each file the checker could not read and why, so a
/// reader of the report can tell a file that was checked from one that was left unchecked.
///
/// A model's section is a heading with its name, a result line with the status, the state
/// count and the file, the chart, and a fenced block with the path to each broken rule and
/// each finding. The chart is left out when the outcome holds no graph, and the fenced block
/// when nothing is wrong. Pass `changes` only when the user asked for a comparison: with no
/// changed rule the section still appears and says `none`.
#[must_use]
pub fn report(
    sections: &[Section<'_>],
    unreadable: &[Unreadable<'_>],
    changes: Option<&RuleChanges<'_>>,
) -> String {
    report_with_limit(sections, unreadable, changes, ARROW_LIMIT)
}

/// [`report`] with charts of at most `max_arrows` arrows instead of
/// [`ARROW_LIMIT`](crate::ARROW_LIMIT).
#[must_use]
pub fn report_with_limit(
    sections: &[Section<'_>],
    unreadable: &[Unreadable<'_>],
    changes: Option<&RuleChanges<'_>>,
    max_arrows: usize,
) -> String {
    let charts: Vec<Option<String>> = sections
        .iter()
        .map(|section| {
            section
                .outcome
                .graph
                .as_ref()
                .map(|graph| chart_with_limit(graph, max_arrows))
        })
        .collect();
    let charted: Vec<Charted<'_>> = sections
        .iter()
        .zip(&charts)
        .map(|(section, chart)| Charted {
            section: *section,
            chart: chart.as_deref(),
        })
        .collect();
    report_with_charts(&charted, unreadable, changes)
}

/// [`report`] with each model's chart drawn by the caller, as one that kept an
/// [`Outline`](earnest_check::Outline) in place of the graph draws it with
/// [`chart_outline`](crate::chart_outline). The graph of an outcome is not read.
#[must_use]
pub fn report_with_charts(
    sections: &[Charted<'_>],
    unreadable: &[Unreadable<'_>],
    changes: Option<&RuleChanges<'_>>,
) -> String {
    let detailed: Vec<Detailed<'_>> = sections
        .iter()
        .map(|charted| Detailed {
            charted: *charted,
            detail: None,
            read_in_part: false,
        })
        .collect();
    report_with_details(&detailed, unreadable, changes)
}

/// [`report_with_charts`] with the fenced block of some models written by the caller, as one
/// that knows which broken rules were expected to break words them.
#[must_use]
pub fn report_with_details(
    sections: &[Detailed<'_>],
    unreadable: &[Unreadable<'_>],
    changes: Option<&RuleChanges<'_>>,
) -> String {
    let mut blocks: Vec<String> = sections.iter().map(model).collect();
    if !unreadable.is_empty() {
        let lines: Vec<String> = unreadable.iter().flat_map(unread_lines).collect();
        blocks.push(format!("## Unreadable files\n\n{}", fenced(&lines)));
    }
    if let Some(changes) = changes {
        blocks.push(format!(
            "## Rule changes\n\n{}",
            fenced(&rule_change_lines(changes))
        ));
    }
    blocks.join("\n\n")
}

/// The `## Rule changes` section that [`report_with_charts`] ends with, with the lines of
/// [`rule_change_lines_noting`](crate::rule_change_lines_noting), for a caller that passes no
/// changes to [`report_with_charts`] and appends this section after its output.
#[must_use]
pub fn rule_changes_section(changes: &RuleChanges<'_>, not_compared: &[NotCompared<'_>]) -> String {
    format!(
        "## Rule changes\n\n{}",
        fenced(&rule_change_lines_noting(changes, not_compared))
    )
}
