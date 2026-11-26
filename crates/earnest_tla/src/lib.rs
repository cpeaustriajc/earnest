//! The TLA+ writer: a checked model as a TLA+ module and the configuration TLC checks it with.
//!
//! [`write`] gives the text of both files. The module is written so that TLC finds the states
//! `earnest_check` finds and judges each rule the same way:
//!
//! - `Init` is every combination of the fields' start values, and `Next` is one disjunct per
//!   action: for each combination of its input values the guard, then every variable primed.
//! - `InRange` says each variable holds a value its field declares. No action can happen
//!   outside it, so a state out of range is reached and counted and nothing is searched from
//!   it. Each rule is the invariant `InRange => rule`, because the search does not judge a rule
//!   in a state out of range.
//! - `Stop` lets a state out of range or a marked end repeat itself. It adds no state, and with
//!   it the states TLC would call a deadlock are the ones the search calls stuck, apart from a
//!   state where a guard or an update faults. The configuration still turns the deadlock check
//!   off, because TLC stops at the first deadlock and would count fewer states than the search
//!   does for a model that is stuck.
//! - An optional value is the empty tuple when absent and a tuple of one value when present,
//!   so TLC never compares values of two kinds.
//! - A guard or an update that has no value in a state is a fault in the search, which then
//!   takes no step with that action and those inputs. The action's `IF` therefore first asks
//!   that every value read is present and every remainder has a value, and takes no step
//!   otherwise. A rule with no value in a state is left to stop TLC with an error, which is
//!   what the verdict `unknown` means.
//! - A model with a rule about runs also gets `Spec`: `Init`, `[][Next]_vars`, weak fairness of
//!   `Next`, so a run never stops while an action can still change the state, and weak fairness
//!   of each action the model declares fair. Each such rule is a temporal property: `<>` of
//!   what it reaches, or `~>` from where it starts to what it reaches. A condition that reads
//!   no variable is written with `vars = vars` beside it, because TLC misjudges a temporal
//!   property over a constant.

mod faulting;
mod helper;
mod kind;
mod limits;
mod names;
mod scribe;

use earnest_ir::{Action, Expr, Model};
use earnest_value::Value;
use helper::{EVERY_HELPER, Helper};
use kind::Kind;
pub use limits::BEYOND_32_BITS_COMPUTED;
use names::{EVERY_VARIABLE, IN_RANGE, Names, SPEC, STARTS, STEPS, STOP};
use scribe::{Scope, Scribe, Variable, Written, names_written, set_of};
use std::collections::BTreeSet;

/// The invariants one rule is exported as.
///
/// TLC's verdict for the rule is broken when a state breaks `holds`; otherwise unknown when a
/// state breaks `judged`, because the rule has no value there; otherwise it holds. The module
/// defines both and [`Files::config`] names only `holds`: [`Files::config_judging`] gives the
/// configuration that checks the two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invariants {
    /// The rule's id in the model.
    pub rule: String,
    /// The invariant a state breaks when the rule is false in it: `Rule_` followed by the
    /// rule's id, respelled or numbered as [`write`] says. It is true in a state where the
    /// rule has no value.
    pub holds: String,
    /// The invariant a state breaks when the rule has no value in it, such as a read of a
    /// field of `null`: `Judged_` followed by the rule's id. `None` for a rule that has a
    /// value in every state.
    pub judged: Option<String>,
}

impl Invariants {
    /// The names of the rule's invariants as the module defines them: `holds`, then `judged`
    /// when the rule has one.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.holds.as_str()).chain(self.judged.as_deref())
    }
}

/// The temporal property one rule about runs is exported as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    /// The rule's id in the model.
    pub rule: String,
    /// The property's name in the module: `Eventually_` followed by the rule's id, respelled
    /// or numbered as [`write`] says. A behavior of `Spec` that never reaches what the rule
    /// wants breaks it.
    pub name: String,
}

