use crate::json::{Def, Entry, Expr as Quint};
use crate::reader::{Read, Reader};
use earnest_ir::{Code, Eventually, Expr, Fairness, Rule};
use earnest_value::Value;
use std::collections::HashMap;

const PROPERTIES: &str = "q::temporalProps";
const DECLARED_ON: u32 = 1;
const TEMPORAL: [&str; 9] = [
    "always",
    "eventually",
    "leadsTo",
    "weakFair",
    "strongFair",
    "next",
    "orKeep",
    "mustChange",
    "enabled",
];
const ONLY_WEAK_FAIRNESS: &str = "only `weakFair(A, vars)` is read as fairness";

pub(crate) struct Temporal {
    pub(crate) eventually: Vec<Eventually>,
    pub(crate) rules: Vec<Rule>,
}

pub(crate) struct Step<'s> {
    pub(crate) defs: Vec<u64>,
    pub(crate) parts: &'s HashMap<String, Vec<usize>>,
    pub(crate) fields: usize,
}

enum Goal {
    Reaches { after: Option<Expr>, reaches: Expr },
    Always(Expr),
}

struct Properties<'r, 'a> {
    reader: &'r mut Reader<'a>,
    table: &'a HashMap<String, Entry>,
    defs: &'r HashMap<u64, &'a Def>,
}

fn conjuncts(expr: &Quint) -> &[Quint] {
    match expr {
        Quint::App { opcode, args, .. } if opcode == "and" => args,
        other => std::slice::from_ref(other),
    }
}

fn property_says(name: &str, number: usize, of: usize) -> String {
    if of < 2 {
        return format!("The Quint temporal property `{name}` holds");
    }
    format!("Part {number} of the Quint temporal property `{name}` holds")
}

fn numbered(name: &str, number: usize) -> String {
    if number < 2 {
        return name.to_owned();
    }
    format!("{name}#{number}")
}

