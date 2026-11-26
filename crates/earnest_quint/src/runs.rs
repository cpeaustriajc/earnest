use crate::json::{Declaration, Def, Expr as Quint};
use crate::model::{action, ids};
use crate::reader::{Read, Reader, Refusal, all_of, constant, refusal};
use earnest_ir::{Action, Code, Expr, Plan, Script, Stage};
use earnest_value::Value;

const TESTS_END_WITH: &str = "Test";
const DECLARED_ON: u32 = 1;
const MOST_STEPS: usize = 1000;
const MOST_PLANS: usize = 1000;
const UNMAPPED: [&str; 3] = ["q::test", "q::testOnce", "q::debug"];

struct Step {
    choices: Vec<usize>,
    asserts: Option<Expr>,
    expect: Option<Expr>,
    refused: bool,
}

#[derive(Default)]
struct Drawn {
    started: bool,
    start: Option<Expr>,
    steps: Vec<Step>,
    must_not_finish: bool,
}

struct Run<'r, 'a> {
    reader: &'r mut Reader<'a>,
    fields: &'r [String],
    init: u64,
    name: &'a str,
    actions: Vec<Action>,
    plans: Vec<Plan>,
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
        Err(refusal(
            feature,
            &format!("{why}, in the run `{}`", self.name),
        ))
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

    fn push(&mut self, plan: Plan) -> Read<()> {
        if self.plans.len() >= MOST_PLANS {
            return self.refuse(
                "`nondet`",
                &format!("the run's own choices give more than {MOST_PLANS} plans"),
            );
        }
        self.plans.push(plan);
        Ok(())
    }

    fn constant_plan(chosen: &[(String, Value)], holds: bool) -> Plan {
        Plan {
            chosen: chosen.to_vec(),
            start: None,
            steps: Vec::new(),
            must_not_finish: !holds,
        }
    }