/// The text of the two files TLC reads to check one model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Files {
    /// The module's name. TLC wants the files named after it: `<name>.tla` and `<name>.cfg`.
    /// It is the model's name, respelled when TLA+ can't spell it or keeps the word for itself.
    pub name: String,
    /// The text of `<name>.tla`.
    pub module: String,
    /// The text of `<name>.cfg`. It names one invariant per rule, in the order of the model's
    /// rules: the `holds` invariant of each entry of `rules`. A rule that has no value in a
    /// state is true there, so this configuration never stops TLC on such a rule and never
    /// reports it either.
    pub config: String,
    /// The invariants each rule is exported as, in the order of the model's rules.
    pub rules: Vec<Invariants>,
    /// The temporal property each rule about runs is exported as, in the model's order. When
    /// there is one, `config` names the specification with its fairness in place of `Init`
    /// and `Next`, and each property after the invariants.
    pub eventually: Vec<Property>,
    /// The name of the specification with its fairness, which the module defines only for a
    /// model with a rule about runs.
    pub specification: Option<String>,
}

const INVARIANT: &str = "INVARIANT ";
const PROPERTY: &str = "PROPERTY ";
const NO_DEADLOCK_CHECK: &str = "CHECK_DEADLOCK FALSE\n";

impl Files {
    /// The invariants `config` names, one per rule, in the order of the model's rules.
    #[must_use]
    pub fn invariants(&self) -> Vec<&str> {
        self.config
            .lines()
            .filter_map(|line| line.strip_prefix(INVARIANT))
            .collect()
    }

    /// The text of a configuration that checks only the invariants in `kept`, from `Init` by
    /// `Next` and without the deadlock check, as `config` does for a model with no rule about
    /// runs.
    ///
    /// TLC reports one broken invariant per state, so a run that checks one rule alone is how
    /// to learn whether that rule is broken whatever the other rules do. An empty `kept` gives
    /// a run that only counts the states. A name `config` does not have is ignored. No
    /// temporal property is checked.
    #[must_use]
    pub fn config_checking(&self, kept: &[&str]) -> String {
        let mut text = head();
        for invariant in self.invariants() {
            if kept.contains(&invariant) {
                text.push_str(INVARIANT);
                text.push_str(invariant);
                text.push('\n');
            }
        }
        text
    }

    /// The text of a configuration that checks one rule about runs alone: the specification
    /// with its fairness and the rule's temporal property. `None` when the module defines no
    /// specification, as for a model with no rule about runs.
    #[must_use]
    pub fn config_living(&self, property: &Property) -> Option<String> {
        let specification = self.specification.as_deref()?;
        Some(format!(
            "SPECIFICATION {specification}\n{NO_DEADLOCK_CHECK}{PROPERTY}{}\n",
            property.name
        ))
    }
}

impl Files {
    /// The text of a configuration that checks one rule alone and also says where the rule
    /// has no value: its `holds` invariant, then its `judged` invariant when it has one, with
    /// everything else as in `config`.
    ///
    /// A state breaks at most one of the two, so one run of TLC with `-continue` gives the
    /// rule's verdict as [`Invariants`] describes it.
    #[must_use]
    pub fn config_judging(&self, rule: &Invariants) -> String {
        let mut text = self.config_checking(&[]);
        for invariant in rule.names() {
            text.push_str(INVARIANT);
            text.push_str(invariant);
            text.push('\n');
        }
        text
    }
}

const INDENT: &str = "    ";
const EXTENDED: &str = "EXTENDS Integers, Sequences, FiniteSets, TLC";
const FILLER: &str = "nothing";

/// Why TLC can't confirm `model`, from what the model declares: a number outside TLC's 32-bit
/// integers, or a text that is not ASCII, which TLC may count as equal to another. Empty when
/// nothing declared is beyond TLC. A number the model computes while it runs is not seen
/// here: the search notes it, and [`BEYOND_32_BITS_COMPUTED`] is the sentence for it.
#[must_use]
pub fn limits(model: &Model) -> Vec<String> {
    limits::declared(model)
}

