use crate::reading::Readable;
use crate::workers::run_each;
use earnest_chart::{chart_outline, chart_with_limit, detail_lines};
use earnest_check::{Limits, Outcome, explore};
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

pub(crate) fn explore_each(searches: &[Search<'_>], bounds: Bounds, graphs: Graphs) -> Vec<Ran> {
    run_each(searches, bounds.jobs, |search, helpers| {
        let limits = Limits {
            max_states: bounds.max_states,
            time_limit: bounds.time_limit,
            jobs: 1,
            helpers: Some(helpers.clone()),
            keep_graph: graphs != Graphs::Dropped,
            outline: graphs != Graphs::Dropped,
            only_rule: search.only_rule.map(str::to_owned),
        };
        let began = Instant::now();
        let mut outcome = explore(search.model, &limits);
        let elapsed = began.elapsed();
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
        },
        detail: detail_lines(&ran.outcome),
        position: readable.position,
    })
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
