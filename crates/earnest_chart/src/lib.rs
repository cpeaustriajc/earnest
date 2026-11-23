//! The Mermaid chart and Markdown report writer.
//!
//! [`chart`] draws a [`Graph`](earnest_check::Graph) as one fenced `mermaid` block, and
//! [`chart_outline`] draws the same chart from an [`Outline`](earnest_check::Outline), and
//! [`report`] writes the Markdown an agent pastes into a PR body: one section per model and,
//! when asked, the rule changes. [`path_lines`], [`detail_lines`] and [`rule_change_lines`] give
//! the same paths and rule changes as plain lines, for the text output of the CLI.
//!
//! Nothing here reads a file, runs a search or calls git: the caller passes what it found.

mod changes;
mod markdown;
mod mermaid;
mod outline;
mod plot;
mod report;
mod select;
mod text;

pub use changes::{ChangeKind, RuleChange, RuleChanges, RuleText, rule_change_lines};
pub use markdown::one_line;
pub use mermaid::{chart, chart_outline, chart_with_limit};
pub use report::{
    Charted, Section, Unreadable, report, report_with_charts, report_with_limit, skipped_section,
};
pub use select::ARROW_LIMIT;
pub use text::{detail_lines, path_lines};