/// The TLA+ module and TLC configuration for `model`.
///
/// To compare TLC with the search:
///
/// - Run TLC with `-continue`. Without it TLC stops at the first broken rule and counts fewer
///   states than the search.
/// - In one state TLC reports only the first invariant it finds broken. A rule that breaks only
///   where an earlier rule breaks too is then not reported, so give such a rule a run of its
///   own.
/// - A rule that has no value in a state, which the search reports as a fault, is true there
///   in its `Rule_` invariant, so TLC goes on and counts every state. Such a state breaks the
///   rule's `Judged_` invariant, which the module defines and the configuration does not
///   name: check it with [`Files::config_judging`].
/// - A construct this writer does not know is written as a call of `Fault`, which is an error
///   in TLC, and TLC then stops whatever the flags. So is a number beyond 32 bits, which TLC
///   can't hold.
///
/// A model in a setup is a module of its own, named after the model and the setup's id, with
/// the setup's start values in `Init`.
///
/// A field, input, action, rule or end keeps its name when TLA+ can spell it and nothing else
/// has it; otherwise letters TLA+ lacks become `_` and a taken name gets `_2`, `_3` and so on.
#[must_use]
pub fn write(model: &Model) -> Files {
    write_apart_from(model, &[])
}

/// [`write`], with a module name that is none of `modules`: when the name [`write`] gives is
/// one of them, it gets `_2`, `_3` and so on, as a taken name does inside a module. Give it the
/// names of the modules already written, so two runs of one model whose names TLA+ spells
/// alike get files of their own.
#[must_use]
pub fn write_apart_from(model: &Model, modules: &[&str]) -> Files {
    let mut names = Names::of_a_module(EVERY_HELPER.iter().map(|helper| helper.name()));
    let name = names.claim_apart_from(
        &match model.setup() {
            Some(setup) => format!("{}_{}", model.name, setup.id),
            None => model.name.clone(),
        },
        modules,
    );
    let variables: Vec<Variable> = model
        .fields
        .iter()
        .map(|field| Variable {
            name: names.claim(&field.name),
            kind: Kind::of_domain(&field.domain),
        })
        .collect();
    let filler = variables.is_empty().then(|| names.claim(FILLER));
    let ids = |all: Vec<&str>, kind: &str, names: &mut Names| -> Vec<String> {
        all.into_iter()
            .map(|id| names.claim(&format!("{kind}_{id}")))
            .collect()
    };
    let actions = ids(
        model.actions.iter().map(|each| each.id.as_str()).collect(),
        "Action",
        &mut names,
    );
    let rules = ids(
        model.rules.iter().map(|each| each.id.as_str()).collect(),
        "Rule",
        &mut names,
    );
    let ends = ids(
        model.ends.iter().map(|each| each.id.as_str()).collect(),
        "End",
        &mut names,
    );
    let labels = Names::of_a_module(EVERY_HELPER.iter().map(|helper| helper.name()));
    let mut writer = Writer {
        model,
        scribe: Scribe::new(&model.symbols, variables, labels),
        names,
        filler,
    };
    let mut blocks = vec![writer.in_range(), writer.starts()];
    for (position, (action, defined)) in model.actions.iter().zip(&actions).enumerate() {
        blocks.push(writer.action(position, action, defined));
    }
    for (end, defined) in model.ends.iter().zip(&ends) {
        let when = writer.condition(end.when.expr());
        let marked = match when.sound {
            Some(sound) => format!("((~{sound}) \\/ {})", when.text),
            None => when.text,
        };
        blocks.push(definition(defined, &[marked]));
    }
    blocks.push(stop(&ends));
    blocks.push(steps(&actions));
    let mut exported = Vec::with_capacity(model.rules.len());
    for (rule, defined) in model.rules.iter().zip(rules) {
        let holds = writer.condition(rule.holds.expr());
        let judged = holds.sound.map(|sound| {
            let judged = writer.names.claim(&format!("Judged_{}", rule.id));
            (judged, sound)
        });
        let judged_or_holds = match &judged {
            Some((_, sound)) => format!("((~{sound}) \\/ {})", holds.text),
            None => holds.text,
        };
        blocks.push(definition(
            &defined,
            &[format!("{IN_RANGE} => {judged_or_holds}")],
        ));
        if let Some((name, sound)) = &judged {
            blocks.push(definition(name, &[format!("{IN_RANGE} => {sound}")]));
        }
        exported.push(Invariants {
            rule: rule.id.clone(),
            holds: defined,
            judged: judged.map(|(name, _)| name),
        });
    }
    let (specification, eventually) = writer.runs(&actions, &mut blocks);
    Files {
        module: writer.module(&name, &blocks),
        config: config(&exported, specification.as_deref(), &eventually),
        rules: exported,
        eventually,
        specification,
        name,
    }
}

