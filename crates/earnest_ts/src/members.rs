use crate::Reader;
use crate::bodies::body_span;
use crate::terms::bare;
use earnest_ast::{
    ActionBody, ActionDraft, COLORED_PARTS, ConditionDraft, Draft, EndDraft, EventuallyDraft,
    LabelDraft, Quoting, RuleDraft, SetupDraft, Shape, StepDraft, Term, TermKind, ViewDraft,
    WitnessDraft, condition_text, is_color,
};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionExpression, BindingPattern, CallExpression,
    Expression, ObjectPattern,
};
use oxc_span::GetSpan;

#[derive(Clone, Copy)]
pub(crate) enum MemberKind {
    Action,
    Rule,
    End,
    View,
    Witness,
    Eventually,
    Setup,
}

pub(crate) struct Member<'p, 'a> {
    pub(crate) model: usize,
    pub(crate) kind: MemberKind,
    pub(crate) id: String,
    pub(crate) start: u32,
    pub(crate) call: &'p CallExpression<'a>,
}

impl MemberKind {
    pub(crate) fn named(part: &str) -> Option<Self> {
        match part {
            "action" => Some(Self::Action),
            "rule" => Some(Self::Rule),
            "end" => Some(Self::End),
            "view" => Some(Self::View),
            "witness" => Some(Self::Witness),
            "eventually" => Some(Self::Eventually),
            "setup" => Some(Self::Setup),
            _ => None,
        }
    }
}

const SENTENCE: usize = 1;
const FUNCTION: usize = 2;
const ACTION_OBJECT: usize = 2;
const VIEW_OBJECT: usize = 1;
const WITNESS_OBJECT: usize = 2;
const EVENTUALLY_OBJECT: usize = 2;
const SETUP_OBJECT: usize = 2;
const NOT_A_SETUP: &str = "a witness names its setup by the constant of a setup of this model \
    declared above it, as in `setup: behind`";
const FAIR_IS_WRITTEN_OUT: &str = "`fair` is written out as `true` or `false`: `true` says the \
    action is not put off forever while it stays possible";
const STEP: &str = "step";
const NOT_A_STEP: &str = "a step is an action of this model declared above the witness, written \
    as its constant's name, or as `m.step(action, { input: value })` for an action with inputs";

type Entries<'p, 'a> = Vec<(String, u32, &'p Expression<'a>)>;

#[derive(Default)]
struct ActionParameters {
    state: Option<String>,
    whole_input: Option<String>,
    picked_inputs: Vec<(String, String)>,
}

struct StateFunction {
    state_name: Option<String>,
    term: Term,
    text: String,
}

fn pick<'p, 'a>(entries: &Entries<'p, 'a>, key: &str) -> Option<&'p Expression<'a>> {
    entries
        .iter()
        .find(|(name, _, _)| name == key)
        .map(|(_, _, value)| *value)
}

