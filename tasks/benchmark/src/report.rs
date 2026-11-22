//! Medians, rates and the Markdown the benchmark prints.

use std::time::Duration;

const KIB_PER_MIB: f64 = 1024.0;
pub(crate) const TOGETHER_TARGET_SECONDS: u32 = 5;
pub(crate) const SLOWEST_TARGET_SECONDS: u32 = 10;
pub(crate) const LARGE_TARGET_SECONDS: u32 = 5;
pub(crate) const TARGET_WORKERS: usize = 4;
pub(crate) const LARGEST_TARGET_WORKERS_BUDGET_SECONDS: f64 = 3.0;
pub(crate) const LARGEST_ONE_WORKER_BUDGET_SECONDS: f64 = 6.0;
pub(crate) const LARGEST_PEAK_MEMORY_BUDGET_MIB: f64 = 256.0;
pub(crate) const NARROW_LEVELS_ALL_CORES_OVER_ONE_WORKER_BUDGET: f64 = 1.25;
pub(crate) const WIDE_ALL_CORES_OVER_ONE_WORKER_BUDGET: f64 = 0.6;
const PERCENT: f64 = 100.0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub(crate) model: String,
    pub(crate) language: String,
    pub(crate) states: u64,
    pub(crate) median: Duration,
    pub(crate) peak_kib: Option<u64>,
}

pub(crate) fn median(times: &[Duration]) -> Duration {
    let mut sorted = times.to_vec();
    sorted.sort();
    let middle = sorted.len() / 2;
    match sorted.len() {
        0 => Duration::ZERO,
        length if length % 2 == 1 => sorted[middle],
        _ => (sorted[middle - 1] + sorted[middle]) / 2,
    }
}

fn float(number: u64) -> f64 {
    f64::from(u32::try_from(number).unwrap_or(u32::MAX))
}

pub(crate) fn states_per_second(states: u64, wall: Duration) -> u64 {
    let micros = wall.as_micros().max(1);
    u64::try_from(u128::from(states) * 1_000_000 / micros).unwrap_or(u64::MAX)
}

fn seconds(wall: Duration) -> String {
    format!("{:.3}", wall.as_secs_f64())
}