struct Writer<'a> {
    model: &'a Model,
    scribe: Scribe<'a>,
    names: Names,
    filler: Option<String>,
}

impl Writer<'_> {
    fn every_variable(&self) -> Vec<String> {
        self.scribe
            .variables()
            .iter()
            .map(|variable| variable.name.clone())
            .chain(self.filler.clone())
            .collect()
    }

    fn module(&self, name: &str, blocks: &[String]) -> String {
        let every = self.every_variable().join(", ");
        let written: BTreeSet<&str> = blocks
            .iter()
            .flat_map(|block| names_written(block))
            .collect();
        let used: Vec<Helper> = EVERY_HELPER
            .into_iter()
            .filter(|helper| written.contains(helper.name()))
            .collect();
        let mut texts: Vec<String> = self.model.symbols.texts().map(str::to_owned).collect();
        texts.sort();
        let helpers: Vec<String> = EVERY_HELPER
            .into_iter()
            .filter(|helper| {
                used.iter()
                    .any(|other| other == helper || other.needs() == Some(*helper))
            })
            .map(|helper| helper.definition(&self.names, &texts))
            .collect();
        let mut sections = vec![
            format!("---- MODULE {name} ----\n{EXTENDED}"),
            format!("VARIABLES {every}"),
            format!("{EVERY_VARIABLE} == <<{every}>>"),
        ];
        if !helpers.is_empty() {
            sections.push(helpers.join("\n"));
        }
        sections.extend(blocks.iter().cloned());
        sections.push("====\n".to_owned());
        sections.join("\n\n")
    }

    fn runs(
        &mut self,
        actions: &[String],
        blocks: &mut Vec<String>,
    ) -> (Option<String>, Vec<Property>) {
        let model = self.model;
        if model.runs.eventually.is_empty() {
            return (None, Vec::new());
        }
        let specification = self.names.claim(SPEC);
        let mut parts = vec![
            STARTS.to_owned(),
            format!("[][{STEPS}]_{EVERY_VARIABLE}"),
            format!("WF_{EVERY_VARIABLE}({STEPS})"),
        ];
        for action in model
            .runs
            .fair
            .iter()
            .filter_map(|position| actions.get(*position))
        {
            parts.push(format!("WF_{EVERY_VARIABLE}({action})"));
        }
        let fairness = parts.join(" /\\ ");
        blocks.push(definition(&specification, &[fairness]));
        let mut properties = Vec::with_capacity(model.runs.eventually.len());
        for rule in &model.runs.eventually {
            let name = self.names.claim(&format!("Eventually_{}", rule.id));
            let reaches = self.of_the_state(rule.reaches.expr());
            let body = match &rule.after {
                Some(after) => {
                    let after = self.of_the_state(after.expr());
                    format!("({after}) ~> ({reaches})")
                }
                None => format!("<>({reaches})"),
            };
            blocks.push(definition(&name, &[body]));
            properties.push(Property {
                rule: rule.id.clone(),
                name,
            });
        }
        (Some(specification), properties)
    }

    fn of_the_state(&mut self, expr: &Expr) -> String {
        let text = self.condition(expr).text;
        if reads_the_state(expr) {
            return text;
        }
        format!("{EVERY_VARIABLE} = {EVERY_VARIABLE} /\\ {text}")
    }

    fn in_range(&mut self) -> String {
        let mut scope = Scope::new(&self.names);
        let mut lines = Vec::with_capacity(self.model.fields.len());
        for (field, variable) in self.model.fields.iter().zip(self.every_variable()) {
            let fits = self.scribe.within(&variable, &field.domain, &mut scope);
            lines.push(format!("/\\ {fits}"));
        }
        if lines.is_empty() {
            lines.push("TRUE".to_owned());
        }
        definition(IN_RANGE, &lines)
    }

    fn listed_starts(&mut self, listed: &[Vec<Value>]) -> String {
        let variables = self.scribe.variables().to_vec();
        let mut states = Vec::with_capacity(listed.len());
        for state in listed {
            let parts: Vec<String> = state
                .iter()
                .zip(&variables)
                .map(|(value, variable)| {
                    format!(
                        "({} = {})",
                        variable.name,
                        self.scribe.value(value, &variable.kind)
                    )
                })
                .collect();
            states.push(match parts.as_slice() {
                [] => "TRUE".to_owned(),
                [only] => only.clone(),
                many => format!("({})", many.join(" /\\ ")),
            });
        }
        let either = if states.is_empty() {
            "FALSE".to_owned()
        } else {
            states.join(" \\/ ")
        };
        let mut lines = vec![format!("/\\ ({either})")];
        if let Some(filler) = &self.filler {
            lines.push(format!("/\\ {filler} = 0"));
        }
        definition(STARTS, &lines)
    }

    fn starts(&mut self) -> String {
        if let Some(listed) = self.model.runs.starts.clone() {
            return self.listed_starts(&listed);
        }
        let mut lines = Vec::with_capacity(self.model.fields.len());
        let variables = self.scribe.variables().to_vec();
        for (field, variable) in self.model.fields.iter().zip(variables) {
            let values = self.scribe.values(field.starts.iter(), &variable.kind);
            lines.push(match values.as_slice() {
                [only] => format!("/\\ {} = {only}", variable.name),
                many => format!("/\\ {} \\in {}", variable.name, set_of(many)),
            });
        }
        if let Some(filler) = &self.filler {
            lines.push(format!("/\\ {filler} = 0"));
        }
        definition(STARTS, &lines)
    }

    fn condition(&mut self, expr: &Expr) -> Written {
        let mut scope = Scope::new(&self.names);
        self.scribe.expr(expr, &mut scope)
    }

    fn action(&mut self, position: usize, action: &Action, defined: &str) -> String {
        let mut scope = Scope::new(&self.names);
        let mut choices = Vec::with_capacity(action.inputs.len());
        let mut needed = vec![IN_RANGE.to_owned()];
        let picks: Vec<_> = self
            .model
            .runs
            .picks
            .iter()
            .filter(|pick| pick.action == position)
            .collect();
        for (at, input) in action.inputs.iter().enumerate() {
            let (set, kind) = if let Some(pick) = picks.iter().find(|pick| pick.input == at) {
                let written = self.scribe.expr(pick.from.expr(), &mut scope);
                needed.extend(written.sound);
                (written.text, written.kind.item())
            } else {
                let kind = Kind::of_domain(&input.domain);
                let values = self.scribe.values(input.values.iter(), &kind);
                (set_of(&values), kind)
            };
            let name = scope.bind(input.slot, &input.name, kind);
            choices.push(format!("{name} \\in {set}"));
        }
        let joined = if picks.is_empty() { ", " } else { " : \\E " };
        if !matches!(action.guard.expr(), Expr::Const(Value::Bool(true))) {
            let allowed = self.scribe.expr(action.guard.expr(), &mut scope);
            needed.extend(allowed.sound);
            needed.push(allowed.text);
        }
        let mut primed = Vec::new();
        let mut kept = Vec::new();
        for (position, variable) in self.scribe.variables().to_vec().iter().enumerate() {
            if let Some(Some(update)) = action.updates.get(position) {
                let next = self.scribe.expr(update.expr(), &mut scope);
                needed.extend(next.sound);
                primed.push(format!("/\\ {}' = {}", variable.name, next.text));
            } else {
                kept.push(variable.name.clone());
            }
        }
        kept.extend(self.filler.clone());
        if !kept.is_empty() {
            primed.push(format!("/\\ UNCHANGED <<{}>>", kept.join(", ")));
        }
        let mut lines = vec![format!("IF {}", needed.join(" /\\ "))];
        for (position, conjunct) in primed.iter().enumerate() {
            let lead = if position == 0 { "THEN" } else { "    " };
            lines.push(format!("{lead} {conjunct}"));
        }
        lines.push("ELSE FALSE".to_owned());
        if choices.is_empty() {
            return definition(defined, &lines);
        }
        let mut chosen = vec![format!("\\E {} :", choices.join(joined))];
        chosen.extend(lines.iter().map(|line| format!("{INDENT}{line}")));
        definition(defined, &chosen)
    }
}

