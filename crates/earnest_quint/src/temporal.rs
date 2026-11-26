use crate::json::{Def, Entry, Expr as Quint};
use crate::reader::{Read, Reader, Refusal};
use earnest_ir::{Code, Eventually, Expr, Fairness, Op, Rule};
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
    pub(crate) fields: &'s [String],
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
            let listed = self.over_every_variable(vars, step.fields)?;
            match self.fair_part(action, step)? {
                None => fairness.step = true,
                Some(part) => {
                    self.assigns_every_variable(action, &listed)?;
                    if !fairness.actions.contains(&part) {
                        fairness.actions.push(part);
                    }
                }
            }
        }
        Ok(fairness)
    }

    fn named_value(&self, expr: &'a Quint) -> &'a Quint {
        let mut found = expr;
        for _ in 0..=self.defs.len() {
            let Quint::Name { id, .. } = found else {
                break;
            };
            match self.def_of(*id) {
                Some(def)
                    if def.qualifier != "action" && !matches!(def.expr, Quint::Lambda { .. }) =>
                {
                    found = &def.expr;
                }
                _ => break,
            }
        }
        found
    }

    fn over_every_variable(
        &mut self,
        vars: &'a Quint,
        fields: &[String],
    ) -> Read<Vec<(usize, &'a str)>> {
        let listed: &[Quint] = match self.named_value(vars) {
            Quint::App { opcode, args, .. } if opcode == "Tup" => args,
            other => std::slice::from_ref(other),
        };
        let mut seen = vec![false; fields.len()];
        let mut named = Vec::with_capacity(listed.len());
        for each in listed {
            let read = match each {
                Quint::Name { name, .. } => self.reader.pure(each).ok().zip(Some(name.as_str())),
                _ => None,
            };
            let Some((Expr::Field(position), name)) = read else {
                return self.reader.refuse(
                    "`weakFair`",
                    "fairness over something that is not the state variables: write \
                     `weakFair(A, vars)` with `vars` a tuple of every variable, such as \
                     `(x, y)`, or a `val` that is one",
                );
            };
            if let Some(flag) = seen.get_mut(position) {
                *flag = true;
            }
            named.push((position, name));
        }
        let missing: Vec<String> = fields
            .iter()
            .zip(&seen)
            .filter(|(_, listed)| !**listed)
            .map(|(name, _)| format!("`{name}`"))
            .collect();
        if missing.is_empty() {
            return Ok(named);
        }
        self.reader.refuse(
            "`weakFair`",
            &format!(
                "fairness over only some of the state variables is not read: the tuple leaves \
                 out {}, so list every variable",
                missing.join(", ")
            ),
        )
    }

    fn assigns_every_variable(&mut self, action: &'a Quint, listed: &[(usize, &str)]) -> Read<()> {
        let Quint::Name { id, name } = action else {
            return Ok(());
        };
        let Some(def) = self.def_of(*id) else {
            return Ok(());
        };
        for path in self.reader.paths(&def.expr)? {
            let open = listed
                .iter()
                .find(|(position, _)| path.assigns.iter().all(|(given, _)| given != position));
            if let Some((_, variable)) = open {
                return self.reader.refuse(
                    "`weakFair`",
                    &format!(
                        "`weakFair({name}, …)`, where `{name}` alone does not give `{variable}'`, \
                         is not read: Quint's TLC runs stop on such a formula. Make `{name}` \
                         assign every variable, or make the whole step fair"
                    ),
                );
            }
        }
        Ok(())
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
                ("not", [inner]) => self.negated(each, inner)?,
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

    fn negated(&mut self, whole: &'a Quint, inner: &'a Quint) -> Read<Goal> {
        let not = |holds: Expr| Expr::Apply(Op::Not, vec![holds]);
        if let Quint::App { opcode, args, .. } = self.unfolded(inner)
            && let [over] = args.as_slice()
            && self.first_temporal(over, 0).is_none()
        {
            match opcode.as_str() {
                "eventually" => return Ok(Goal::Always(not(self.reader.pure(over)?))),
                "always" => {
                    return Ok(Goal::Reaches {
                        after: None,
                        reaches: not(self.reader.pure(over)?),
                    });
                }
                _ => {}
            }
        }
        self.not_a_goal(whole)
    }

    fn not_a_goal<T>(&mut self, expr: &'a Quint) -> Read<T> {
        let opcode = match expr {
            Quint::App { opcode, .. } => opcode.as_str(),
            _ => "",
        };
        if let Some(inner) = self.first_temporal(expr, 0) {
            let why = if opcode == "implies" {
                "an `implies` that is not fairness `implies` what must hold is not read".to_owned()
            } else if opcode == "not" {
                "a temporal operator under `not` is read only as `not(eventually(P))` or \
                 `not(always(P))` over a state condition `P`"
                    .to_owned()
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
) -> Result<Temporal, Vec<Refusal>> {
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
    let mut refused = Vec::new();
    for conjunct in conjuncts(&listed.expr) {
        let Some(def) = properties.temporal_def(conjunct) else {
            PROPERTIES.clone_into(&mut properties.reader.within);
            refused.extend(
                properties
                    .reader
                    .refuse::<()>(
                        "a temporal property",
                        "a temporal property that is not the name of a definition",
                    )
                    .err(),
            );
            continue;
        };
        let name = &def.name;
        name.clone_into(&mut properties.reader.within);
        let (fair, goals) = match properties.property(&def.expr, step) {
            Ok(read) => read,
            Err(refusal) => {
                refused.push(refusal);
                continue;
            }
        };
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
    if refused.is_empty() {
        Ok(read)
    } else {
        Err(refused)
    }
}

#[cfg(test)]
mod tests;
