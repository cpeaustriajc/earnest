use crate::Runs;
use crate::code;
use crate::json::{Declaration, Def, Expr as Quint, Param};
use crate::model::{action, ids, picks};
use crate::names::Short;
use crate::reader::{
    Path, Picked, READ_BEFORE_SET, Read, Reader, Refusal, Settled, all_of, constant,
    constant_of_a_run, refusal,
};
use crate::starts::{Asserts, start_states};
use earnest_ir::{
    Action, Call, Expr, Falls, Field, Pick, Plan, RETRIED_BELOW, Script, Stage, eval, truth,
};
use earnest_value::{Domain, Value};
use std::collections::BTreeSet;

const TESTS_END_WITH: &str = "Test";
const DECLARED_ON: u32 = 1;
const MOST_STEPS: usize = 1000;
const MOST_PLANS: usize = 1000;
const MOST_ALIASES: usize = 100;
const UNMAPPED: [&str; 3] = ["q::test", "q::testOnce", "q::debug"];
const CHAINS: [&str; 4] = ["then", "reps", "expect", "fail"];
const AFTER_A_FALSE_TRY: (&str, &str) = (
    "a chain tried again",
    "a chain of steps that Quint tries again after a false try, whose first step reads the \
     state: a run called after a step with a `nondet` of its own over fewer than 100 values, a \
     branch of an `any` or a part of an `and` after the first; Quint plays it again from the \
     state the false try left, after that try's steps; start it with an action that reads no \
     variable, such as `init`, or pick inside its steps",
);
const AFTER_NO_REPS: &str = "a `reps` of 0 rounds or fewer makes no step, so Quint leaves every \
     variable unset after it, and what comes next reads one: `quint test` stops the run here \
     (QNT502, \"not set\")";
const CHAIN_IN_A_STEP: &str = "a chain of steps where one step is wanted";
const JOINS: [&str; 3] = ["actionAny", "and", "or"];

type Helper<'a> = (&'a Def, &'a [Param], &'a Quint, &'a [Quint]);
type FirstStates = (Option<Vec<Vec<Value>>>, Vec<(String, Value)>);

struct Alternative<'a> {
    body: &'a Quint,
    held: Option<u64>,
    value: Value,
}

#[derive(Clone)]
struct Step {
    choices: Vec<usize>,
    asserts: Option<Expr>,
    expect: Option<Expr>,
    refused: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Last,
    Inside,
    Failing,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum First {
    #[default]
    NotRead,
    Read,
    Asserted,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Halt {
    Run,
    Call(usize),
}

#[derive(Clone, Copy, Default)]
struct Pending {
    fresh: bool,
    unset: bool,
}

#[derive(Clone, Default)]
struct Drawn {
    first: First,
    starts: Option<Vec<Vec<Value>>>,
    start: Option<Expr>,
    steps: Vec<Step>,
    must_not_finish: bool,
    ends_false: bool,
    start_falls: Vec<(String, Value)>,
    last_can_fail: bool,
    chosen: Vec<(String, Value)>,
    retried: Vec<bool>,
    within: Vec<Option<usize>>,
    calls: Vec<Call>,
    dead: Option<Halt>,
    dead_says: String,
}

pub(crate) struct Variables<'v> {
    pub(crate) every: &'v [(u64, String, Domain)],
    pub(crate) searched: &'v [u64],
}

struct Run<'r, 'a> {
    reader: &'r mut Reader<'a>,
    fields: Vec<String>,
    own: bool,
    differs: Option<String>,
    init: Option<&'a Def>,
    name: &'a str,
    actions: Vec<Action>,
    plans: Vec<Plan>,
    number: usize,
    picks: Vec<Pick>,
    nondets: bool,
    retried: Vec<bool>,
    ordered: Vec<bool>,
    scope: Option<usize>,
    pending: Pending,
    called: Option<&'a str>,
}

fn chain_in<'q>(
    expr: &'q Quint,
    holder: Option<&'q Quint>,
) -> Option<(&'q Quint, Option<&'q Quint>)> {
    match expr {
        Quint::App { opcode, .. } if CHAINS.contains(&opcode.as_str()) => Some((expr, holder)),
        Quint::App { args, .. } => args.iter().find_map(|each| chain_in(each, Some(expr))),
        Quint::Let { opdef, expr: body } => {
            chain_in(&opdef.expr, Some(expr)).or_else(|| chain_in(body, Some(expr)))
        }
        Quint::Lambda { expr: body, .. } => chain_in(body, holder),
        Quint::Name { .. } | Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } => None,
    }
}

fn unset_read(drawn: &mut [Drawn]) {
    for each in live(drawn) {
        each.calls.push(Call {
            says: "the `reps` of 0 rounds".to_owned(),
            last: Some(each.steps.len()),
            ends_the_run: false,
            reason: AFTER_NO_REPS.to_owned(),
        });
        each.dead = Some(Halt::Call(each.calls.len().saturating_sub(1)));
    }
}

fn live(drawn: &mut [Drawn]) -> impl Iterator<Item = &mut Drawn> {
    drawn.iter_mut().filter(|each| each.dead.is_none())
}

fn both(first: Option<Expr>, second: Expr) -> Expr {
    match first {
        Some(first) => all_of(vec![first, second]),
        None => second,
    }
}

fn is_assert(expr: &Quint) -> bool {
    matches!(expr, Quint::App { opcode, .. } if opcode == "assert")
}