fn reads_the_state(expr: &Expr) -> bool {
    let any = |parts: &[Expr]| parts.iter().any(reads_the_state);
    match expr {
        Expr::Field(_) => true,
        Expr::Apply(_, arguments) | Expr::SetOf(arguments) | Expr::SeqOf(arguments) => {
            any(arguments)
        }
        Expr::And(left, right) | Expr::Or(left, right) => {
            reads_the_state(left) || reads_the_state(right)
        }
        Expr::Choose(test, yes, no) => {
            reads_the_state(test) || reads_the_state(yes) || reads_the_state(no)
        }
        Expr::Bind { value, body, .. } => reads_the_state(value) || reads_the_state(body),
        Expr::Record(fields) => fields.iter().any(|(_, value)| reads_the_state(value)),
        Expr::Get(record, _) => reads_the_state(record),
        Expr::With(record, fields) => {
            reads_the_state(record) || fields.iter().any(|(_, value)| reads_the_state(value))
        }
        Expr::MapOf(pairs) => pairs
            .iter()
            .any(|(key, value)| reads_the_state(key) || reads_the_state(value)),
        Expr::Sweep { source, body, .. } => reads_the_state(source) || reads_the_state(body),
        _ => false,
    }
}

fn definition(name: &str, lines: &[String]) -> String {
    let mut text = format!("{name} ==");
    for line in lines {
        text.push('\n');
        text.push_str(INDENT);
        text.push_str(line);
    }
    text
}

