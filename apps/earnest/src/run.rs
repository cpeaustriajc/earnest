use crate::reading::Readable;
use crate::workers::run_each;
use earnest_chart::{chart_outline, chart_with_limit, detail_lines, one_line};
use earnest_check::{EVERY_EXPECTED_FOUND, Limits, Outcome, Status as Judged, Verdict, explore};
use earnest_ir::Model;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::num::NonZeroUsize;
use std::thread::available_parallelism;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub(crate) max_states: usize,
    pub(crate) time_limit: Option<Duration>,
    pub(crate) jobs: usize,
}

impl Bounds {
    pub(crate) fn new(
        max_states: usize,
        time_limit_seconds: Option<u64>,
        jobs: Option<NonZeroUsize>,
    ) -> Self {
        Self {
            max_states,
            time_limit: time_limit_seconds.map(Duration::from_secs),
            jobs: jobs
                .or_else(|| available_parallelism().ok())
                .map_or(1, NonZeroUsize::get),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Graphs {
    Charted { max_arrows: usize },
    Dropped,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Search<'a> {
    pub(crate) model: &'a Model,
    pub(crate) only_rule: Option<&'a str>,
}

#[derive(Clone, Debug)]
pub(crate) struct Ran {
    pub(crate) outcome: Outcome,
    pub(crate) elapsed: Duration,
    pub(crate) chart: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stops {
    Early,
    AtTheEnd,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Expecting<'a> {
    pub(crate) search: Search<'a>,
    pub(crate) expected: &'a [String],
    pub(crate) stops: Stops,
}

#[cfg(test)]
pub(crate) fn explore_each(searches: &[Search<'_>], bounds: Bounds, graphs: Graphs) -> Vec<Ran> {
    let expecting: Vec<Expecting<'_>> = searches
        .iter()
        .map(|search| Expecting {
            search: *search,
            expected: &[],
            stops: Stops::AtTheEnd,
        })
        .collect();
    explore_expecting(&expecting, bounds, graphs)
}

pub(crate) fn whole_of(id: &str) -> &str {
    match id.rsplit_once('#') {
        Some((before, number))
            if !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            before
        }
        _ => id,
    }
}

fn base_id(rule: &earnest_check::RuleOutcome) -> &str {
    whole_of(&rule.id)
}

fn is_expected(expected: &[String], rule: &earnest_check::RuleOutcome) -> bool {
    expected.iter().any(|name| name == base_id(rule))
}

fn stop_once_broken(search: &Search<'_>, expected: &[String]) -> Vec<String> {
    let wanted = |id: &str| search.only_rule.is_none_or(|only| only == id);
    let model = search.model;
    let eventually_wanted = model
        .runs
        .eventually
        .iter()
        .any(|rule| wanted(&rule.id) || wanted(whole_of(&rule.id)));
    let always: Vec<String> = model
        .rules
        .iter()
        .filter(|rule| wanted(&rule.id))
        .map(|rule| rule.id.clone())
        .collect();
    if eventually_wanted
        || always.is_empty()
        || !always
            .iter()
            .all(|id| expected.iter().any(|name| name == whole_of(id)))
    {
        return Vec::new();
    }
    always
}

fn group_verdict(expected: &str, outcome: &Outcome) -> Option<Verdict> {
    let parts: Vec<Verdict> = outcome
        .rules
        .iter()
        .filter(|rule| base_id(rule) == expected)
        .map(|rule| rule.verdict)
        .collect();
    if parts.is_empty() {
        None
    } else if parts.contains(&Verdict::Broken) {
        Some(Verdict::Broken)
    } else if parts.iter().all(|verdict| *verdict == Verdict::Holds) {
        Some(Verdict::Holds)
    } else {
        Some(Verdict::Unknown)
    }
}

fn judge(outcome: &mut Outcome, expected: &[String]) {
    if expected.is_empty() {
        return;
    }
    let groups: Vec<Verdict> = expected
        .iter()
        .filter_map(|name| group_verdict(name, outcome))
        .collect();
    let ordinary = outcome
        .rules
        .iter()
        .filter(|rule| !is_expected(expected, rule));
    let mut ordinary_broken = false;
    let mut ordinary_unknown = false;
    for rule in ordinary {
        ordinary_broken |= rule.verdict == Verdict::Broken;
        ordinary_unknown |= rule.verdict == Verdict::Unknown;
    }
    let stopped_early = outcome.reason.as_deref() == Some(EVERY_EXPECTED_FOUND);
    let failed =
        !outcome.findings.is_empty() || ordinary_broken || groups.contains(&Verdict::Holds);
    let unanswered = ordinary_unknown
        || groups.contains(&Verdict::Unknown)
        || (outcome.reason.is_some() && !stopped_early);
    outcome.status = if failed {
        Judged::Fail
    } else if unanswered {
        Judged::Unfinished
    } else {
        Judged::Pass
    };
}

pub(crate) fn explore_expecting(
    searches: &[Expecting<'_>],
    bounds: Bounds,
    graphs: Graphs,
) -> Vec<Ran> {
    run_each(searches, bounds.jobs, |expecting, helpers| {
        let search = &expecting.search;
        let limits = Limits {
            max_states: bounds.max_states,
            time_limit: bounds.time_limit,
            jobs: 1,
            helpers: Some(helpers.clone()),
            keep_graph: graphs != Graphs::Dropped,
            outline: graphs != Graphs::Dropped,
            only_rule: search.only_rule.map(str::to_owned),
            stop_once_broken: if expecting.stops == Stops::Early {
                stop_once_broken(search, expecting.expected)
            } else {
                Vec::new()
            },
        };
        let began = Instant::now();
        let mut outcome = explore(search.model, &limits);
        let elapsed = began.elapsed();
        judge(&mut outcome, expecting.expected);
        let chart = match graphs {
            Graphs::Charted { max_arrows } => outcome
                .outline
                .take()
                .map(|outline| chart_outline(&outline, max_arrows))
                .or_else(|| {
                    outcome
                        .graph
                        .take()
                        .map(|graph| chart_with_limit(&graph, max_arrows))
                }),
            Graphs::Dropped => None,
        };
        Ran {
            outcome,
            elapsed,
            chart,
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Status {
    Pass,
    Fail,
    Unfinished,
}

impl Status {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unfinished => "unfinished",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NamedValue {
    pub(crate) name: String,
    pub(crate) value: Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FieldChange {
    pub(crate) field: String,
    pub(crate) before: Json,
    pub(crate) after: Json,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathStep {
    pub(crate) action: String,
    pub(crate) label: String,
    pub(crate) input: Vec<NamedValue>,
    pub(crate) changes: Vec<FieldChange>,
    pub(crate) state: Vec<NamedValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PathTaken {
    pub(crate) start: Vec<NamedValue>,
    pub(crate) steps: Vec<PathStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RuleResult {
    pub(crate) id: String,
    pub(crate) says: String,
    pub(crate) verdict: String,
    pub(crate) path: Option<PathTaken>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(
        default,
        rename = "repeatsFrom",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) repeats_from: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FindingResult {
    pub(crate) kind: String,
    pub(crate) message: String,
    pub(crate) path: PathTaken,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct Searched {
    status: Status,
    states: usize,
    rules: Vec<RuleResult>,
    findings: Vec<FindingResult>,
    reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SetupName {
    pub(crate) id: String,
    pub(crate) says: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ModelResult {
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) setup: Option<SetupName>,
    pub(crate) file: String,
    pub(crate) line: u32,
    pub(crate) status: Status,
    pub(crate) states: usize,
    pub(crate) milliseconds: u64,
    pub(crate) cached: bool,
    pub(crate) reason: Option<String>,
    pub(crate) rules: Vec<RuleResult>,
    pub(crate) findings: Vec<FindingResult>,
    #[serde(
        rename = "runsNotPlayed",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub(crate) runs_not_played: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Checked {
    pub(crate) result: ModelResult,
    pub(crate) detail: Vec<String>,
    #[serde(default)]
    pub(crate) position: usize,
}

impl ModelResult {
    pub(crate) fn run_name(&self) -> String {
        match &self.setup {
            Some(setup) => format!("{} ({})", self.name, setup.id),
            None => self.name.clone(),
        }
    }
}

impl Checked {
    pub(crate) fn is_of(&self, readable: &Readable) -> bool {
        let model = &readable.model;
        let setup = self.result.setup.as_ref().map(|setup| setup.id.as_str());
        self.position == readable.position
            && self.result.name == model.name
            && self.result.line == model.line
            && setup == model.setup().map(|setup| setup.id.as_str())
    }

    pub(crate) fn is_the_run_of(&self, other: &Self) -> bool {
        let id = |checked: &Self| checked.result.setup.as_ref().map(|setup| setup.id.clone());
        self.position == other.position
            && self.result.name == other.result.name
            && self.result.line == other.result.line
            && id(self) == id(other)
    }
}

pub(crate) fn milliseconds(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

pub(crate) fn checked(readable: &Readable, ran: &Ran) -> Result<Checked, String> {
    let model = &readable.model;
    let searched: Searched = serde_json::to_value(&ran.outcome)
        .and_then(serde_json::from_value)
        .map_err(|error| {
            format!(
                "the result of the model `{}` can't be reported: {error}",
                model.name
            )
        })?;
    Ok(Checked {
        result: ModelResult {
            name: model.name.clone(),
            setup: model.setup().map(|setup| SetupName {
                id: setup.id.clone(),
                says: setup.says.clone(),
            }),
            file: model.file.clone(),
            line: model.line,
            status: searched.status,
            states: searched.states,
            milliseconds: milliseconds(ran.elapsed),
            cached: false,
            reason: searched.reason,
            rules: searched.rules,
            findings: searched.findings,
            runs_not_played: readable.runs_not_played.clone(),
        },
        detail: detail_of(&ran.outcome, &readable.expected),
        position: readable.position,
    })
}

pub(crate) fn is_expected_result(expected: &[String], rule: &RuleResult) -> bool {
    expected.iter().any(|name| name == whole_of(&rule.id))
}

fn detail_of(outcome: &Outcome, expected: &[String]) -> Vec<String> {
    if expected.is_empty() {
        return detail_lines(outcome);
    }
    let mut lines: Vec<String> = Vec::new();
    for name in expected {
        match group_verdict(name, outcome) {
            Some(Verdict::Broken) => lines.push(format!("broken  {}  (expected)", one_line(name))),
            Some(Verdict::Holds) => lines.push(format!(
                "holds  {}  a counterexample was expected",
                one_line(name)
            )),
            _ => {}
        }
    }
    let mut rest = outcome.clone();
    for rule in &mut rest.rules {
        if is_expected(expected, rule) {
            rule.path = None;
        }
    }
    lines.extend(detail_lines(&rest));
    lines
}

#[cfg(test)]
mod tests {
    use super::{Bounds, Graphs, Search, checked, explore_each};
    use crate::discovery::{Found, Language};
    use crate::reading::load;

    const TWINS: &str = "import { m } from \"@earnestjs/model\";\n\
        export const A = m.model(\"Jar\", { n: m.int(0, 2).initial(0) }); \
        export const B = m.model(\"Jar\", { k: m.int(0, 1).initial(0) });\n";

    #[test]
    fn a_result_is_of_its_own_model_when_two_of_one_name_share_a_line() {
        let file = load(Found::model("twins.model.ts", Language::TypeScript, TWINS));
        let [first, second] = file.models() else {
            panic!("the file has two models: {:?}", file.problems());
        };
        let searches = [first, second].map(|readable| Search {
            model: &readable.model,
            only_rule: None,
        });
        let bounds = Bounds::new(1_000, None, None);
        let ran = explore_each(&searches, bounds, Graphs::Dropped);
        let results: Vec<_> = [first, second]
            .iter()
            .zip(&ran)
            .map(|(readable, ran)| checked(readable, ran).expect("a result can be reported"))
            .collect();
        let [of_first, of_second] = results.as_slice() else {
            panic!("one result per model");
        };
        assert!(
            of_first.is_of(first) && !of_first.is_of(second),
            "the first result is of the first model only"
        );
        assert!(
            of_second.is_of(second) && !of_second.is_of(first),
            "the second result is of the second model only"
        );
        assert!(
            !of_first.is_the_run_of(of_second),
            "a fresh result of one never replaces the cached result of the other"
        );
    }
}