impl<'a> Run<'_, 'a> {
    fn refuse<T>(&self, feature: &str, why: &str) -> Read<T> {
        Err(refusal(feature, &format!("{why}, in the run `{}`", self.name)).within(self.name))
    }

    fn refuse_at<T>(&self, expr: &Quint, feature: &str, why: &str) -> Read<T> {
        self.refuse(feature, why).map_err(|refused| match expr {
            Quint::App { id, .. } | Quint::Name { id, .. } => refused.at(*id),
            Quint::Let { opdef, .. } => refused.at(opdef.id),
            Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } | Quint::Lambda { .. } => {
                refused
            }
        })
    }

    fn called_run(&self, expr: &Quint) -> Option<&'a Def> {
        let Quint::Name { id, name } = expr else {
            return None;
        };
        self.reader
            .definition(*id, name)
            .ok()
            .filter(|def| def.qualifier == "run" && !matches!(def.expr, Quint::Lambda { .. }))
    }

    fn helper(&self, expr: &'a Quint, depth: usize) -> Option<Helper<'a>> {
        let (id, name, args): (u64, &str, &'a [Quint]) = match expr {
            Quint::Name { id, name } => (*id, name, &[]),
            Quint::App { id, opcode, args } if !CHAINS.contains(&opcode.as_str()) => {
                (*id, opcode, args)
            }
            _ => return None,
        };
        let def = self.reader.definition(id, name).ok()?;
        let (params, body): (&'a [Param], &'a Quint) = match &def.expr {
            Quint::Lambda { params, expr } => (params, expr),
            body => (&[], body),
        };
        (params.len() == args.len() && self.chains(body, depth))
            .then_some((def, params, body, args))
    }

    fn chains(&self, expr: &'a Quint, depth: usize) -> bool {
        match expr {
            Quint::App { opcode, .. } if CHAINS.contains(&opcode.as_str()) => true,
            Quint::App { opcode, args, .. } if opcode == "ite" => {
                args.iter().skip(1).any(|each| self.chains(each, depth))
            }
            Quint::App { opcode, args, .. } if JOINS.contains(&opcode.as_str()) => {
                args.iter().any(|each| self.chains(each, depth))
            }
            Quint::Let { expr, .. } => self.chains(expr, depth),
            _ => depth < MOST_ALIASES && self.helper(expr, depth.saturating_add(1)).is_some(),
        }
    }

    fn walk_helper(
        &mut self,
        (def, params, body, args): Helper<'a>,
        place: Place,
        drawn: &mut Vec<Drawn>,
    ) -> Read<()> {
        let mut held = Vec::with_capacity(params.len());
        for (param, arg) in params.iter().zip(args) {
            let value = self.reader.pure(arg)?;
            if constant(&value, self.reader.slots)?.is_none() {
                return self.refuse_at(
                    arg,
                    &format!("`{}`", def.name),
                    &format!(
                        "an argument of `{}`, a chain of steps, that reads the state: give it a \
                         value that is known before the run starts, such as a constant or the \
                         run's own `nondet`",
                        def.name
                    ),
                );
            }
            held.push((param.id, value));
        }
        let mut was = Vec::with_capacity(held.len());
        for (id, value) in held {
            was.push((id, self.reader.hold(id, value)));
        }
        self.called = Some(&def.name);
        let walked = self.walk(body, place, drawn);
        for (id, before) in was.into_iter().rev() {
            self.reader.release(id, before);
        }
        walked
    }

    fn push(&mut self, plan: Plan) -> Read<()> {
        if self.plans.len() >= MOST_PLANS {
            return self.refuse(
                "`nondet`",
                &format!(
                    "the `nondet`s written at the top of the run and of the runs it calls pick \
                     among more than {MOST_PLANS} combinations, and each one is played as a run of \
                     its own: \
                     pick from smaller sets, or pick inside a step, as in `nondet x = \
                     S.oneOf() act(x)`, where every value is tried too"
                ),
            );
        }
        self.plans.push(plan);
        Ok(())
    }

    fn falls(&self) -> Falls {
        if self.nondets {
            Falls::Retried {
                last_step: false,
                retried: self.retried.clone(),
                within: vec![None; self.retried.len()],
                calls: Vec::new(),
                ordered: self.ordered.clone(),
            }
        } else {
            Falls::Breaks
        }
    }

    fn without_steps(&self, chosen: &[(String, Value)]) -> Plan {
        Plan {
            chosen: chosen.to_vec(),
            start: None,
            starts: None,
            steps: Vec::new(),
            must_not_finish: false,
            falls: self.falls(),
            start_falls: Vec::new(),
        }
    }

    fn plans(&mut self, expr: &'a Quint, chosen: &mut Vec<(String, Value)>) -> Read<()> {
        if let Some(def) = self.called_run(expr) {
            return self.plans(&def.expr, chosen);
        }
        match expr {
            Quint::Let { opdef, expr } if opdef.qualifier == "nondet" => {
                let Picked::Listed(_, values) = self.reader.picked_from(opdef)? else {
                    return self.refuse(
                        &format!("`nondet {}`", opdef.name),
                        "a run's own `nondet` over a set read from the state: pick inside the \
                         step that uses it",
                    );
                };
                self.nondets = true;
                if values.is_empty() {
                    return self.push(Plan {
                        must_not_finish: true,
                        starts: Some(vec![Vec::new()]),
                        ..self.without_steps(chosen)
                    });
                }
                let retried = values.len() < RETRIED_BELOW;
                for value in values {
                    let was = self.reader.hold(opdef.id, Expr::Const(value.clone()));
                    chosen.push((opdef.name.clone(), value));
                    self.retried.push(retried);
                    self.ordered.push(false);
                    let played = self.plans(expr, chosen);
                    self.ordered.pop();
                    self.retried.pop();
                    chosen.pop();
                    self.reader.release(opdef.id, was);
                    played?;
                }
                Ok(())
            }
            Quint::Let { opdef, expr } => {
                if matches!(opdef.expr, Quint::Lambda { .. }) {
                    return self.refuse(
                        &format!("`{}`", opdef.name),
                        "an operator defined inside a run",
                    );
                }
                let value = self.reader.pure(&opdef.expr)?;
                let was = self.reader.hold(opdef.id, value);
                let played = self.plans(expr, chosen);
                self.reader.release(opdef.id, was);
                played
            }
            Quint::App { opcode, args, .. } if opcode == "ite" && self.chains(expr, 0) => {
                let branch = self.branch(expr, args)?;
                self.plans(branch, chosen)
            }
            Quint::App { opcode, args, .. }
                if opcode == "or" && args.iter().any(|each| self.chains(each, 0)) =>
            {
                self.or_of_chains(args, chosen)
            }
            Quint::App { opcode, args, .. }
                if JOINS.contains(&opcode.as_str())
                    && args.iter().any(|each| self.chains(each, 0)) =>
            {
                self.plan(expr, chosen)
            }
            Quint::App { opcode, .. }
                if matches!(opcode.as_str(), "then" | "reps" | "expect" | "fail") =>
            {
                self.plan(expr, chosen)
            }
            _ if self.reader.is_action(expr) || self.helper(expr, 0).is_some() => {
                self.plan(expr, chosen)
            }
            _ => {
                let holds = self.reader.pure(expr)?;
                let starts = Some(vec![Vec::new()]);
                match constant_of_a_run(&holds, self.reader.slots)? {
                    Some(Settled::Value(Value::Bool(holds))) => self.push(Plan {
                        must_not_finish: !holds,
                        starts,
                        ..self.without_steps(chosen)
                    }),
                    Some(Settled::Asserted) => self.push(Plan {
                        must_not_finish: true,
                        starts,
                        falls: Falls::Breaks,
                        ..self.without_steps(chosen)
                    }),
                    _ => self.refuse(
                        "a run",
                        "a run that is neither steps nor a constant boolean",
                    ),
                }
            }
        }
    }

    fn or_of_chains(&mut self, args: &'a [Quint], chosen: &mut Vec<(String, Value)>) -> Read<()> {
        self.nondets = true;
        for (part, number) in args.iter().zip(1_i64..) {
            chosen.push(("or".to_owned(), Value::Int(number)));
            self.retried.push(false);
            self.ordered.push(true);
            let played = self.plans(part, chosen);
            self.ordered.pop();
            self.retried.pop();
            chosen.pop();
            played?;
        }
        Ok(())
    }

    fn plan(&mut self, expr: &'a Quint, chosen: &[(String, Value)]) -> Read<()> {
        let mut branches = vec![Drawn::default()];
        self.walk(expr, Place::Last, &mut branches)?;
        for drawn in branches {
            let dead_run = drawn.dead == Some(Halt::Run);
            let mut calls = drawn.calls;
            if dead_run {
                calls.push(Call {
                    says: format!("the run `{}`", self.name),
                    last: Some(drawn.steps.len()),
                    ends_the_run: true,
                    reason: drawn.dead_says,
                });
            }
            let falls = if self.nondets || !drawn.chosen.is_empty() || !calls.is_empty() {
                let mut within = vec![None; self.retried.len()];
                let mut ordered = self.ordered.clone();
                ordered.resize(self.retried.len().saturating_add(drawn.within.len()), false);
                within.extend(drawn.within);
                Falls::Retried {
                    last_step: drawn.ends_false,
                    retried: self.retried.iter().copied().chain(drawn.retried).collect(),
                    within,
                    calls,
                    ordered,
                }
            } else {
                Falls::Breaks
            };
            let starts = match drawn.first {
                First::NotRead => Some(vec![Vec::new()]),
                _ => drawn.starts,
            };
            self.push(Plan {
                chosen: chosen.iter().cloned().chain(drawn.chosen).collect(),
                start: drawn.start.map(code),
                starts,
                steps: drawn
                    .steps
                    .into_iter()
                    .map(|step| Stage {
                        choices: step.choices,
                        asserts: step.asserts.map(code),
                        expect: step.expect.map(code),
                        refused: step.refused,
                    })
                    .collect(),
                must_not_finish: drawn.must_not_finish,
                falls,
                start_falls: drawn.start_falls,
            })?;
        }
        Ok(())
    }

    fn walk(&mut self, expr: &'a Quint, place: Place, drawn: &mut Vec<Drawn>) -> Read<()> {
        let called = self.called.take();
        if let Some(def) = self.called_run(expr) {
            self.called = Some(&def.name);
            return self.walk(&def.expr, place, drawn);
        }
        if let Some(helper) = self.helper(expr, 0) {
            return self.walk_helper(helper, place, drawn);
        }
        if let Some(walked) = self.around(expr, (place, called), drawn) {
            return walked;
        }
        let Quint::App { opcode, args, .. } = expr else {
            return self.leaf(expr, place, drawn);
        };
        match (opcode.as_str(), args.as_slice()) {
            ("then", [first, second]) => {
                self.walk(first, Place::Inside, drawn)?;
                self.walk(second, place, drawn)
            }
            ("reps", [count, repeated]) => self.reps(count, repeated, drawn),
            ("expect", [first, condition]) => {
                self.walk(first, Place::Inside, drawn)?;
                let holds = self.reader.pure(condition)?;
                if self.pending.unset && constant(&holds, self.reader.slots)?.is_none() {
                    self.pending.unset = false;
                    unset_read(drawn);
                    return Ok(());
                }
                for each in live(drawn) {
                    if let Some(step) = each.steps.last_mut() {
                        step.expect = Some(both(step.expect.take(), holds.clone()));
                    } else {
                        each.start = Some(both(each.start.take(), holds.clone()));
                    }
                    each.last_can_fail = false;
                }
                Ok(())
            }
            ("fail", [first]) => {
                if place != Place::Last || self.scope.is_some() {
                    return self.refuse_at(
                        expr,
                        "`fail`",
                        "a `fail()` that is not the run's last operator: write `A.fail()` only at \
                         the end of the run, as Quint judges it there",
                    );
                }
                self.walk(first, Place::Failing, drawn)?;
                for each in live(drawn) {
                    match each.steps.last_mut() {
                        Some(step) if each.last_can_fail => step.refused = true,
                        _ if each.first == First::Asserted => {}
                        _ => each.must_not_finish = true,
                    }
                    each.last_can_fail = false;
                }
                Ok(())
            }
            ("then" | "reps" | "expect" | "fail", _) => self.refuse_at(
                expr,
                &format!("`{opcode}`"),
                &format!("it is given {} arguments", args.len()),
            ),
            _ if UNMAPPED.contains(&opcode.as_str()) => self.refuse_at(
                expr,
                &format!("`{opcode}`"),
                "a run operator the reader does not map",
            ),
            _ => self.leaf(expr, place, drawn),
        }
    }

    fn around(
        &mut self,
        expr: &'a Quint,
        (place, called): (Place, Option<&'a str>),
        drawn: &mut Vec<Drawn>,
    ) -> Option<Read<()>> {
        match expr {
            Quint::Let { opdef, expr: body }
                if opdef.qualifier == "nondet" && self.chains(body, 0) =>
            {
                Some(self.scoped(opdef, body, (place, called), drawn))
            }
            Quint::Let { opdef, expr: body }
                if !matches!(opdef.expr, Quint::Lambda { .. }) && self.chains(body, 0) =>
            {
                let value = match self.reader.pure(&opdef.expr) {
                    Ok(value) => value,
                    Err(refused) => return Some(Err(refused)),
                };
                let was = self.reader.hold(opdef.id, value);
                self.called = called;
                let walked = self.walk(body, place, drawn);
                self.reader.release(opdef.id, was);
                Some(walked)
            }
            Quint::App { opcode, args, .. } if opcode == "ite" && self.chains(expr, 0) => {
                let branch = match self.branch(expr, args) {
                    Ok(branch) => branch,
                    Err(refused) => return Some(Err(refused)),
                };
                self.called = called;
                Some(self.walk(branch, place, drawn))
            }
            Quint::App { opcode, args, .. }
                if JOINS.contains(&opcode.as_str())
                    && args.iter().any(|each| self.chains(each, 0)) =>
            {
                Some(match opcode.as_str() {
                    "actionAny" => self.any_of_chains(args, place, drawn),
                    "and" => self.and_of_chains(expr, args, place, drawn),
                    _ => self.refuse_at(
                        expr,
                        "`or`",
                        "an `or` between chains of steps after the run's first step, or in a run \
                         the run calls: Quint plays the next part only where the one before is \
                         false, from the state that part left; earnest plays an `or` of chains \
                         only at the top of a run, under its `nondet`s and `if`s",
                    ),
                })
            }
            _ => None,
        }
    }

    fn branch(&mut self, expr: &'a Quint, args: &'a [Quint]) -> Read<&'a Quint> {
        let [condition, then, otherwise] = args else {
            return self.refuse_at(
                expr,
                "`if`",
                "an `if` without one condition and two branches",
            );
        };
        let tested = self.reader.pure(condition)?;
        match constant_of_a_run(&tested, self.reader.slots)? {
            Some(Settled::Value(Value::Bool(true))) => Ok(then),
            Some(Settled::Value(Value::Bool(false))) => Ok(otherwise),
            _ => self.refuse_at(
                expr,
                "`if`",
                "an `if` between chains of steps whose condition is not known before the run \
                 starts: Quint reads it from the state where the run gets to it; write the \
                 condition inside a step, as in `all { c, x' = x }`, or test only constants \
                 and the `nondet`s written above it",
            ),
        }
    }

    fn scoped(
        &mut self,
        opdef: &'a Def,
        body: &'a Quint,
        (place, called): (Place, Option<&'a str>),
        drawn: &mut Vec<Drawn>,
    ) -> Read<()> {
        let feature = format!("`nondet {}`", opdef.name);
        let says = called.map_or_else(
            || {
                format!(
                    "the `nondet {}` written around a chain of steps",
                    opdef.name
                )
            },
            |name| format!("the run `{name}`"),
        );
        let Picked::Listed(_, values) = self.reader.picked_from(opdef)? else {
            return self.refuse_at(
                &opdef.expr,
                &feature,
                &format!(
                    "{says} picks with a `nondet` over a set read from the state, before a chain \
                     of steps: pick inside the step that uses it, or from a set known before the \
                     run starts"
                ),
            );
        };
        if place == Place::Failing && values.is_empty() {
            return self.refuse_at(
                &opdef.expr,
                &feature,
                &format!(
                    "a `fail()` over {says}, which picks from an empty set before a chain of \
                     steps: pick from a set with a value, or write `fail()` inside it"
                ),
            );
        }
        let retried = values.len() < RETRIED_BELOW && place != Place::Failing;
        let bodies: Vec<Alternative<'a>> = values
            .into_iter()
            .map(|value| Alternative {
                body,
                held: Some(opdef.id),
                value,
            })
            .collect();
        self.alternatives((&opdef.name, &says), (bodies, retried), place, drawn)
    }

    fn any_of_chains(
        &mut self,
        args: &'a [Quint],
        place: Place,
        drawn: &mut Vec<Drawn>,
    ) -> Read<()> {
        let bodies = args
            .iter()
            .zip(1_i64..)
            .map(|(body, number)| Alternative {
                body,
                held: None,
                value: Value::Int(number),
            })
            .collect();
        let says = "the `any` between chains of steps".to_owned();
        let retried = place != Place::Failing;
        self.alternatives(("any", &says), (bodies, retried), place, drawn)
    }

    fn and_of_chains(
        &mut self,
        expr: &'a Quint,
        args: &'a [Quint],
        place: Place,
        drawn: &mut Vec<Drawn>,
    ) -> Read<()> {
        let ends_the_run = place == Place::Last;
        if place == Place::Failing || (ends_the_run && self.scope.is_some()) {
            return self.refuse_at(
                expr,
                "`and`",
                "an `and` between chains of steps under a `fail()`, or as the last part of a run \
                 the run calls with a `nondet` of its own: earnest plays an `and` of chains at \
                 the top of a run and before more steps",
            );
        }
        let Some((last, parts)) = args.split_last() else {
            return Ok(());
        };
        for part in parts {
            let (live, dead): (Vec<Drawn>, Vec<Drawn>) = std::mem::take(drawn)
                .into_iter()
                .partition(|each| each.dead.is_none());
            *drawn = live;
            let starting = drawn
                .first()
                .is_some_and(|each| each.first == First::NotRead);
            if starting && self.nondets && !self.chains(part, 0) {
                return self.refuse_at(
                    part,
                    "`and`",
                    "an `and` whose first part is one action, under a `nondet` at the top of the \
                     run: where that action is false the run is false, which earnest does not \
                     tell apart from a first action that gives no state; write the action as the \
                     first step of a chain, as in `init.then(...)`",
                );
            }
            let mut call = None;
            for each in drawn.iter_mut() {
                each.calls.push(Call {
                    says: "the first part of the `and`".to_owned(),
                    last: None,
                    ends_the_run,
                    reason: String::new(),
                });
                call = Some(each.calls.len().saturating_sub(1));
            }
            let outer = std::mem::replace(&mut self.scope, call);
            let walked = self.walk(part, Place::Last, drawn);
            self.scope = outer;
            walked?;
            let mut out = dead;
            Self::closed(call, std::mem::take(drawn), &mut out);
            *drawn = out;
            self.pending.fresh = true;
        }
        self.walk(last, place, drawn)
    }

    fn alternatives(
        &mut self,
        (name, says): (&str, &str),
        (bodies, retried): (Vec<Alternative<'a>>, bool),
        place: Place,
        drawn: &mut Vec<Drawn>,
    ) -> Read<()> {
        let fresh = (retried && bodies.len() > 1) || std::mem::take(&mut self.pending.fresh);
        let opens = place == Place::Inside;
        let mut out = Vec::with_capacity(drawn.len().saturating_mul(bodies.len().max(1)));
        for mut each in std::mem::take(drawn) {
            if each.dead.is_some() {
                out.push(each);
                continue;
            }
            let scope = if opens {
                each.calls.push(Call {
                    says: says.to_owned(),
                    last: None,
                    ends_the_run: false,
                    reason: String::new(),
                });
                Some(each.calls.len().saturating_sub(1))
            } else {
                self.scope
            };
            if bodies.is_empty() {
                each.dead = Some(scope.map_or(Halt::Run, Halt::Call));
                each.dead_says = format!("{says} picks from an empty set here");
                Self::closed(scope.filter(|_| opens), vec![each], &mut out);
                continue;
            }
            for Alternative { body, held, value } in &bodies {
                let mut one = each.clone();
                one.chosen.push((name.to_owned(), value.clone()));
                one.retried.push(retried);
                one.within.push(scope);
                let mut walked = vec![one];
                let was = held.map(|id| (id, self.reader.hold(id, Expr::Const(value.clone()))));
                let outer = std::mem::replace(&mut self.scope, scope);
                self.pending.fresh = fresh;
                let inner = if opens { Place::Last } else { place };
                let played = self.walk(body, inner, &mut walked);
                self.pending.fresh = false;
                self.scope = outer;
                if let Some((id, was)) = was {
                    self.reader.release(id, was);
                }
                played?;
                Self::closed(scope.filter(|_| opens), walked, &mut out);
            }
        }
        *drawn = out;
        Ok(())
    }

    fn closed(opened: Option<usize>, walked: Vec<Drawn>, out: &mut Vec<Drawn>) {
        for mut each in walked {
            if let Some(call) = opened {
                let last = if each.dead == Some(Halt::Call(call)) {
                    Some(each.steps.len())
                } else if each.last_can_fail {
                    Some(each.steps.len().saturating_sub(1))
                } else {
                    None
                };
                if let Some(found) = each.calls.get_mut(call) {
                    found.last = last;
                    if last == Some(each.steps.len()) {
                        found.reason.clone_from(&each.dead_says);
                    }
                }
                each.last_can_fail = false;
            }
            out.push(each);
        }
    }

    fn reps(&mut self, count: &'a Quint, repeated: &'a Quint, drawn: &mut Vec<Drawn>) -> Read<()> {
        let Quint::Lambda { params, expr: body } = repeated else {
            return self.refuse("`reps`", "a `reps` whose step is not written as `i => ...`");
        };
        let [index] = params.as_slice() else {
            return self.refuse("`reps`", "a `reps` whose step does not take one index");
        };
        let counted = self.reader.pure(count)?;
        let Some(Value::Int(times)) = constant(&counted, self.reader.slots)? else {
            return self.refuse(
                "`reps`",
                "a `reps` whose count is not a constant number when the file is read",
            );
        };
        if times < 1 {
            self.pending.unset = true;
            for each in live(drawn) {
                each.last_can_fail = false;
            }
            return Ok(());
        }
        if usize::try_from(times).map_or(true, |times| times > MOST_STEPS) {
            return self.refuse(
                "`reps`",
                &format!("a `reps` of more than {MOST_STEPS} steps"),
            );
        }
        for round in 0..times {
            let was = self.reader.hold(index.id, Expr::Const(Value::Int(round)));
            let walked = self.walk(body, Place::Inside, drawn);
            self.reader.release(index.id, was);
            walked?;
        }
        for each in live(drawn) {
            each.last_can_fail = false;
        }
        Ok(())
    }

    fn aliased(&self, def: &'a Def) -> &'a Def {
        let mut found = def;
        for _ in 0..MOST_ALIASES {
            let Quint::Name { id, name } = &found.expr else {
                break;
            };
            match self.reader.definition(*id, name) {
                Ok(next) if next.qualifier == "action" => found = next,
                _ => break,
            }
        }
        found
    }

    fn is_init(&self, expr: &Quint) -> bool {
        let (Quint::Name { id, name }, Some(init)) = (expr, self.init) else {
            return false;
        };
        self.reader
            .definition(*id, name)
            .is_ok_and(|def| self.aliased(def).id == self.aliased(init).id)
    }

    fn action_name(&self, expr: &Quint) -> String {
        match expr {
            Quint::Name { name, .. } => format!("`{name}`"),
            Quint::App { id, opcode, .. } if self.reader.definition(*id, opcode).is_ok() => {
                format!("`{opcode}`")
            }
            _ => "an action written in place".to_owned(),
        }
    }

    fn first_states(&mut self, expr: &'a Quint) -> Read<FirstStates> {
        let first = self.action_name(expr);
        self.reader.in_init = true;
        let paths = self.reader.paths(expr);
        self.reader.in_init = false;
        let paths = paths.map_err(|mut refused| {
            if refused.why.starts_with(READ_BEFORE_SET) {
                refused.why = format!(
                    "the run starts with {first}, which reads this variable before anything gives \
                     it a value, and Quint stops the run there (\"not set\"): start the run with \
                     an action that gives each variable a value without reading one, in the run \
                     `{}`",
                    self.name
                );
                refused.within = Some(self.name.to_owned());
                refused.id = refused.id.or(match expr {
                    Quint::App { id, .. } | Quint::Name { id, .. } => Some(*id),
                    Quint::Let { opdef, .. } => Some(opdef.id),
                    _ => None,
                });
            }
            refused
        })?;
        for path in &paths {
            let unset = self
                .fields
                .iter()
                .enumerate()
                .find(|(position, _)| !path.assigns.iter().any(|(at, _)| at == position));
            if let Some((_, field)) = unset {
                return self.refuse_at(
                    expr,
                    &format!("`{field}`"),
                    &format!(
                        "the run reads this variable, and it starts with {first}, which gives it \
                         no value, so Quint stops the run where it is read (\"not set\"): start \
                         the run with an action that assigns it too"
                    ),
                );
            }
        }
        let starts = start_states(
            (&paths, &first),
            &self.fields,
            self.reader.slots,
            Asserts::EndTheRun,
        )
        .map_err(|refused| {
            let why = format!("{}, in the run `{}`", refused.why, self.name);
            Refusal { why, ..refused }.within(self.name)
        })?;
        let falls = match &starts {
            Some(found) if !found.is_empty() => self.first_falls(&paths),
            _ => Vec::new(),
        };
        Ok((starts, falls))
    }

    fn first_falls(&self, paths: &[Path]) -> Vec<(String, Value)> {
        let slots = paths
            .iter()
            .flat_map(|path| &path.choices)
            .map(|choice| choice.slot.saturating_add(1))
            .fold(self.reader.slots, usize::max);
        let members: Vec<usize> = (0..paths.len()).collect();
        let mut frame = crate::frame(&[], slots);
        falls_for(paths, &members, &mut frame, &mut Vec::new()).unwrap_or_default()
    }

    fn leaf(&mut self, expr: &'a Quint, place: Place, drawn: &mut [Drawn]) -> Read<()> {
        let fresh = std::mem::take(&mut self.pending.fresh);
        let unset = std::mem::take(&mut self.pending.unset);
        let ends_false = place == Place::Last && self.scope.is_none();
        let Some(first) = drawn
            .iter()
            .find(|each| each.dead.is_none())
            .map(|each| each.first)
        else {
            return Ok(());
        };
        if first == First::NotRead {
            return self.first_leaf(expr, place, drawn);
        }
        if unset && !self.reads_nothing(expr) {
            unset_read(drawn);
            return Ok(());
        }
        if fresh && !self.reads_nothing(expr) {
            return self.refuse_at(expr, AFTER_A_FALSE_TRY.0, AFTER_A_FALSE_TRY.1);
        }
        let step = self
            .step(expr)
            .map_err(|refused| self.chain_inside(expr, refused))?;
        for each in live(drawn) {
            if each.steps.len() >= MOST_STEPS {
                return self.refuse("a run", &format!("a run of more than {MOST_STEPS} steps"));
            }
            each.steps.push(step.clone());
            each.ends_false = ends_false;
            each.last_can_fail = true;
        }
        Ok(())
    }

    fn chain_inside(&self, expr: &'a Quint, refused: Refusal) -> Refusal {
        if !refused.why.starts_with(CHAIN_IN_A_STEP) {
            return refused;
        }
        let Some((chain, holder)) = chain_in(expr, None) else {
            return refused;
        };
        let held = match holder {
            Some(Quint::App { opcode, .. }) => match opcode.as_str() {
                "actionAll" => "an `all { ... }`".to_owned(),
                "actionAny" => "an `any` inside a step".to_owned(),
                "ite" => "an `if` inside a step".to_owned(),
                "and" | "or" => format!("an `{opcode}` inside a step"),
                "matchVariant" => "a `match`".to_owned(),
                other => format!("the arguments of `{other}`"),
            },
            Some(Quint::Let { opdef, .. }) => format!("the step that picks `{}`", opdef.name),
            _ => "a step".to_owned(),
        };
        let name = match chain {
            Quint::App { opcode, .. } => opcode.as_str(),
            _ => "then",
        };
        let replaced = self.refuse_at::<()>(
            chain,
            &format!("`{name}`"),
            &format!(
                "a chain of steps (`{name}`) inside {held}, where Quint wants one action: a run \
                 joins chains with `then`, `reps`, `expect` and `fail`, and with `if`, `and`, \
                 `or` and `any` written around whole chains"
            ),
        );
        match replaced {
            Err(found) => found,
            Ok(()) => refused,
        }
    }

    fn reads_nothing(&mut self, expr: &'a Quint) -> bool {
        self.reader.in_init = true;
        let paths = self.reader.paths(expr);
        self.reader.in_init = false;
        !matches!(paths, Err(refused) if refused.why.starts_with(READ_BEFORE_SET))
    }

    fn first_leaf(&mut self, expr: &'a Quint, place: Place, drawn: &mut [Drawn]) -> Read<()> {
        let ends_false = place == Place::Last && self.scope.is_none();
        if self.is_init(expr) {
            if let Some(field) = &self.differs {
                return self.refuse_at(
                    expr,
                    &format!("`{field}`"),
                    "the run reads this variable, and it starts with `init`, which gives it \
                     no value, so Quint stops the run where it is read (\"not set\"): start \
                     the run with an action that assigns it",
                );
            }
            self.reader.in_init = true;
            let paths = self.reader.paths(expr);
            self.reader.in_init = false;
            let falls = match paths {
                Ok(paths) => self.first_falls(&paths),
                Err(_) => Vec::new(),
            };
            for each in live(drawn) {
                each.first = First::Read;
                each.ends_false = ends_false;
                each.last_can_fail = false;
                each.start_falls.clone_from(&falls);
            }
            return Ok(());
        }
        let (starts, falls) = self
            .first_states(expr)
            .map_err(|refused| self.chain_inside(expr, refused))?;
        for each in live(drawn) {
            each.first = if starts.is_some() {
                First::Read
            } else {
                First::Asserted
            };
            each.ends_false = ends_false && starts.is_some();
            each.last_can_fail = false;
            each.starts = Some(starts.clone().unwrap_or_default());
            each.start_falls.clone_from(&falls);
        }
        Ok(())
    }

    fn step(&mut self, expr: &'a Quint) -> Read<Step> {
        let (asserted, rest): (&[Quint], &[Quint]) = match expr {
            Quint::App { opcode, args, .. } if opcode == "actionAll" => {
                let leading = args.iter().take_while(|each| is_assert(each)).count();
                args.split_at(leading)
            }
            _ => (&[], std::slice::from_ref(expr)),
        };
        if let Some(late) = rest.iter().find(|each| is_assert(each)) {
            return self.refuse_at(
                late,
                "`assert`",
                "an `assert` after another part of the step: it is read only at the start of a \
                 step's `all { ... }`",
            );
        }
        let mut asserts = Vec::with_capacity(asserted.len());
        for each in asserted {
            let Quint::App { args, .. } = each else {
                continue;
            };
            let [condition] = args.as_slice() else {
                return self.refuse("`assert`", "an `assert` without one condition");
            };
            asserts.push(self.reader.pure(condition)?);
        }
        let paths = if asserted.is_empty() {
            self.reader.paths(expr)?
        } else {
            self.reader.all_paths(rest)?
        };
        let fallback = match rest {
            [Quint::Name { name, .. }] => name.as_str(),
            _ => self.name,
        };
        let first = self.actions.len();
        for (id, path) in ids(&paths, fallback).into_iter().zip(&paths) {
            let taken = action(id, path, &self.fields, self.reader.slots)?;
            self.picks
                .extend(picks(Some(self.number), self.actions.len(), path));
            self.actions.push(taken);
        }
        Ok(Step {
            choices: (first..self.actions.len()).collect(),
            asserts: (!asserts.is_empty()).then(|| all_of(asserts)),
            expect: None,
            refused: false,
        })
    }
}

fn values_of(path: &Path, depth: usize, frame: &mut earnest_ir::Frame<'_>) -> Option<Vec<Value>> {
    let choice = path.choices.get(depth)?;
    let Some(set) = &choice.from else {
        return Some(choice.values.clone());
    };
    match eval(set, frame).ok()? {
        Value::Set(items) => Some(items.iter().cloned().collect()),
        _ => None,
    }
}

fn falls_for(
    paths: &[Path],
    members: &[usize],
    frame: &mut earnest_ir::Frame<'_>,
    bound: &mut Vec<(String, Value)>,
) -> Option<Vec<(String, Value)>> {
    let depth = bound.len();
    let mut fell = None;
    let mut groups: Vec<(usize, Vec<Value>, Vec<usize>)> = Vec::new();
    for member in members {
        let path = paths.get(*member)?;
        let Some(choice) = path.choices.get(depth) else {
            let mut holds = true;
            for guard in &path.guards {
                match truth(guard, frame) {
                    Ok(true) => {}
                    Ok(false) => {
                        holds = false;
                        break;
                    }
                    Err(_) => return None,
                }
            }
            if holds {
                return None;
            }
            fell.get_or_insert_with(|| bound.clone());
            continue;
        };
        let values = values_of(path, depth, frame)?;
        match groups
            .iter_mut()
            .find(|(slot, listed, _)| *slot == choice.slot && *listed == values)
        {
            Some((.., found)) => found.push(*member),
            None => groups.push((choice.slot, values, vec![*member])),
        }
    }
    for (slot, values, group) in groups {
        let name = paths.get(*group.first()?)?.choices.get(depth)?.name.clone();
        let retried = values.len() < RETRIED_BELOW;
        let (mut first, mut every) = (None, true);
        if values.is_empty() {
            first = Some(bound.clone());
        }
        for value in values {
            frame.put(slot, value.clone());
            bound.push((name.clone(), value));
            let under = falls_for(paths, &group, frame, bound);
            bound.pop();
            match under {
                Some(found) => {
                    first.get_or_insert(found);
                }
                None => every = false,
            }
        }
        let falls = if retried { every } else { first.is_some() };
        match first {
            Some(found) if falls => {
                if fell
                    .as_ref()
                    .is_none_or(|kept: &Vec<_>| kept.len() < found.len())
                {
                    fell = Some(found);
                }
            }
            _ => return None,
        }
    }
    fell
}

fn own_fields(run: &mut Run<'_, '_>, touched: &[&(u64, String, Domain)]) -> Vec<Field> {
    if !run.own {
        return Vec::new();
    }
    touched
        .iter()
        .enumerate()
        .map(|(position, (_, name, domain))| {
            let mut values: Vec<Value> = run
                .plans
                .iter()
                .filter_map(|plan| plan.starts.as_ref())
                .flatten()
                .filter_map(|state| state.get(position).cloned())
                .collect();
            values.sort();
            values.dedup();
            Field {
                name: name.clone(),
                sym: run.reader.symbols.intern(name),
                domain: domain.clone(),
                starts: values,
            }
        })
        .collect()
}

fn script<'a>(
    reader: &mut Reader<'a>,
    (def, number): (&'a Def, usize),
    init: Option<&'a Def>,
    variables: &Variables<'_>,
) -> Read<(Script, Vec<Pick>)> {
    def.name.clone_into(&mut reader.within);
    let reached = reader.variables_reached(&[&def.expr]);
    let touched: Vec<&(u64, String, Domain)> = variables
        .every
        .iter()
        .filter(|(id, ..)| reached.contains(id))
        .collect();
    let searched: BTreeSet<u64> = variables.searched.iter().copied().collect();
    let differs = variables
        .every
        .iter()
        .find(|(id, ..)| reached.contains(id) && !searched.contains(id))
        .or_else(|| {
            variables
                .every
                .iter()
                .find(|(id, ..)| searched.contains(id) && !reached.contains(id))
        })
        .map(|(_, name, _)| name.clone());
    let own = differs.is_some();
    let outer = own.then(|| {
        reader.place_fields(
            touched
                .iter()
                .enumerate()
                .map(|(position, (id, ..))| (*id, position))
                .collect(),
        )
    });
    let mut run = Run {
        reader,
        fields: touched.iter().map(|(_, name, _)| name.clone()).collect(),
        own,
        differs,
        init,
        name: &def.name,
        actions: Vec::new(),
        plans: Vec::new(),
        number,
        picks: Vec::new(),
        nondets: false,
        retried: Vec::new(),
        ordered: Vec::new(),
        scope: None,
        pending: Pending::default(),
        called: None,
    };
    let played = run.plans(&def.expr, &mut Vec::new());
    if let Some(outer) = outer {
        run.reader.place_fields(outer);
    }
    played?;
    let fields = own_fields(&mut run, &touched);
    let slots = run.reader.slots;
    let picked = std::mem::take(&mut run.picks);
    let script = Script {
        id: def.name.clone(),
        says: format!("Quint run `{}`", def.name),
        line: DECLARED_ON,
        actions: run.actions,
        fields,
        plans: run.plans,
        slots,
    };
    Ok((script, picked))
}

fn not_found(name: &str, imported: &[String]) -> Refusal {
    let why = if imported.iter().any(|each| each == name) {
        "the check names this run, which is written in a module this one imports: `quint \
         compile` drops the runs of imported modules, so the checker plays them only for a \
         setup it builds itself from one `import Spec(...).*`"
    } else {
        "the check names this run, and the compiled module has no run of this name"
    };
    refusal(&format!("`run {name}`"), why).within(name)
}

fn chosen<'a>(
    (declarations, imported): (&'a [Declaration], &[String]),
    runs: &Runs,
) -> Result<Vec<&'a Def>, Vec<Refusal>> {
    let every = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Def(def) if def.qualifier == "run" => Some(def),
            _ => None,
        });
    let all: Vec<&Def> = every.collect();
    let short = Short::of(all.iter().map(|def| def.name.as_str()));
    let by_ending = matches!(runs, Runs::Tests | Runs::TestsAnd(_));
    let mut found: Vec<&Def> = all
        .iter()
        .filter(|def| by_ending && def.name.ends_with(TESTS_END_WITH))
        .copied()
        .collect();
    found.sort_by_key(|def| def.id);
    let mut missing = Vec::new();
    for name in runs.listed() {
        let named = all.iter().find(|def| {
            def.name == *name
                || short.name(&def.name) == *name
                || def.name.ends_with(&format!("::{name}"))
        });
        match named {
            Some(def) if !found.iter().any(|kept| kept.id == def.id) => found.push(*def),
            Some(_) => {}
            None => missing.push(not_found(name, imported)),
        }
    }
    if let Runs::TestsAnd(_) = runs {
        found.sort_by_key(|def| def.id);
    }
    if missing.is_empty() {
        Ok(found)
    } else {
        Err(missing)
    }
}

pub(crate) fn scripts<'a>(
    reader: &mut Reader<'a>,
    declarations: (&'a [Declaration], &[String]),
    (init, runs): (Option<&'a Def>, &Runs),
    variables: &Variables<'_>,
) -> Result<(Vec<Script>, Vec<Pick>), Vec<Refusal>> {
    let played_runs = chosen(declarations, runs)?;
    let mut played = Vec::with_capacity(played_runs.len());
    let mut picked = Vec::new();
    let mut refused = Vec::new();
    for def in played_runs {
        match script(reader, (def, played.len()), init, variables) {
            Ok((script, picks)) => {
                played.push(script);
                picked.extend(picks);
            }
            Err(refusal) => refused.push(refusal),
        }
    }
    if refused.is_empty() {
        Ok((played, picked))
    } else {
        Err(refused)
    }
}