fn stop(ends: &[String]) -> String {
    let mut reasons = vec![format!("(~{IN_RANGE})")];
    reasons.extend(ends.iter().cloned());
    definition(
        STOP,
        &[
            format!("IF {}", reasons.join(" \\/ ")),
            format!("THEN UNCHANGED {EVERY_VARIABLE}"),
            "ELSE FALSE".to_owned(),
        ],
    )
}

fn steps(actions: &[String]) -> String {
    let lines: Vec<String> = actions
        .iter()
        .map(String::as_str)
        .chain([STOP])
        .map(|action| format!("\\/ {action}"))
        .collect();
    definition(STEPS, &lines)
}

fn head() -> String {
    format!("INIT {STARTS}\nNEXT {STEPS}\n{NO_DEADLOCK_CHECK}")
}

fn config(rules: &[Invariants], specification: Option<&str>, eventually: &[Property]) -> String {
    let mut text = match specification {
        Some(specification) => format!("SPECIFICATION {specification}\n{NO_DEADLOCK_CHECK}"),
        None => head(),
    };
    for rule in rules {
        text.push_str(INVARIANT);
        text.push_str(&rule.holds);
        text.push('\n');
    }
    for property in eventually {
        text.push_str(PROPERTY);
        text.push_str(&property.name);
        text.push('\n');
    }
    text
}