impl<'a> Properties<'_, 'a> {
    fn def_of(&self, id: u64) -> Option<&'a Def> {
        let entry = self.table.get(&id.to_string())?;
        if entry.kind != "def" {
            return None;
        }
        self.defs.get(&entry.id).copied()
    }

    fn temporal_def(&self, expr: &Quint) -> Option<&'a Def> {
        let Quint::Name { id, .. } = expr else {
            return None;
        };
        self.def_of(*id)
            .filter(|def| def.qualifier == "temporal" && !matches!(def.expr, Quint::Lambda { .. }))
    }

    fn unfolded<'e>(&self, expr: &'e Quint) -> &'e Quint
    where
        'a: 'e,
    {
        match self.temporal_def(expr) {
            Some(def) => self.unfolded(&def.expr),
            None => expr,
        }
    }

    fn first_temporal(&self, expr: &Quint, depth: usize) -> Option<String> {
        if depth > self.defs.len() {
            return None;
        }
        match expr {
            Quint::App { id, opcode, args } => {
                if TEMPORAL.contains(&opcode.as_str()) {
                    return Some(opcode.clone());
                }
                let called = self
                    .def_of(*id)
                    .and_then(|def| self.first_temporal(&def.expr, depth.saturating_add(1)));
                called.or_else(|| {
                    args.iter()
                        .find_map(|each| self.first_temporal(each, depth))
                })
            }
            Quint::Name { id, .. } => self
                .def_of(*id)
                .and_then(|def| self.first_temporal(&def.expr, depth.saturating_add(1))),
            Quint::Lambda { expr, .. } => self.first_temporal(expr, depth),
            Quint::Let { opdef, expr } => self
                .first_temporal(&opdef.expr, depth)
                .or_else(|| self.first_temporal(expr, depth)),
            Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } => None,
        }
    }

    fn state(&mut self, expr: &'a Quint, around: &str) -> Read<Expr> {
        if let Some(inner) = self.first_temporal(expr, 0) {
            let why = match (around, inner.as_str()) {
                ("eventually", "always") => {
                    "`eventually(always(…))` is not read: the checker finds runs that reach a \
                     state, not runs that stay"
                        .to_owned()
                }
                _ => format!(
                    "a temporal operator inside `{around}` is not read: only `eventually(P)`, \
                     `always(P)`, `always(eventually(P))` and leads-to over state conditions are"
                ),
            };
            return self.reader.refuse(&format!("`{inner}`"), &why);
        }
        self.reader.pure(expr)
    }

    fn property(&mut self, expr: &'a Quint, step: &Step<'_>) -> Read<(Fairness, Vec<Goal>)> {
        let expr = self.unfolded(expr);
        if let Quint::App { opcode, args, .. } = expr
            && opcode == "implies"
            && let [fair, goals] = args.as_slice()
            && self.is_fairness(fair)
        {
            let fairness = self.fairness(fair, step)?;
            return Ok((fairness, self.goals(goals)?));
        }
        Ok((Fairness::default(), self.goals(expr)?))
    }

    fn is_fairness(&self, expr: &Quint) -> bool {
        conjuncts(self.unfolded(expr)).iter().all(|each| {
            matches!(self.unfolded(each), Quint::App { opcode, .. }
                if opcode == "weakFair" || opcode == "strongFair")
        })
    }

    fn fairness(&mut self, expr: &'a Quint, step: &Step<'_>) -> Read<Fairness> {
        let mut fairness = Fairness::default();
        for each in conjuncts(self.unfolded(expr)) {
            let Quint::App { opcode, args, .. } = self.unfolded(each) else {
                return self.reader.refuse("fairness", ONLY_WEAK_FAIRNESS);
            };
            if opcode != "weakFair" {
                return self.reader.refuse(
                    &format!("`{opcode}`"),
                    &format!("{ONLY_WEAK_FAIRNESS}, not strong fairness"),
                );
            }
            let [action, vars] = args.as_slice() else {
                return self.reader.refuse("`weakFair`", ONLY_WEAK_FAIRNESS);
            };
            self.over_every_variable(vars, step.fields)?;
            match self.fair_part(action, step)? {
                None => fairness.step = true,
                Some(part) => {
                    if !fairness.actions.contains(&part) {
                        fairness.actions.push(part);
                    }
                }
            }
        }
        Ok(fairness)
    }

    fn over_every_variable(&mut self, vars: &'a Quint, fields: usize) -> Read<()> {
        let listed: &[Quint] = match vars {
            Quint::App { opcode, args, .. } if opcode == "Tup" => args,
            other => std::slice::from_ref(other),
        };
        let mut seen = vec![false; fields];
        for each in listed {
            let read = match each {
                Quint::Name { .. } => self.reader.pure(each).ok(),
                _ => None,
            };
            let Some(Expr::Field(position)) = read else {
                return self.reader.refuse(
                    "`weakFair`",
                    "fairness over something that is not a tuple of the state variables",
                );
            };
            if let Some(flag) = seen.get_mut(position) {
                *flag = true;
            }
        }
        if seen.iter().all(|flag| *flag) {
            return Ok(());
        }
        self.reader.refuse(
            "`weakFair`",
            "fairness over only some of the state variables is not read: list them all",
        )
    }

    fn fair_part(&mut self, action: &'a Quint, step: &Step<'_>) -> Read<Option<Vec<usize>>> {
        let Quint::Name { id, name } = action else {
            return self.reader.refuse(
                "`weakFair`",
                "fairness of an action given arguments is not read: name the step or one of \
                 its actions",
            );
        };
        let Some(def) = self.def_of(*id) else {
            return self.reader.refuse(
                &format!("`{name}`"),
                "fairness of something that is not the step or one of its top-level actions",
            );
        };
        if step.defs.contains(&def.id) {
            return Ok(None);
        }
        if matches!(def.expr, Quint::Lambda { .. }) {
            return self.reader.refuse(
                &format!("`{name}`"),
                "fairness of an action that takes arguments is not read",
            );
        }
        match step.parts.get(&def.name) {
            Some(part) => Ok(Some(part.clone())),
            None => self.reader.refuse(
                &format!("`{name}`"),
                "fairness of something that is not the step or one of its top-level actions",
            ),
        }
    }

    fn goals(&mut self, expr: &'a Quint) -> Read<Vec<Goal>> {
        let mut goals = Vec::new();
        for each in conjuncts(self.unfolded(expr)) {
            let each = self.unfolded(each);
            let Quint::App { opcode, args, .. } = each else {
                return self.not_a_goal(each);
            };
            let goal = match (opcode.as_str(), args.as_slice()) {
                ("and", _) => {
                    goals.extend(self.goals(each)?);
                    continue;
                }
                ("eventually", [reaches]) => Goal::Reaches {
                    after: None,
                    reaches: self.state(reaches, "eventually")?,
                },
                ("leadsTo", [after, reaches]) => Goal::Reaches {
                    after: Some(self.state(after, "leadsTo")?),
                    reaches: self.state(reaches, "leadsTo")?,
                },
                ("always", [inner]) => self.always(inner)?,
                _ => return self.not_a_goal(each),
            };
            goals.push(goal);
        }
        Ok(goals)
    }

    fn always(&mut self, inner: &'a Quint) -> Read<Goal> {
        let inner = self.unfolded(inner);
        if let Quint::App { opcode, args, .. } = inner {
            match (opcode.as_str(), args.as_slice()) {
                ("eventually", [reaches]) => {
                    return Ok(Goal::Reaches {
                        after: Some(Expr::Const(Value::Bool(true))),
                        reaches: self.state(reaches, "eventually")?,
                    });
                }
                ("implies", [after, later]) => {
                    if let Quint::App { opcode, args, .. } = self.unfolded(later)
                        && opcode == "eventually"
                        && let [reaches] = args.as_slice()
                    {
                        return Ok(Goal::Reaches {
                            after: Some(self.state(after, "always")?),
                            reaches: self.state(reaches, "eventually")?,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(Goal::Always(self.state(inner, "always")?))
    }

    fn not_a_goal<T>(&mut self, expr: &'a Quint) -> Read<T> {
        let opcode = match expr {
            Quint::App { opcode, .. } => opcode.as_str(),
            _ => "",
        };
        if let Some(inner) = self.first_temporal(expr, 0) {
            let why = if opcode == "implies" {
                "an `implies` that is not fairness `implies` what must hold is not read".to_owned()
            } else if TEMPORAL.contains(&opcode) {
                format!("`{opcode}` is not read in a temporal property")
            } else {
                format!("a temporal operator under `{opcode}` is not read")
            };
            return self.reader.refuse(&format!("`{inner}`"), &why);
        }
        self.reader.refuse(
            "a temporal property",
            "a condition outside `always` or `eventually`, which TLC judges in the start states \
             only, is not read",
        )
    }
}

pub(crate) fn temporal<'a>(
    reader: &mut Reader<'a>,
    table: &'a HashMap<String, Entry>,
    defs: &HashMap<u64, &'a Def>,
    step: &Step<'_>,
) -> Read<Temporal> {
    let mut read = Temporal {
        eventually: Vec::new(),
        rules: Vec::new(),
    };
    let Some(listed) = defs.values().find(|def| def.name == PROPERTIES) else {
        return Ok(read);
    };
    let mut properties = Properties {
        reader,
        table,
        defs,
    };
    for conjunct in conjuncts(&listed.expr) {
        let Some(def) = properties.temporal_def(conjunct) else {
            PROPERTIES.clone_into(&mut properties.reader.within);
            return properties.reader.refuse(
                "a temporal property",
                "a temporal property that is not the name of a definition",
            );
        };
        let name = &def.name;
        name.clone_into(&mut properties.reader.within);
        let (fair, goals) = properties.property(&def.expr, step)?;
        let count = goals.len();
        for (index, goal) in goals.into_iter().enumerate() {
            let number = index.saturating_add(1);
            let id = numbered(name, number);
            let says = property_says(name, number, count);
            match goal {
                Goal::Reaches { after, reaches } => read.eventually.push(Eventually {
                    id,
                    says,
                    line: DECLARED_ON,
                    after: after.map(Code::new),
                    reaches: Code::new(reaches),
                    slots: 0,
                    fair: Some(fair.clone()),
                }),
                Goal::Always(holds) => read.rules.push(Rule {
                    id,
                    says,
                    line: DECLARED_ON,
                    text: name.clone(),
                    holds: Code::new(holds),
                    slots: 0,
                }),
            }
        }
    }
    Ok(read)
}

#[cfg(test)]
mod tests;