pub(crate) fn with_separators(number: u64) -> String {
    let digits = number.to_string();
    let mut grouped = String::new();
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && (digits.len() - position).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub(crate) fn memory(peak_kib: Option<u64>) -> String {
    peak_kib.map_or_else(
        || "n/a".to_owned(),
        |kib| format!("{:.1}", float(kib) / KIB_PER_MIB),
    )
}

pub(crate) fn first_line(cores: usize, processor: Option<&str>, runs: usize) -> String {
    let machine = processor.map_or_else(|| cores.to_string(), |name| format!("{cores} ({name})"));
    format!(
        "Cores: {machine}. Runs per model: {runs} after one unmeasured run. Wall time includes starting the process and reading the files; peak memory is the largest resident set of any run.\n"
    )
}

pub(crate) fn table(rows: &[Row]) -> String {
    let header = [
        "| Model | Language | States | Median wall time (s) | States per second | Peak memory (MiB) |",
        "|---|---|---:|---:|---:|---:|",
    ]
    .map(str::to_owned);
    let body = rows.iter().map(|row| {
        format!(
            "| {} | {} | {} | {} | {} | {} |",
            row.model,
            row.language,
            with_separators(row.states),
            seconds(row.median),
            with_separators(states_per_second(row.states, row.median)),
            memory(row.peak_kib)
        )
    });
    let mut text = header
        .into_iter()
        .chain(body)
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    text
}

pub(crate) fn spec_lines(together: Duration, slowest: Duration) -> [String; 2] {
    [
        format!(
            "12 models together: {} s, target under {TOGETHER_TARGET_SECONDS} s (a stand-in: these benchmark models, not the first app's 12)",
            seconds(together)
        ),
        format!(
            "slowest model alone: {} s, target under {SLOWEST_TARGET_SECONDS} s (a stand-in: these benchmark models, not the first app's)",
            seconds(slowest)
        ),
    ]
}

pub(crate) fn target_line(
    model: &str,
    median: Duration,
    peak_kib: Option<u64>,
    cores: usize,
) -> String {
    let machine = if cores >= TARGET_WORKERS {
        format!("this machine has {cores} cores, at least the {TARGET_WORKERS} of the target")
    } else {
        format!(
            "a stand-in: this machine has {cores} cores, fewer than the {TARGET_WORKERS} of the target"
        )
    };
    format!(
        "{model} with {TARGET_WORKERS} workers: {} s, peak memory {} MiB, target under {LARGE_TARGET_SECONDS} s on {TARGET_WORKERS} cores and 16 GB ({machine})",
        seconds(median),
        memory(peak_kib)
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Met,
    Missed,
    NotJudged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unit {
    Seconds,
    Mib,
    Times,
}

impl Unit {
    fn shown(self, value: f64) -> String {
        match self {
            Self::Seconds => format!("{value:.3} s"),
            Self::Mib => format!("{value:.1} MiB"),
            Self::Times => format!("{value:.2}x"),
        }
    }

    const fn bound(self) -> &'static str {
        match self {
            Self::Seconds | Self::Mib => "under",
            Self::Times => "at most",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Target,
    Budget,
}

impl Kind {
    const fn label(self) -> &'static str {
        match self {
            Self::Target => "target",
            Self::Budget => "budget",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Judged {
    OnTargetCores,
    Anywhere,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Limit {
    pub(crate) what: String,
    pub(crate) measured: Option<f64>,
    pub(crate) allowed: f64,
    pub(crate) unit: Unit,
    pub(crate) kind: Kind,
    pub(crate) judged: Judged,
}

impl Limit {
    pub(crate) fn verdict(&self, cores: usize) -> Verdict {
        let Some(measured) = self.measured else {
            return Verdict::NotJudged;
        };
        let within = match self.unit {
            Unit::Seconds | Unit::Mib => measured < self.allowed,
            Unit::Times => measured <= self.allowed,
        };
        if self.judged == Judged::OnTargetCores && cores < TARGET_WORKERS {
            Verdict::NotJudged
        } else if within {
            Verdict::Met
        } else {
            Verdict::Missed
        }
    }

    pub(crate) fn line(&self, cores: usize) -> String {
        let measured = self
            .measured
            .map_or_else(|| "n/a".to_owned(), |value| self.unit.shown(value));
        let verdict = match (self.verdict(cores), self.measured) {
            (Verdict::Missed, Some(value)) => {
                format!(
                    "missed, {:.0}% over",
                    (value / self.allowed - 1.0) * PERCENT
                )
            }
            (Verdict::NotJudged, Some(_)) => {
                format!("not judged, fewer than {TARGET_WORKERS} cores")
            }
            (Verdict::NotJudged | Verdict::Missed, None) => "not judged, not measured".to_owned(),
            (Verdict::Met, _) => "met".to_owned(),
        };
        format!(
            "{}: {measured}, {} {} {}: {verdict}",
            self.what,
            self.kind.label(),
            self.unit.bound(),
            self.unit.shown(self.allowed)
        )
    }
}

pub(crate) fn budget_section(limits: &[Limit], cores: usize) -> String {
    let lines: Vec<String> = limits
        .iter()
        .filter(|limit| limit.kind == Kind::Budget)
        .map(|limit| format!("- {}\n", limit.line(cores)))
        .collect();
    format!("## Budget\n\n{}", lines.concat())
}

pub(crate) fn mib(peak_kib: Option<u64>) -> Option<f64> {
    peak_kib.map(|kib| float(kib) / KIB_PER_MIB)
}

pub(crate) fn ratio(over: Duration, under: Duration) -> f64 {
    over.as_secs_f64() / under.as_secs_f64().max(f64::EPSILON)
}

pub(crate) fn speedup_line(model: &str, one: Duration, all: Duration, cores: usize) -> String {
    let speedup = one.as_secs_f64() / all.as_secs_f64().max(f64::EPSILON);
    format!(
        "{model}: 1 worker {} s, {cores} workers (all cores) {} s, speed-up {speedup:.2}x",
        seconds(one),
        seconds(all)
    )
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Alone<'a> {
    pub(crate) model: &'a str,
    pub(crate) median: Duration,
}

pub(crate) fn r12_lines(
    together: Duration,
    models: usize,
    largest: Alone<'_>,
    slowest: Alone<'_>,
) -> [String; 2] {
    let longest = if slowest.median > largest.median {
        slowest
    } else {
        largest
    };
    let ratio = together.as_secs_f64() / longest.median.as_secs_f64().max(f64::EPSILON);
    [
        format!(
            "the {models} models in one run: {} s; the large model ({}) alone: {} s; the slowest of the 12 alone ({}): {} s",
            seconds(together),
            largest.model,
            seconds(largest.median),
            slowest.model,
            seconds(slowest.median)
        ),
        format!(
            "R12 asks for about as long as the largest model alone: the one run took {ratio:.2}x the longest of them alone ({})",
            longest.model
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(count: u64) -> Duration {
        Duration::from_millis(count)
    }

    #[test]
    fn first_line_names_the_processor_when_it_is_known() {
        assert_eq!(
            first_line(4, Some("AMD EPYC 7763 64-Core Processor"), 5),
            "Cores: 4 (AMD EPYC 7763 64-Core Processor). Runs per model: 5 after one unmeasured run. Wall time includes starting the process and reading the files; peak memory is the largest resident set of any run.\n",
            "with a name"
        );
    }

    #[test]
    fn first_line_has_only_the_core_count_when_the_processor_is_not_known() {
        assert_eq!(
            first_line(4, None, 5),
            "Cores: 4. Runs per model: 5 after one unmeasured run. Wall time includes starting the process and reading the files; peak memory is the largest resident set of any run.\n",
            "without a name"
        );
    }

    #[test]
    fn median_of_odd_even_and_no_runs() {
        assert_eq!(
            median(&[millis(30), millis(10), millis(20)]),
            millis(20),
            "odd count"
        );
        assert_eq!(
            median(&[millis(40), millis(10), millis(20), millis(30)]),
            millis(25),
            "even count"
        );
        assert_eq!(median(&[]), Duration::ZERO, "no runs");
    }

    #[test]
    fn rate_is_states_over_seconds() {
        assert_eq!(
            states_per_second(1_000_000, millis(2_000)),
            500_000,
            "a million states in two seconds"
        );
        assert_eq!(
            states_per_second(5, Duration::ZERO),
            5_000_000,
            "zero time does not divide by zero"
        );
    }

    #[test]
    fn thousands_are_separated() {
        assert_eq!(with_separators(0), "0", "zero");
        assert_eq!(with_separators(999), "999", "below a thousand");
        assert_eq!(with_separators(1_000), "1,000", "a thousand");
        assert_eq!(with_separators(1_234_567), "1,234,567", "millions");
    }

    #[test]
    fn table_has_a_header_and_one_line_per_row() {
        let rows = [
            Row {
                model: "Grid1k".to_owned(),
                language: "Rust".to_owned(),
                states: 1_000,
                median: millis(500),
                peak_kib: Some(2_048),
            },
            Row {
                model: "Other".to_owned(),
                language: "TypeScript".to_owned(),
                states: 10,
                median: millis(1_500),
                peak_kib: None,
            },
        ];
        let text = table(&rows);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "header, rule and two rows");
        assert_eq!(
            lines[2], "| Grid1k | Rust | 1,000 | 0.500 | 2,000 | 2.0 |",
            "first row"
        );
        assert_eq!(
            lines[3], "| Other | TypeScript | 10 | 1.500 | 6 | n/a |",
            "second row"
        );
    }

    #[test]
    fn spec_lines_name_the_targets_and_the_stand_in() {
        let [together, slowest] = spec_lines(millis(1_250), millis(2_500));
        assert!(
            together.starts_with("12 models together: 1.250 s, target under 5 s"),
            "{together}"
        );
        assert!(
            slowest.starts_with("slowest model alone: 2.500 s, target under 10 s"),
            "{slowest}"
        );
        assert!(
            together.contains("stand-in") && slowest.contains("stand-in"),
            "both are marked"
        );
    }

    #[test]
    fn target_line_says_whether_the_machine_has_the_cores() {
        let enough = target_line("Big", millis(2_500), Some(2_048), 4);
        assert_eq!(
            enough,
            "Big with 4 workers: 2.500 s, peak memory 2.0 MiB, target under 5 s on 4 cores and 16 GB (this machine has 4 cores, at least the 4 of the target)",
            "a machine with the cores"
        );
        let fewer = target_line("Big", millis(2_500), None, 2);
        assert!(
            fewer.contains("peak memory n/a")
                && fewer.contains("a stand-in: this machine has 2 cores, fewer than the 4"),
            "{fewer}"
        );
    }

    fn limit(measured: Option<f64>, allowed: f64, unit: Unit, judged: Judged) -> Limit {
        Limit {
            what: "Big with 4 workers, wall time".to_owned(),
            measured,
            allowed,
            unit,
            kind: Kind::Budget,
            judged,
        }
    }

    #[test]
    fn a_time_is_met_only_under_its_limit_and_judged_only_on_the_target_cores() {
        assert_eq!(
            limit(Some(4.999), 5.0, Unit::Seconds, Judged::OnTargetCores).verdict(4),
            Verdict::Met,
            "under the limit"
        );
        assert_eq!(
            limit(Some(5.0), 5.0, Unit::Seconds, Judged::OnTargetCores).verdict(16),
            Verdict::Missed,
            "the limit itself is not under the limit"
        );
        assert_eq!(
            limit(Some(60.0), 5.0, Unit::Seconds, Judged::OnTargetCores).verdict(2),
            Verdict::NotJudged,
            "a slow run on two cores says nothing about four"
        );
        assert_eq!(
            limit(Some(300.0), 256.0, Unit::Mib, Judged::Anywhere).verdict(2),
            Verdict::Missed,
            "memory is judged on any machine"
        );
        assert_eq!(
            limit(None, 256.0, Unit::Mib, Judged::Anywhere).verdict(4),
            Verdict::NotJudged,
            "no peak memory to judge"
        );
    }

    #[test]
    fn a_ratio_may_equal_its_limit() {
        assert_eq!(
            limit(Some(1.25), 1.25, Unit::Times, Judged::OnTargetCores).verdict(4),
            Verdict::Met,
            "at most the limit"
        );
        assert_eq!(
            limit(Some(1.26), 1.25, Unit::Times, Judged::OnTargetCores).verdict(4),
            Verdict::Missed,
            "above the limit"
        );
    }

    #[test]
    fn a_limit_line_names_the_run_the_measurement_the_limit_and_how_far_over() {
        assert_eq!(
            limit(Some(3.6), 3.0, Unit::Seconds, Judged::OnTargetCores).line(4),
            "Big with 4 workers, wall time: 3.600 s, budget under 3.000 s: missed, 20% over",
            "a miss"
        );
        assert_eq!(
            limit(Some(0.33), 0.6, Unit::Times, Judged::OnTargetCores).line(4),
            "Big with 4 workers, wall time: 0.33x, budget at most 0.60x: met",
            "a ratio that is met"
        );
        assert_eq!(
            limit(Some(3.6), 3.0, Unit::Seconds, Judged::OnTargetCores).line(2),
            "Big with 4 workers, wall time: 3.600 s, budget under 3.000 s: not judged, fewer than 4 cores",
            "too few cores"
        );
        assert_eq!(
            limit(None, 256.0, Unit::Mib, Judged::Anywhere).line(4),
            "Big with 4 workers, wall time: n/a, budget under 256.0 MiB: not judged, not measured",
            "no peak memory"
        );
    }

    #[test]
    fn a_ratio_is_the_first_time_over_the_second() {
        assert!(
            (ratio(millis(500), millis(1_000)) - 0.5).abs() < f64::EPSILON,
            "half"
        );
        assert_eq!(mib(Some(2_048)), Some(2.0), "2,048 KiB");
        assert_eq!(mib(None), None, "no peak");
    }

    #[test]
    fn speedup_is_one_worker_over_all_cores() {
        assert_eq!(
            speedup_line("Big", millis(8_000), millis(2_000), 4),
            "Big: 1 worker 8.000 s, 4 workers (all cores) 2.000 s, speed-up 4.00x",
            "four times faster"
        );
    }

    #[test]
    fn r12_compares_the_one_run_with_the_longest_model_alone() {
        let [first, second] = r12_lines(
            millis(3_000),
            13,
            Alone {
                model: "Big",
                median: millis(1_000),
            },
            Alone {
                model: "Slow",
                median: millis(2_000),
            },
        );
        assert!(
            first.starts_with(
                "the 13 models in one run: 3.000 s; the large model (Big) alone: 1.000 s"
            ),
            "{first}"
        );
        assert!(
            second.contains("1.50x") && second.contains("(Slow)"),
            "the longer of the two is the yardstick: {second}"
        );
    }
}