impl Reader<'_> {
    pub(crate) fn member_of(&mut self, draft: &mut Draft, member: Member<'_, '_>) {
        let Member {
            kind,
            id,
            start,
            call,
            ..
        } = member;
        match kind {
            MemberKind::Action => {
                if let Some(action) = self.action(id, start, call) {
                    draft.actions.push(action);
                }
            }
            MemberKind::Rule => {
                if let Some(rule) = self.rule(id, start, call) {
                    draft.rules.push(rule);
                }
            }
            MemberKind::End => {
                if let Some(end) = self.end(id, start, call) {
                    draft.ends.push(end);
                }
            }
            MemberKind::View => draft.view = self.view(call),
            MemberKind::Witness => {
                if let Some(witness) = self.witness(id, start, call, draft) {
                    draft.runs.witnesses.push(witness);
                }
            }
            MemberKind::Eventually => {
                if let Some(rule) = self.eventually(id, start, call) {
                    draft.runs.eventually.push(rule);
                }
            }
            MemberKind::Setup => {
                if let Some(setup) = self.setup(id, start, call) {
                    draft.runs.setups.push(setup);
                }
            }
        }
    }

    pub(crate) fn sentence(&mut self, wrapped: &Expression<'_>) -> String {
        let expr = bare(wrapped);
        if let TermKind::Text(text) = self.term(expr).kind {
            text
        } else {
            let before = self.quoted(expr.span());
            self.refuse(expr.span().start, &before, "text in quotes is needed here");
            String::new()
        }
    }

    fn parts<'p, 'a>(
        &mut self,
        call: &'p CallExpression<'a>,
        index: usize,
        allowed: &[&str],
    ) -> Option<Entries<'p, 'a>> {
        let expected = index + 1;
        let found = call.arguments.get(index).and_then(Argument::as_expression);
        let (Some(object), true) = (found, call.arguments.len() == expected) else {
            self.refuse(
                call.span.start,
                &self.quoted(call.callee.span()),
                &format!(
                    "its parts are needed here, as an object with {}",
                    allowed.join(", ")
                ),
            );
            return None;
        };
        let entries = self.entries(object);
        for (name, start, _) in &entries {
            if !allowed.contains(&name.as_str()) {
                self.refuse(
                    *start,
                    name,
                    &format!(
                        "it is not a part of this; the parts are {}",
                        allowed.join(", ")
                    ),
                );
            }
        }
        Some(entries)
    }

    fn action(&mut self, id: String, start: u32, call: &CallExpression<'_>) -> Option<ActionDraft> {
        let entries = self.parts(call, ACTION_OBJECT, &["input", "when", "changes", "fair"])?;
        let sentence = self.argument(
            call,
            SENTENCE,
            "its sentence is needed here, as text in quotes and an argument of its own",
        )?;
        let label = self.sentence(sentence);
        let inputs: Vec<(String, Shape)> = match pick(&entries, "input") {
            Some(declared) => self.fields(declared),
            None => Vec::new(),
        };
        let when = pick(&entries, "when").and_then(|written| self.arrow(written));
        let changes = pick(&entries, "changes");
        if changes.is_none() {
            self.refuse(
                call.span.start,
                &self.quoted(call.callee.span()),
                "it needs a `changes`",
            );
        }
        let changing = changes.and_then(|written| self.arrow(written));
        let guard = when.map(|arrow| self.action_body(arrow));
        let fair = pick(&entries, "fair").is_some_and(|written| self.written_out(written));
        let change = changing.map(|arrow| self.action_body(arrow))?;
        Some(ActionDraft {
            id,
            label,
            place: self.place(start),
            inputs,
            guard,
            change,
            fair,
        })
    }

    fn written_out(&mut self, wrapped: &Expression<'_>) -> bool {
        let written = bare(wrapped);
        if let Expression::BooleanLiteral(flag) = written {
            return flag.value;
        }
        let feature = self.quoted(written.span());
        self.refuse(written.span().start, &feature, FAIR_IS_WRITTEN_OUT);
        false
    }

    fn action_parameters(&mut self, arrow: &ArrowFunctionExpression<'_>) -> ActionParameters {
        let mut found = ActionParameters::default();
        for (index, parameter) in arrow.params.items.iter().enumerate() {
            match (index, &parameter.pattern) {
                (0, BindingPattern::BindingIdentifier(name)) => {
                    found.state = Some(name.name.to_string());
                }
                (1, BindingPattern::BindingIdentifier(name)) => {
                    found.whole_input = Some(name.name.to_string());
                }
                (1, BindingPattern::ObjectPattern(pattern)) => {
                    found.picked_inputs = self.picked_inputs(pattern);
                }
                (_, other) => {
                    let written = self.quoted(other.span());
                    self.refuse(
                        other.span().start,
                        &written,
                        "an action's function takes the state by one name, then its input",
                    );
                }
            }
        }
        if let Some(rest) = &arrow.params.rest {
            self.refuse(
                rest.span.start,
                "...",
                "a rest parameter is outside the library: take each value as a parameter of its own",
            );
        }
        found
    }

    fn picked_inputs(&mut self, pattern: &ObjectPattern<'_>) -> Vec<(String, String)> {
        let mut picked = Vec::with_capacity(pattern.properties.len());
        for property in &pattern.properties {
            let key = property.key.static_name();
            match (key, &property.value) {
                (Some(input), BindingPattern::BindingIdentifier(local)) if !property.computed => {
                    picked.push((local.name.to_string(), input.into_owned()));
                }
                _ => {
                    let written = self.quoted(property.span);
                    self.refuse(
                        property.span.start,
                        &written,
                        "an input is picked by its name: `{ name }`",
                    );
                }
            }
        }
        if let Some(rest) = &pattern.rest {
            self.refuse(rest.span.start, "...", "each input is picked by its name");
        }
        picked
    }

    fn action_body(&mut self, arrow: &ArrowFunctionExpression<'_>) -> ActionBody {
        let parameters = self.action_parameters(arrow);
        let mark = self.locals.len();
        for own in [&parameters.state, &parameters.whole_input]
            .into_iter()
            .flatten()
        {
            self.locals.push((own.clone(), own.clone()));
        }
        self.locals.extend(parameters.picked_inputs.iter().cloned());
        let term = self.body(arrow);
        self.locals.truncate(mark);
        ActionBody {
            state_name: parameters.state,
            input_name: parameters.whole_input,
            taken: parameters
                .picked_inputs
                .into_iter()
                .map(|(_, input)| input)
                .collect(),
            term,
        }
    }

    fn state_function(&mut self, wrapped: &Expression<'_>) -> Option<StateFunction> {
        let arrow = self.arrow(wrapped)?;
        let (parameters, term) = self.function(arrow);
        if parameters.len() > 1 {
            self.refuse(
                arrow.params.span.start,
                &self.quoted(arrow.params.span),
                "this function takes only the state",
            );
        }
        Some(StateFunction {
            state_name: parameters.into_iter().next(),
            term,
            text: condition_text(self.text(body_span(arrow)), Quoting::TypeScript),
        })
    }

    fn sentence_and_function(
        &mut self,
        call: &CallExpression<'_>,
    ) -> (Option<String>, Option<StateFunction>) {
        let sentence = call
            .arguments
            .get(SENTENCE)
            .and_then(Argument::as_expression);
        let function = call
            .arguments
            .get(FUNCTION)
            .and_then(Argument::as_expression);
        let (Some(sentence), Some(function), 3) = (sentence, function, call.arguments.len()) else {
            let callee = self.quoted(call.callee.span());
            self.refuse(
                call.span.start,
                &callee,
                &format!(
                    "its model, its sentence and its condition are needed here, as in \
                     `{callee}(Model, \"sentence\", (s) => ...)`"
                ),
            );
            return (None, None);
        };
        (Some(self.sentence(sentence)), self.state_function(function))
    }

    fn rule(&mut self, id: String, start: u32, call: &CallExpression<'_>) -> Option<RuleDraft> {
        let (says, holds) = self.sentence_and_function(call);
        let condition = holds?;
        Some(RuleDraft {
            id,
            says: says?,
            place: self.place(start),
            state_name: condition.state_name,
            holds: condition.term,
            text: condition.text,
        })
    }

    fn end(&mut self, id: String, start: u32, call: &CallExpression<'_>) -> Option<EndDraft> {
        let (says, when) = self.sentence_and_function(call);
        let condition = when?;
        Some(EndDraft {
            id,
            says: says?,
            place: self.place(start),
            state_name: condition.state_name,
            when: condition.term,
        })
    }

    fn condition(&mut self, wrapped: &Expression<'_>) -> Option<ConditionDraft> {
        let function = self.state_function(wrapped)?;
        Some(ConditionDraft {
            state_name: function.state_name,
            term: function.term,
            text: function.text,
        })
    }

    fn eventually(
        &mut self,
        id: String,
        start: u32,
        call: &CallExpression<'_>,
    ) -> Option<EventuallyDraft> {
        let entries = self.parts(call, EVENTUALLY_OBJECT, &["after", "reaches"])?;
        let sentence = self.argument(
            call,
            SENTENCE,
            "its sentence is needed here, as text in quotes and an argument of its own",
        )?;
        let says = self.sentence(sentence);
        let after = match pick(&entries, "after") {
            Some(written) => Some(self.condition(written)?),
            None => None,
        };
        let Some(reached) = pick(&entries, "reaches") else {
            self.refuse(
                call.span.start,
                &self.quoted(call.callee.span()),
                "it needs a `reaches`, the condition every run comes to",
            );
            return None;
        };
        Some(EventuallyDraft {
            id,
            says,
            place: self.place(start),
            after,
            reaches: self.condition(reached)?,
        })
    }

    fn setup(&mut self, id: String, start: u32, call: &CallExpression<'_>) -> Option<SetupDraft> {
        let callee = self.quoted(call.callee.span());
        let (Some(sentence), Some(written), 3) = (
            call.arguments
                .get(SENTENCE)
                .and_then(Argument::as_expression),
            call.arguments
                .get(SETUP_OBJECT)
                .and_then(Argument::as_expression),
            call.arguments.len(),
        ) else {
            self.refuse(
                call.span.start,
                &callee,
                &format!(
                    "its model, its sentence and the fields it starts elsewhere are needed \
                     here, as in `{callee}(Model, \"sentence\", {{ field: value }})`"
                ),
            );
            return None;
        };
        let says = self.sentence(sentence);
        let starts: Vec<(String, earnest_diagnostics::Place, Term)> = self
            .entries(written)
            .into_iter()
            .map(|(name, offset, value)| (name, self.place(offset), self.term(value)))
            .collect();
        if starts.is_empty() {
            self.refuse(
                written.span().start,
                &self.quoted(written.span()),
                "a setup starts at least one field at another value: without one it is the \
                 model as declared, which is checked already",
            );
            return None;
        }
        Some(SetupDraft {
            id,
            says,
            place: self.place(start),
            starts,
            text: condition_text(self.text(written.span()), Quoting::TypeScript),
        })
    }

    fn setup_named(
        &mut self,
        wrapped: &Expression<'_>,
        draft: &Draft,
    ) -> Option<(String, earnest_diagnostics::Place)> {
        let written = bare(wrapped);
        if let Expression::Identifier(name) = written
            && draft
                .runs
                .setups
                .iter()
                .any(|setup| setup.id == name.name.as_str())
        {
            return Some((name.name.to_string(), self.place(written.span().start)));
        }
        let feature = self.quoted(written.span());
        self.refuse(written.span().start, &feature, NOT_A_SETUP);
        None
    }

    fn witness(
        &mut self,
        id: String,
        start: u32,
        call: &CallExpression<'_>,
        draft: &Draft,
    ) -> Option<WitnessDraft> {
        let actions = draft.actions.as_slice();
        let entries = self.parts(call, WITNESS_OBJECT, &["setup", "steps", "expect"])?;
        let sentence = self.argument(
            call,
            SENTENCE,
            "its sentence is needed here, as text in quotes and an argument of its own",
        )?;
        let says = self.sentence(sentence);
        let callee = self.quoted(call.callee.span());
        let steps = if let Some(written) = pick(&entries, "steps") {
            self.steps(written, actions)
        } else {
            self.refuse(
                call.span.start,
                &callee,
                "it needs its `steps`, as a list of actions",
            );
            Vec::new()
        };
        let Some(expected) = pick(&entries, "expect") else {
            self.refuse(
                call.span.start,
                &callee,
                "it needs an `expect`, the condition that is true after the last step",
            );
            return None;
        };
        let setup = pick(&entries, "setup").and_then(|written| self.setup_named(written, draft));
        let condition = self.state_function(expected)?;
        let written = call
            .arguments
            .get(WITNESS_OBJECT)
            .and_then(Argument::as_expression)?;
        Some(WitnessDraft {
            id,
            says,
            place: self.place(start),
            setup,
            steps,
            state_name: condition.state_name,
            expect: condition.term,
            text: condition_text(self.text(written.span()), Quoting::TypeScript),
        })
    }

    fn steps(&mut self, wrapped: &Expression<'_>, actions: &[ActionDraft]) -> Vec<StepDraft> {
        let Expression::ArrayExpression(list) = bare(wrapped) else {
            let written = self.quoted(wrapped.span());
            self.refuse(
                wrapped.span().start,
                &written,
                "the steps are a list written out: `[first, second]`",
            );
            return Vec::new();
        };
        let mut steps = Vec::with_capacity(list.elements.len());
        for element in &list.elements {
            let step = match element {
                ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => {
                    None
                }
                other => other
                    .as_expression()
                    .and_then(|written| self.step(written, actions)),
            };
            if let Some(step) = step {
                steps.push(step);
            } else {
                let written = self.quoted(element.span());
                self.refuse(element.span().start, &written, NOT_A_STEP);
            }
        }
        steps
    }

    fn step(&mut self, wrapped: &Expression<'_>, actions: &[ActionDraft]) -> Option<StepDraft> {
        let written = bare(wrapped);
        let place = self.place(written.span().start);
        let is_action = |name: &str| actions.iter().any(|action| action.id == name);
        match written {
            Expression::Identifier(name) if is_action(name.name.as_str()) => Some(StepDraft {
                action: name.name.to_string(),
                place,
                inputs: Vec::new(),
            }),
            Expression::CallExpression(call) => {
                let Expression::StaticMemberExpression(callee) = bare(&call.callee) else {
                    return None;
                };
                let Expression::Identifier(root) = bare(&callee.object) else {
                    return None;
                };
                if !self.is_library(root.name.as_str()) || callee.property.name != STEP {
                    return None;
                }
                let [Argument::Identifier(action), given] = call.arguments.as_slice() else {
                    return None;
                };
                if !is_action(action.name.as_str()) {
                    return None;
                }
                let inputs = self
                    .entries(given.as_expression()?)
                    .into_iter()
                    .map(|(name, _, value)| (name, self.term(value)))
                    .collect();
                Some(StepDraft {
                    action: action.name.to_string(),
                    place,
                    inputs,
                })
            }
            _ => None,
        }
    }

    fn view(&mut self, call: &CallExpression<'_>) -> ViewDraft {
        let allowed = ["title", "state", "group", "step", "colors"];
        let Some(entries) = self.parts(call, VIEW_OBJECT, &allowed) else {
            return ViewDraft::default();
        };
        ViewDraft {
            title: pick(&entries, "title").map(|title| self.sentence(title)),
            state: pick(&entries, "state").and_then(|label| self.label(label)),
            group: pick(&entries, "group").and_then(|label| self.label(label)),
            steps: pick(&entries, "step").map_or_else(Vec::new, |steps| self.texts(steps)),
            colors: pick(&entries, "colors").map_or_else(Vec::new, |colors| self.colors(colors)),
        }
    }

    fn label(&mut self, wrapped: &Expression<'_>) -> Option<LabelDraft> {
        let function = self.state_function(wrapped)?;
        Some(LabelDraft {
            state_name: function.state_name,
            term: function.term,
        })
    }

    fn colors(&mut self, wrapped: &Expression<'_>) -> Vec<(String, String)> {
        let mut found = Vec::new();
        for (name, start, value) in self.entries(wrapped) {
            let color = self.sentence(value);
            if !COLORED_PARTS.contains(&name.as_str()) {
                self.refuse(
                    start,
                    &name,
                    "`colors` colors `broken`, `start` and `end`, and no other part",
                );
            } else if !is_color(&color) {
                let written = self.quoted(value.span());
                self.refuse(
                    value.span().start,
                    &written,
                    "a color is `#` and 3, 4, 6 or 8 hexadecimal digits, or a color name of \
                     letters such as `red`; the chart would leave anything else out",
                );
            }
            found.push((name, color));
        }
        found
    }

    fn texts(&mut self, wrapped: &Expression<'_>) -> Vec<(String, String)> {
        let mut found = Vec::new();
        for (name, start, value) in self.entries(wrapped) {
            self.step_places.push((name.clone(), start));
            found.push((name, self.sentence(value)));
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use crate::read;
    use crate::show::{show, show_shape};
    use earnest_ast::{ActionDraft, Draft};
    use earnest_diagnostics::Place;

    const HEAD: &str = "import { m } from \"@earnestjs/model\";\n\
        const Press = m.enum([\"key\", \"mouse\"]);\n\
        export const M = m.model(\"M\", { n: m.int(0, 3), last: Press.optional() });\n";

    const NO_NAMES: [&str; 0] = [];

    fn draft(rest: &str) -> Draft {
        let reading = read("m.model.ts", &format!("{HEAD}{rest}"));
        assert_eq!(reading.problems, [], "no problem in: {rest}");
        reading.drafts.into_iter().next().expect("one model")
    }

    fn action(parts: &str) -> ActionDraft {
        let rest = format!("export const act = m.action(M, \"the user acts\", {parts});\n");
        draft(&rest).actions.into_iter().next().expect("one action")
    }

    fn features(rest: &str) -> Vec<String> {
        read("m.model.ts", &format!("{HEAD}{rest}"))
            .problems
            .into_iter()
            .map(|problem| problem.feature)
            .collect()
    }

    #[test]
    fn an_action_with_only_its_changes() {
        let found = action("{ changes: (s) => ({ n: s.n + 1 }) }");
        assert_eq!(found.id, "act", "the exported name");
        assert_eq!(found.label, "the user acts", "the sentence");
        assert_eq!(found.place, Place { line: 4, column: 1 }, "the `export`");
        assert_eq!(found.inputs, [], "no input");
        assert_eq!(found.guard, None, "no guard");
        assert_eq!(
            found.change.state_name.as_deref(),
            Some("s"),
            "the state's name"
        );
        assert_eq!(found.change.input_name, None, "no input name");
        assert_eq!(found.change.taken, NO_NAMES, "no input is taken");
        assert_eq!(show(&found.change.term), "{n: (+ s.n 1)}", "the change");
    }

    #[test]
    fn an_action_that_picks_its_input() {
        let found = action(
            "{ input: { by: Press }, when: (s, { by }) => s.n < 3 && by === \"key\", \
             changes: (_, { by }) => ({ last: by }) }",
        );
        let inputs: Vec<(&str, String)> = found
            .inputs
            .iter()
            .map(|(name, shape)| (name.as_str(), show_shape(shape)))
            .collect();
        assert_eq!(
            inputs,
            [("by", "enum(\"key\", \"mouse\")".to_owned())],
            "the input"
        );
        let guard = found.guard.expect("a guard");
        assert_eq!(
            guard.state_name.as_deref(),
            Some("s"),
            "the state in `when`"
        );
        assert_eq!(guard.input_name, None, "the input is read by its own name");
        assert_eq!(guard.taken, ["by"], "`when` takes the input");
        assert_eq!(
            show(&guard.term),
            "(&& (< s.n 3) (=== by \"key\"))",
            "the guard"
        );
        assert_eq!(
            found.change.state_name.as_deref(),
            Some("_"),
            "the state in `changes`"
        );
        assert_eq!(found.change.taken, ["by"], "`changes` takes the input");
        assert_eq!(show(&found.change.term), "{last: by}", "the change");
    }

    #[test]
    fn an_action_that_takes_its_whole_input() {
        let found = action(
            "{ input: { by: Press }, when: (s, input) => input.by === \"key\", \
             changes: (state, given) => ({ last: given.by, n: state.n }) }",
        );
        let guard = found.guard.expect("a guard");
        assert_eq!(
            guard.state_name.as_deref(),
            Some("s"),
            "the state in `when`"
        );
        assert_eq!(
            guard.input_name.as_deref(),
            Some("input"),
            "the input in `when`"
        );
        assert_eq!(guard.taken, NO_NAMES, "no input is picked out");
        assert_eq!(show(&guard.term), "(=== input.by \"key\")", "the guard");
        assert_eq!(
            found.change.state_name.as_deref(),
            Some("state"),
            "the state in `changes`"
        );
        assert_eq!(
            found.change.input_name.as_deref(),
            Some("given"),
            "the input in `changes`"
        );
        assert_eq!(
            show(&found.change.term),
            "{last: given.by, n: state.n}",
            "`changes` reads through its own names"
        );
    }

    #[test]
    fn a_picked_input_may_be_renamed() {
        let found =
            action("{ input: { by: Press }, changes: (s, { by: who }) => ({ last: who }) }");
        assert_eq!(found.change.taken, ["by"], "the input that is taken");
        assert_eq!(
            show(&found.change.term),
            "{last: by}",
            "the input's own name reaches the tree"
        );
    }

    #[test]
    fn an_action_without_parameters_has_no_state_name() {
        let found = action("{ changes: () => ({ n: 0 }) }");
        assert_eq!(found.change.state_name, None, "nothing is read");
    }

    #[test]
    fn a_block_in_changes_folds_like_a_helper() {
        let found = action(
            "{ changes: (s) => { const next = s.n + 1; if (next > 3) return { n: 0 }; \
             return { n: next }; } }",
        );
        assert_eq!(
            show(&found.change.term),
            "(let next = (+ s.n 1) in (? (> next 3) {n: 0} {n: next}))",
            "a binding and an early return"
        );
    }

    #[test]
    fn a_rule_keeps_its_source_text() {
        let found = draft(
            "export const low = m.rule(M, \"The count is low\", (s) =>\n\t\ts.n <= 2 &&\n\t\ts.n >= 0,\n);\n",
        );
        let rule = found.rules.first().expect("one rule");
        assert_eq!(rule.id, "low", "the exported name");
        assert_eq!(rule.says, "The count is low", "the sentence");
        assert_eq!(rule.place, Place { line: 4, column: 1 }, "the `export`");
        assert_eq!(rule.state_name.as_deref(), Some("s"), "the state's name");
        assert_eq!(
            show(&rule.holds),
            "(&& (<= s.n 2) (>= s.n 0))",
            "the condition"
        );
        assert_eq!(
            rule.text, "s.n <= 2 && s.n >= 0",
            "as written, with each run of white space as one space"
        );
    }

    #[test]
    fn an_end() {
        let found = draft("export const full = m.end(M, \"Full\", (s) => s.n === 3);\n");
        let end = found.ends.first().expect("one end");
        assert_eq!(end.id, "full", "the exported name");
        assert_eq!(end.says, "Full", "the sentence");
        assert_eq!(end.state_name.as_deref(), Some("s"), "the state's name");
        assert_eq!(show(&end.when), "(=== s.n 3)", "the condition");
    }

    #[test]
    fn a_view_with_every_part() {
        let found = draft(
            "export const act = m.action(M, \"act\", { changes: () => ({ n: 0 }) });\n\
             export const view = m.view(M, { title: \"Counting\", \
             state: (s) => `at ${s.n}, by ${s.last}!`, group: (s) => s.last, \
             step: { act: \"go\" }, colors: { broken: \"#c62828\", start: \"#fff\" } });\n",
        );
        let view = found.view;
        assert_eq!(view.title.as_deref(), Some("Counting"), "the title");
        let state = view.state.expect("a state label");
        assert_eq!(state.state_name.as_deref(), Some("s"), "the state's name");
        assert_eq!(
            show(&state.term),
            "`at ${s.n}, by ${s.last}!`",
            "a template"
        );
        assert_eq!(
            view.group.map(|label| show(&label.term)).as_deref(),
            Some("s.last"),
            "group"
        );
        assert_eq!(
            view.steps,
            [("act".to_owned(), "go".to_owned())],
            "arrow labels"
        );
        assert_eq!(
            view.colors,
            [
                ("broken".to_owned(), "#c62828".to_owned()),
                ("start".to_owned(), "#fff".to_owned())
            ],
            "colors"
        );
    }

    #[test]
    fn a_file_without_a_view_has_an_empty_one() {
        let found = draft("");
        assert_eq!(
            found.view,
            earnest_ast::ViewDraft::default(),
            "nothing declared"
        );
    }

    #[test]
    fn a_member_that_is_written_wrongly_is_named() {
        assert_eq!(
            features("export const a = m.action(M, \"a\", { when: (s) => s.n < 1 });\n"),
            ["`m.action`"],
            "no changes"
        );
        assert_eq!(
            features(
                "export const a = m.action(M, \"a\", { changes: (s) => ({ n: 1 }), until: 3 });\n"
            ),
            ["`until`"],
            "an unknown part"
        );
        assert_eq!(
            features("export const a = m.action(M, \"a\", { changes: ({ n }) => ({ n }) });\n"),
            ["`{ n }`"],
            "a state taken apart"
        );
        assert_eq!(
            features("export const r = m.rule(M, \"r\", (s, t) => s.n < t.n);\n"),
            ["`(s, t)`"],
            "a rule with two parameters"
        );
        assert_eq!(
            features("export const r = m.rule(M, 3, (s) => s.n < 1);\n"),
            ["`3`"],
            "a sentence that is not text"
        );
        assert_eq!(
            features("export const r = m.rule(M, \"r\", true);\n"),
            ["`true`"],
            "a condition that is not a function"
        );
        assert_eq!(
            features("export const r = m.rule();\n"),
            ["`m.rule`"],
            "no parts"
        );
    }

    const ACTIONS: &str = "export const bump = m.action(M, \"bump\", \
        { when: (s) => s.n < 3, changes: (s) => ({ n: s.n + 1 }) });\n\
        export const press = m.action(M, \"press\", \
        { input: { by: Press, times: m.int(1, 2) }, changes: (_, { by }) => ({ last: by }) });\n";

    fn witness(rest: &str) -> earnest_ast::WitnessDraft {
        let found = draft(&format!("{ACTIONS}{rest}"));
        found
            .runs
            .witnesses
            .into_iter()
            .next()
            .expect("one witness")
    }

    fn refused(rest: &str) -> Vec<(u32, u32, String)> {
        read("m.model.ts", &format!("{HEAD}{ACTIONS}{rest}"))
            .problems
            .into_iter()
            .map(|problem| (problem.line, problem.column, problem.feature))
            .collect()
    }

    fn one_refusal(rest: &str, column: u32, feature: &str) {
        assert_eq!(
            refused(rest),
            [(6, column, feature.to_owned())],
            "the line, column and feature of the one problem in: {rest}"
        );
    }

    #[test]
    fn a_witness_with_its_steps_and_what_it_expects() {
        let found = witness(
            "export const climbs = m.witness(M, \"Bump, press, bump\", \
             { steps: [bump, m.step(press, { times: 2, by: \"mouse\" }), (bump)], \
             expect: (state) => state.n === 2 });\n",
        );
        assert_eq!(found.id, "climbs", "the exported name");
        assert_eq!(found.says, "Bump, press, bump", "the sentence");
        assert_eq!(found.place, Place { line: 6, column: 1 }, "the `export`");
        assert_eq!(found.setup, None, "the model's own start states");
        let steps: Vec<String> = found
            .steps
            .iter()
            .map(|step| {
                let inputs: Vec<String> = step
                    .inputs
                    .iter()
                    .map(|(name, value)| format!("{name} = {}", show(value)))
                    .collect();
                format!(
                    "{} at {}:{} [{}]",
                    step.action,
                    step.place.line,
                    step.place.column,
                    inputs.join(", ")
                )
            })
            .collect();
        assert_eq!(
            steps,
            [
                "bump at 6:67 []",
                "press at 6:73 [times = 2, by = \"mouse\"]",
                "bump at 6:116 []",
            ],
            "each step's action, place and inputs in the order written"
        );
        assert_eq!(
            found.state_name.as_deref(),
            Some("state"),
            "the state's name"
        );
        assert_eq!(show(&found.expect), "(=== state.n 2)", "the condition");
    }

    #[test]
    fn a_witness_may_take_no_steps() {
        let found = witness(
            "export const starts = m.witness(M, \"It starts\", { steps: [], expect: () => true });\n",
        );
        assert_eq!(found.steps, [], "no steps");
        assert_eq!(found.state_name, None, "the condition reads no state");
        assert_eq!(show(&found.expect), "true", "the condition");
    }

    #[test]
    fn the_witnesses_of_a_model_are_kept_in_file_order_after_its_rules() {
        let found = draft(&format!(
            "{ACTIONS}export const second = m.witness(M, \"b\", {{ steps: [bump], expect: (s) => s.n === 1 }});\n\
             export const first = m.witness(M, \"a\", {{ steps: [], expect: (s) => s.n === 0 }});\n\
             export const low = m.rule(M, \"low\", (s) => s.n < 4);\n"
        ));
        let witnesses: Vec<&str> = found
            .runs
            .witnesses
            .iter()
            .map(|witness| witness.id.as_str())
            .collect();
        assert_eq!(witnesses, ["second", "first"], "the order in the file");
        let ids: Vec<&str> = found.rule_ids().collect();
        assert_eq!(
            ids,
            ["low", "second", "first"],
            "the rules come before the witnesses"
        );
    }

    #[test]
    fn a_step_that_is_not_an_action_of_the_model_above_is_refused() {
        let witness = |steps: &str| {
            format!(
                "export const w = m.witness(M, \"w\", {{ steps: [{steps}], expect: (s) => s.n === 0 }});\n"
            )
        };
        one_refusal(&witness("later"), 46, "`later`");
        one_refusal(&witness("1"), 46, "`1`");
        one_refusal(&witness("bump, ...more"), 52, "`...more`");
        one_refusal(&witness("m.step(bump)"), 46, "`m.step(bump)`");
        one_refusal(&witness("m.step(later, {})"), 46, "`m.step(later, {})`");
        one_refusal(
            &witness("m.action(bump, { by: \"key\" })"),
            46,
            "`m.action(bump, { by: \"key\" })`",
        );
        one_refusal(
            &witness("bump.step(press, {})"),
            46,
            "`bump.step(press, {})`",
        );
        one_refusal(&witness("m.step(press, given)"), 60, "`given`");
    }

    #[test]
    fn a_hole_in_the_steps_is_refused() {
        let found = refused(
            "export const w = m.witness(M, \"w\", { steps: [bump, , bump], expect: (s) => s.n === 2 });\n",
        );
        assert_eq!(found.len(), 1, "one problem: {found:?}");
        let (line, _, _) = found.first().expect("one problem");
        assert_eq!(*line, 6, "the line of the witness");
    }

    #[test]
    fn a_rule_an_end_or_an_action_declared_below_is_not_a_step() {
        let source = format!(
            "{ACTIONS}export const low = m.rule(M, \"low\", (s) => s.n < 4);\n\
             export const full = m.end(M, \"full\", (s) => s.n === 3);\n\
             export const w = m.witness(M, \"w\", {{ steps: [low, full, later], expect: (s) => s.n === 0 }});\n\
             export const later = m.action(M, \"later\", {{ changes: () => ({{ n: 0 }}) }});\n"
        );
        let found: Vec<(u32, String)> = read("m.model.ts", &format!("{HEAD}{source}"))
            .problems
            .into_iter()
            .map(|problem| (problem.line, problem.feature))
            .collect();
        assert_eq!(
            found,
            [
                (8, "`low`".to_owned()),
                (8, "`full`".to_owned()),
                (8, "`later`".to_owned())
            ],
            "each step that is not an action declared above"
        );
    }

    #[test]
    fn an_action_of_another_model_is_not_a_step() {
        let source = format!(
            "{HEAD}export const N = m.model(\"N\", {{ on: m.boolean() }});\n\
             export const flip = m.action(N, \"flip\", {{ changes: (s) => ({{ on: !s.on }}) }});\n\
             export const w = m.witness(M, \"w\", {{ steps: [flip], expect: (s) => s.n === 0 }});\n"
        );
        let found: Vec<(u32, u32, String)> = read("m.model.ts", &source)
            .problems
            .into_iter()
            .map(|problem| (problem.line, problem.column, problem.feature))
            .collect();
        assert_eq!(
            found,
            [(6, 46, "`flip`".to_owned())],
            "the action of `N` in a witness of `M`"
        );
    }

    #[test]
    fn steps_that_are_not_a_list_written_out_are_refused() {
        one_refusal(
            "export const w = m.witness(M, \"w\", { steps: ALL, expect: (s) => s.n === 0 });\n",
            45,
            "`ALL`",
        );
        one_refusal(
            "export const w = m.witness(M, \"w\", { steps: [bump].concat([bump]), expect: (s) => s.n === 0 });\n",
            45,
            "`[bump].concat([bump])`",
        );
    }

    #[test]
    fn a_witness_without_its_steps_or_its_expect_or_with_another_part_is_refused() {
        one_refusal(
            "export const w = m.witness(M, \"w\", { expect: (s) => s.n === 0 });\n",
            18,
            "`m.witness`",
        );
        one_refusal(
            "export const w = m.witness(M, \"w\", { steps: [bump] });\n",
            18,
            "`m.witness`",
        );
        one_refusal(
            "export const w = m.witness(M, \"w\", { steps: [], expect: (s) => s.n === 0, until: 1 });\n",
            75,
            "`until`",
        );
        one_refusal("export const w = m.witness(M, \"w\");\n", 18, "`m.witness`");
        one_refusal(
            "export const w = m.witness(M, { steps: [], expect: (s) => s.n === 0 });\n",
            18,
            "`m.witness`",
        );
    }

    #[test]
    fn a_witness_that_is_not_exported_is_refused() {
        one_refusal(
            "const w = m.witness(M, \"w\", { steps: [bump], expect: (s) => s.n === 1 });\n",
            11,
            "`w`",
        );
    }
}