    fn plans(&mut self, expr: &'a Quint, chosen: &mut Vec<(String, Value)>) -> Read<()> {
        if let Some(def) = self.called_run(expr) {
            return self.plans(&def.expr, chosen);
        }
        match expr {
            Quint::Let { opdef, expr } if opdef.qualifier == "nondet" => {
                let (_, values) = self.reader.picked_from(opdef)?;
                if values.is_empty() {
                    return self.push(Self::constant_plan(chosen, false));
                }
                for value in values {
                    let was = self.reader.hold(opdef.id, Expr::Const(value.clone()));
                    chosen.push((opdef.name.clone(), value));
                    let played = self.plans(expr, chosen);
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
            Quint::App { opcode, .. }
                if matches!(opcode.as_str(), "then" | "reps" | "expect" | "fail") =>
            {
                self.plan(expr, chosen)
            }
            _ if self.reader.is_action(expr) => self.plan(expr, chosen),
            _ => {
                let holds = self.reader.pure(expr)?;
                match constant(&holds)? {
                    Some(Value::Bool(holds)) => self.push(Self::constant_plan(chosen, holds)),
                    _ => self.refuse(
                        "a run",
                        "a run that is neither steps nor a constant boolean",
                    ),
                }
            }
        }
    }

    fn plan(&mut self, expr: &'a Quint, chosen: &[(String, Value)]) -> Read<()> {
        let mut drawn = Drawn::default();
        self.walk(expr, true, &mut drawn)?;
        let plan = Plan {
            chosen: chosen.to_vec(),
            start: drawn.start.map(Code::new),
            steps: drawn
                .steps
                .into_iter()
                .map(|step| Stage {
                    choices: step.choices,
                    asserts: step.asserts.map(Code::new),
                    expect: step.expect.map(Code::new),
                    refused: step.refused,
                })
                .collect(),
            must_not_finish: drawn.must_not_finish,
        };
        self.push(plan)
    }

    fn walk(&mut self, expr: &'a Quint, last: bool, drawn: &mut Drawn) -> Read<bool> {
        if let Some(def) = self.called_run(expr) {
            return self.walk(&def.expr, last, drawn);
        }
        let Quint::App { opcode, args, .. } = expr else {
            return self.leaf(expr, drawn);
        };
        match (opcode.as_str(), args.as_slice()) {
            ("then", [first, second]) => {
                self.walk(first, false, drawn)?;
                self.walk(second, last, drawn)
            }
            ("reps", [count, repeated]) => self.reps(count, repeated, drawn),
            ("expect", [first, condition]) => {
                self.walk(first, false, drawn)?;
                let holds = self.reader.pure(condition)?;
                if let Some(step) = drawn.steps.last_mut() {
                    step.expect = Some(both(step.expect.take(), holds));
                } else {
                    drawn.start = Some(both(drawn.start.take(), holds));
                }
                Ok(false)
            }
            ("fail", [first]) => {
                if !last {
                    return self.refuse("`fail`", "a `fail()` that is not the run's last operator");
                }
                let can_be_false = self.walk(first, false, drawn)?;
                match drawn.steps.last_mut() {
                    Some(step) if can_be_false => step.refused = true,
                    _ => drawn.must_not_finish = true,
                }
                Ok(false)
            }
            ("then" | "reps" | "expect" | "fail", _) => self.refuse(
                &format!("`{opcode}`"),
                &format!("it is given {} arguments", args.len()),
            ),
            _ if UNMAPPED.contains(&opcode.as_str()) => self.refuse(
                &format!("`{opcode}`"),
                "a run operator the reader does not map",
            ),
            _ => self.leaf(expr, drawn),
        }
    }

    fn reps(&mut self, count: &'a Quint, repeated: &'a Quint, drawn: &mut Drawn) -> Read<bool> {
        let Quint::Lambda { params, expr: body } = repeated else {
            return self.refuse("`reps`", "a `reps` whose step is not written as `i => ...`");
        };
        let [index] = params.as_slice() else {
            return self.refuse("`reps`", "a `reps` whose step does not take one index");
        };
        let counted = self.reader.pure(count)?;
        let Some(Value::Int(times)) = constant(&counted)? else {
            return self.refuse(
                "`reps`",
                "a `reps` whose count is not a constant number when the file is read",
            );
        };
        if times < 1 {
            return self.refuse(
                "`reps`",
                "a `reps` that repeats 0 times or fewer, after which Quint leaves the state unset",
            );
        }
        if usize::try_from(times).map_or(true, |times| times > MOST_STEPS) {
            return self.refuse(
                "`reps`",
                &format!("a `reps` of more than {MOST_STEPS} steps"),
            );
        }
        for round in 0..times {
            let was = self.reader.hold(index.id, Expr::Const(Value::Int(round)));
            let walked = self.walk(body, false, drawn);
            self.reader.release(index.id, was);
            walked?;
        }
        Ok(false)
    }

    fn is_init(&self, expr: &Quint) -> bool {
        let Quint::Name { id, name } = expr else {
            return false;
        };
        self.reader
            .definition(*id, name)
            .is_ok_and(|def| def.id == self.init)
    }

    fn leaf(&mut self, expr: &'a Quint, drawn: &mut Drawn) -> Read<bool> {
        if !drawn.started {
            if !self.is_init(expr) {
                return self.refuse(
                    "a run",
                    "a run that does not start with `init`: Quint plays a run from no state",
                );
            }
            drawn.started = true;
            return Ok(false);
        }
        if drawn.steps.len() >= MOST_STEPS {
            return self.refuse("a run", &format!("a run of more than {MOST_STEPS} steps"));
        }
        let (asserted, rest): (&[Quint], &[Quint]) = match expr {
            Quint::App { opcode, args, .. } if opcode == "actionAll" => {
                let leading = args.iter().take_while(|each| is_assert(each)).count();
                args.split_at(leading)
            }
            _ => (&[], std::slice::from_ref(expr)),
        };
        if rest.iter().any(is_assert) {
            return self.refuse(
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
        self.reader.fresh_slots();
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
            let taken = action(id, path, self.fields)?;
            self.actions.push(taken);
        }
        drawn.steps.push(Step {
            choices: (first..self.actions.len()).collect(),
            asserts: (!asserts.is_empty()).then(|| all_of(asserts)),
            expect: None,
            refused: false,
        });
        Ok(true)
    }
}

fn script<'a>(reader: &mut Reader<'a>, def: &'a Def, init: u64, fields: &[String]) -> Read<Script> {
    def.name.clone_into(&mut reader.within);
    let mut run = Run {
        reader,
        fields,
        init,
        name: &def.name,
        actions: Vec::new(),
        plans: Vec::new(),
    };
    run.plans(&def.expr, &mut Vec::new())?;
    Ok(Script {
        id: def.name.clone(),
        says: format!("The Quint run `{}` passes", def.name),
        line: DECLARED_ON,
        actions: run.actions,
        plans: run.plans,
        slots: 0,
    })
}

pub(crate) fn scripts<'a>(
    reader: &mut Reader<'a>,
    declarations: &'a [Declaration],
    init: u64,
    fields: &[String],
) -> Result<Vec<Script>, Vec<Refusal>> {
    let mut tests: Vec<&Def> = declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Def(def)
                if def.qualifier == "run" && def.name.ends_with(TESTS_END_WITH) =>
            {
                Some(def)
            }
            _ => None,
        })
        .collect();
    tests.sort_by_key(|def| def.id);
    let mut played = Vec::with_capacity(tests.len());
    let mut refused = Vec::new();
    for def in tests {
        match script(reader, def, init, fields) {
            Ok(script) => played.push(script),
            Err(refusal) => refused.push(refusal),
        }
    }
    if refused.is_empty() {
        Ok(played)
    } else {
        Err(refused)
    }
}
