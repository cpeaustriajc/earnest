use crate::builtins::{Builtin, builtin, not_read};
use crate::data::data_builtin;
use crate::json::{Def, Entry, Expr as Quint};
use earnest_ir::{Expr, Fault, Frame, Op, eval};
use earnest_value::{Domain, MOST_VALUES_LISTED, Symbols, Value};
use std::collections::HashMap;

pub(crate) struct Refusal {
    pub(crate) feature: String,
    pub(crate) why: String,
}

pub(crate) type Read<T> = Result<T, Refusal>;

pub(crate) fn refusal(feature: &str, why: &str) -> Refusal {
    Refusal {
        feature: feature.to_owned(),
        why: why.to_owned(),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Choice {
    pub(crate) name: String,
    pub(crate) slot: usize,
    pub(crate) domain: Domain,
    pub(crate) values: Vec<Value>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Path {
    pub(crate) name: Option<String>,
    pub(crate) choices: Vec<Choice>,
    pub(crate) guards: Vec<Expr>,
    pub(crate) assigns: Vec<(usize, Expr)>,
}

impl Path {
    fn guarded(guard: Expr) -> Self {
        Self {
            guards: vec![guard],
            ..Self::default()
        }
    }

    fn joined(&self, other: &Self) -> Self {
        let mut both = self.clone();
        both.name = self.name.clone().or_else(|| other.name.clone());
        both.choices.extend(other.choices.iter().cloned());
        both.guards.extend(other.guards.iter().cloned());
        both.assigns.extend(other.assigns.iter().cloned());
        both
    }
}

pub(crate) const MOST_ACTIONS: usize = 1000;

enum Bound<'a> {
    Value(Expr),
    Operator(&'a Def),
}

pub(crate) struct Reader<'a> {
    table: &'a HashMap<String, Entry>,
    defs: HashMap<u64, &'a Def>,
    fields: HashMap<u64, usize>,
    bound: HashMap<u64, Bound<'a>>,
    parameters: HashMap<usize, String>,
    pub(crate) slots: usize,
    pub(crate) symbols: Symbols,
    pub(crate) in_init: bool,
    pub(crate) unnamed: Vec<u64>,
    pub(crate) within: String,
}

fn base_name(parameter: &str) -> &str {
    match parameter.rsplit_once('_') {
        Some((base, digits))
            if !base.is_empty()
                && !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => parameter,
    }
}

fn reads_state(expr: &Expr, bound: &[usize]) -> bool {
    let any = |all: &[Expr]| all.iter().any(|each| reads_state(each, bound));
    let within = |slots: &[usize], body: &Expr| {
        let inner: Vec<usize> = bound.iter().chain(slots).copied().collect();
        reads_state(body, &inner)
    };
    match expr {
        Expr::Const(_) => false,
        Expr::Local(slot) => !bound.contains(slot),
        Expr::Apply(_, arguments) | Expr::SetOf(arguments) | Expr::SeqOf(arguments) => {
            any(arguments)
        }
        Expr::And(first, second) | Expr::Or(first, second) => {
            reads_state(first, bound) || reads_state(second, bound)
        }
        Expr::Choose(test, then, otherwise) => {
            reads_state(test, bound) || reads_state(then, bound) || reads_state(otherwise, bound)
        }
        Expr::Record(fields) => fields.iter().any(|(_, each)| reads_state(each, bound)),
        Expr::Get(inner, _) => reads_state(inner, bound),
        Expr::With(base, changes) => {
            reads_state(base, bound) || changes.iter().any(|(_, each)| reads_state(each, bound))
        }
        Expr::MapOf(entries) => entries
            .iter()
            .any(|(key, value)| reads_state(key, bound) || reads_state(value, bound)),
        Expr::Bind { slot, value, body } => reads_state(value, bound) || within(&[*slot], body),
        Expr::Sweep {
            slot, source, body, ..
        } => reads_state(source, bound) || within(&[*slot], body),
        Expr::Fold {
            source,
            start,
            total,
            item,
            body,
        } => {
            reads_state(source, bound)
                || reads_state(start, bound)
                || within(&[*total, *item], body)
        }
        _ => true,
    }
}

fn fold(exprs: Vec<Expr>, empty: Value, join: fn(Box<Expr>, Box<Expr>) -> Expr) -> Expr {
    exprs
        .into_iter()
        .reduce(|first, second| join(Box::new(first), Box::new(second)))
        .unwrap_or(Expr::Const(empty))
}

pub(crate) fn all_of(exprs: Vec<Expr>) -> Expr {
    fold(exprs, Value::Bool(true), Expr::And)
}

fn fault_refusal(fault: Fault) -> Refusal {
    refusal(
        "a constant",
        &format!(
            "working out a value when the file is read {}",
            fault.describe()
        ),
    )
}

pub(crate) fn constant(expr: &Expr, slots: usize) -> Read<Option<Value>> {
    if reads_state(expr, &[]) {
        return Ok(None);
    }
    eval(expr, &mut Frame::new(&[], slots))
        .map(Some)
        .map_err(fault_refusal)
}

fn too_many_actions() -> Refusal {
    refusal(
        "`step`",
        &format!(
            "the step unfolds into more than {MOST_ACTIONS} actions, more than the checker takes \
             from one Quint step"
        ),
    )
}

impl<'a> Reader<'a> {
    pub(crate) fn new(
        table: &'a HashMap<String, Entry>,
        defs: HashMap<u64, &'a Def>,
        fields: HashMap<u64, usize>,
        symbols: Symbols,
    ) -> Self {
        Self {
            table,
            defs,
            fields,
            bound: HashMap::new(),
            parameters: HashMap::new(),
            slots: 0,
            symbols,
            in_init: false,
            unnamed: Vec::new(),
            within: String::new(),
        }
    }

    pub(crate) fn top(&self, name: &str) -> Option<&'a Def> {
        self.defs.values().find(|def| def.name == name).copied()
    }

    pub(crate) fn refuse<T>(&self, feature: &str, why: &str) -> Read<T> {
        Err(refusal(
            feature,
            &format!("{why}, in the definition `{}`", self.within),
        ))
    }

    pub(crate) fn entry(&self, id: u64, name: &str) -> Read<&'a Entry> {
        match self.table.get(&id.to_string()) {
            Some(entry) => Ok(entry),
            None => self.refuse(
                &format!("`{name}`"),
                "the compiled file does not say what this name is",
            ),
        }
    }

    pub(crate) fn operator(&self, entry: &Entry, name: &str) -> Read<&'a Def> {
        if let Some(Bound::Operator(def)) = self.bound.get(&entry.id) {
            return Ok(def);
        }
        match self.defs.get(&entry.id) {
            Some(def) if entry.kind == "def" => Ok(def),
            _ => self.refuse(
                &format!("`{name}`"),
                "the compiled file does not hold this definition",
            ),
        }
    }

    fn enter<T>(&mut self, def: &Def, read: impl FnOnce(&mut Self) -> Read<T>) -> Read<T> {
        let top = self.defs.contains_key(&def.id);
        let outer = if top {
            std::mem::replace(&mut self.within, def.name.clone())
        } else {
            self.within.clone()
        };
        let result = read(self);
        self.within = outer;
        result
    }

    fn with_bound<T>(
        &mut self,
        id: u64,
        bound: Bound<'a>,
        read: impl FnOnce(&mut Self) -> Read<T>,
    ) -> Read<T> {
        let outer = self.bound.insert(id, bound);
        let result = read(self);
        match outer {
            Some(was) => self.bound.insert(id, was),
            None => self.bound.remove(&id),
        };
        result
    }

    pub(crate) fn with_values<T>(
        &mut self,
        values: Vec<(u64, Expr)>,
        read: impl FnOnce(&mut Self) -> Read<T>,
    ) -> Read<T> {
        let outer: Vec<(u64, Option<Bound<'a>>)> = values
            .into_iter()
            .map(|(id, value)| (id, self.bound.insert(id, Bound::Value(value))))
            .collect();
        let result = read(self);
        for (id, was) in outer.into_iter().rev() {
            match was {
                Some(was) => self.bound.insert(id, was),
                None => self.bound.remove(&id),
            };
        }
        result
    }

    fn with_arguments<T>(
        &mut self,
        def: &'a Def,
        arguments: Vec<Expr>,
        read: impl FnOnce(&mut Self, &'a Quint) -> Read<T>,
    ) -> Read<T> {
        let (parameters, body): (&[_], &Quint) = match &def.expr {
            Quint::Lambda { params, expr } => (params, expr),
            body => (&[], body),
        };
        if parameters.len() != arguments.len() {
            return self.refuse(
                &format!("`{}`", def.name),
                &format!(
                    "it takes {} arguments and is given {}",
                    parameters.len(),
                    arguments.len()
                ),
            );
        }
        let mut outer = Vec::with_capacity(parameters.len());
        for (parameter, argument) in parameters.iter().zip(arguments) {
            if def.qualifier == "action"
                && let Expr::Local(slot) = argument
            {
                self.parameters
                    .insert(slot, base_name(&parameter.name).to_owned());
            }
            outer.push((
                parameter.id,
                self.bound.insert(parameter.id, Bound::Value(argument)),
            ));
        }
        let result = self.enter(def, |reader| read(reader, body));
        for (id, was) in outer {
            match was {
                Some(was) => self.bound.insert(id, was),
                None => self.bound.remove(&id),
            };
        }
        result
    }

    fn arguments(&mut self, args: &'a [Quint]) -> Read<Vec<Expr>> {
        args.iter().map(|each| self.pure(each)).collect()
    }

    fn local<T>(&mut self, opdef: &'a Def, read: impl FnOnce(&mut Self) -> Read<T>) -> Read<T> {
        match (opdef.qualifier.as_str(), &opdef.expr) {
            ("nondet", _) => self.refuse(
                &format!("`nondet {}`", opdef.name),
                "a `nondet` is read only where an action picks a value",
            ),
            (_, Quint::Lambda { .. }) => self.with_bound(opdef.id, Bound::Operator(opdef), read),
            (_, body) => {
                let value = self.pure(body)?;
                self.with_bound(opdef.id, Bound::Value(value), read)
            }
        }
    }

    pub(crate) fn pure(&mut self, expr: &'a Quint) -> Read<Expr> {
        match expr {
            Quint::Int { value } => match value.as_i64() {
                Some(number) => Ok(Expr::Const(Value::Int(number))),
                None => self.refuse(
                    &format!("`{value}`"),
                    "an integer beyond 64 bits, which the checker does not hold",
                ),
            },
            Quint::Bool { value } => Ok(Expr::Const(Value::Bool(*value))),
            Quint::Str { value } => Ok(Expr::Const(Value::Str(self.text(value)))),
            Quint::Lambda { .. } => self.refuse(
                "an operator as a value",
                "an operator passed as a value is not read yet",
            ),
            Quint::Let { opdef, expr } => self.local(opdef, |reader| reader.pure(expr)),
            Quint::Name { id, name } => self.name(*id, name),
            Quint::App { id, opcode, args } => self.apply(*id, opcode, args),
        }
    }

    fn name(&mut self, id: u64, name: &str) -> Read<Expr> {
        let entry = self.entry(id, name)?;
        match entry.kind.as_str() {
            "var" => match self.fields.get(&entry.id) {
                Some(_) if self.in_init => self.refuse(
                    &format!("`{name}`"),
                    "`init` reads a variable before it has a value",
                ),
                Some(position) => Ok(Expr::Field(*position)),
                None => self.refuse(&format!("`{name}`"), "an unknown variable"),
            },
            "const" => self.refuse(
                &format!("`{name}`"),
                "a constant without a value: compile the module that gives it one",
            ),
            _ => {
                if let Some(Bound::Value(value)) = self.bound.get(&entry.id) {
                    return Ok(value.clone());
                }
                let def = self.operator(entry, name)?;
                if def.qualifier == "action" {
                    return self
                        .refuse(&format!("`{name}`"), "an action used inside an expression");
                }
                if matches!(def.expr, Quint::Lambda { .. }) {
                    return self.refuse(&format!("`{name}`"), "an operator used as a value");
                }
                self.with_arguments(def, Vec::new(), Self::pure)
            }
        }
    }

    fn apply(&mut self, id: u64, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        if let Some(data) = data_builtin(opcode) {
            return self.data(data, opcode, args);
        }
        let Some(found) = builtin(opcode) else {
            if let Some(why) = not_read(opcode) {
                return self.refuse(&format!("`{opcode}`"), why);
            }
            let entry = self.entry(id, opcode)?;
            let def = self.operator(entry, opcode)?;
            if def.qualifier == "action" {
                return self.refuse(
                    &format!("`{opcode}`"),
                    "an action used inside an expression",
                );
            }
            let arguments = self.arguments(args)?;
            return self.with_arguments(def, arguments, Self::pure);
        };
        let mut arguments = self.arguments(args)?;
        let built = match (found, arguments.len()) {
            (Builtin::Unary(op), 1) | (Builtin::Binary(op), 2) => Expr::Apply(op, arguments),
            (Builtin::And, _) => all_of(arguments),
            (Builtin::Or, _) => fold(arguments, Value::Bool(false), Expr::Or),
            (Builtin::Implies, 2) => {
                let second = arguments.pop().unwrap_or(Expr::Const(Value::Bool(true)));
                let first = arguments.pop().unwrap_or(Expr::Const(Value::Bool(true)));
                Expr::Or(
                    Box::new(Expr::Apply(Op::Not, vec![first])),
                    Box::new(second),
                )
            }
            (Builtin::Iff, 2) => Expr::Apply(Op::Equal, arguments),
            (Builtin::Ite, 3) => {
                let otherwise = arguments.pop().unwrap_or(Expr::Const(Value::Null));
                let then = arguments.pop().unwrap_or(Expr::Const(Value::Null));
                let test = arguments.pop().unwrap_or(Expr::Const(Value::Null));
                Expr::Choose(Box::new(test), Box::new(then), Box::new(otherwise))
            }
            (Builtin::ActionAll | Builtin::ActionAny | Builtin::Assign, _) => {
                return self.refuse(
                    &format!("`{opcode}`"),
                    "an action used inside an expression",
                );
            }
            (Builtin::OneOf, _) => {
                return self.refuse(
                    "`oneOf`",
                    "`oneOf` is read only as the value of a `nondet` in an action",
                );
            }
            _ => {
                return self.refuse(
                    &format!("`{opcode}`"),
                    &format!("it is given {} arguments", args.len()),
                );
            }
        };
        Ok(built)
    }

    fn is_action(&self, expr: &Quint) -> bool {
        match expr {
            Quint::App { id, opcode, args } => match builtin(opcode) {
                Some(Builtin::ActionAll | Builtin::ActionAny | Builtin::Assign) => true,
                Some(Builtin::And | Builtin::Or) => args.iter().any(|each| self.is_action(each)),
                Some(Builtin::Ite) => args.iter().skip(1).any(|each| self.is_action(each)),
                Some(_) => false,
                None => self.is_action_name(*id),
            },
            Quint::Name { id, .. } => self.is_action_name(*id),
            Quint::Let { opdef, expr } => opdef.qualifier == "nondet" || self.is_action(expr),
            Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } | Quint::Lambda { .. } => {
                false
            }
        }
    }

    fn is_action_name(&self, id: u64) -> bool {
        let Some(entry) = self.table.get(&id.to_string()) else {
            return false;
        };
        let def = match self.bound.get(&entry.id) {
            Some(Bound::Operator(def)) => Some(*def),
            Some(Bound::Value(_)) => None,
            None => self.defs.get(&entry.id).copied(),
        };
        def.is_some_and(|found| found.qualifier == "action")
    }

    fn checked(paths: Vec<Path>) -> Read<Vec<Path>> {
        if paths.len() > MOST_ACTIONS {
            return Err(too_many_actions());
        }
        Ok(paths)
    }

    pub(crate) fn paths(&mut self, expr: &'a Quint) -> Read<Vec<Path>> {
        if !self.is_action(expr) {
            return Ok(vec![Path::guarded(self.pure(expr)?)]);
        }
        match expr {
            Quint::App { id, opcode, args } => match builtin(opcode) {
                Some(Builtin::ActionAll | Builtin::And) => {
                    let mut joined = vec![Path::default()];
                    for each in args {
                        let next = self.paths(each)?;
                        if joined.len().saturating_mul(next.len()) > MOST_ACTIONS {
                            return Err(too_many_actions());
                        }
                        joined = joined
                            .iter()
                            .flat_map(|first| next.iter().map(|second| first.joined(second)))
                            .collect();
                    }
                    Ok(joined)
                }
                Some(Builtin::ActionAny | Builtin::Or) => {
                    let mut all = Vec::new();
                    for each in args {
                        all.extend(self.paths(each)?);
                        if all.len() > MOST_ACTIONS {
                            return Err(too_many_actions());
                        }
                    }
                    Ok(all)
                }
                Some(Builtin::Assign) => self.assign(args),
                Some(Builtin::Ite) => self.branches(args),
                _ => {
                    let entry = self.entry(*id, opcode)?;
                    let def = self.operator(entry, opcode)?;
                    let arguments = self.arguments(args)?;
                    self.action(def, arguments)
                }
            },
            Quint::Name { id, name } => {
                let entry = self.entry(*id, name)?;
                let def = self.operator(entry, name)?;
                self.action(def, Vec::new())
            }
            Quint::Let { opdef, expr } if opdef.qualifier == "nondet" => self.nondet(opdef, expr),
            Quint::Let { opdef, expr } => self.local(opdef, |reader| reader.paths(expr)),
            Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } | Quint::Lambda { .. } => {
                Ok(vec![Path::guarded(self.pure(expr)?)])
            }
        }
    }

    fn action(&mut self, def: &'a Def, arguments: Vec<Expr>) -> Read<Vec<Path>> {
        let mut paths = self.with_arguments(def, arguments, Self::paths)?;
        if !self.unnamed.contains(&def.id) {
            for path in &mut paths {
                path.name = Some(def.name.clone());
            }
        }
        Self::checked(paths)
    }

    fn assign(&mut self, args: &'a [Quint]) -> Read<Vec<Path>> {
        let [Quint::Name { id, name }, value] = args else {
            return self.refuse(
                "`assign`",
                "an assignment to something that is not a variable",
            );
        };
        let entry = self.entry(*id, name)?;
        let Some(position) = self.fields.get(&entry.id).copied() else {
            return self.refuse(
                &format!("`{name}`"),
                "an assignment to something that is not a variable",
            );
        };
        let value = self.pure(value)?;
        Ok(vec![Path {
            assigns: vec![(position, value)],
            ..Path::default()
        }])
    }

    fn branches(&mut self, args: &'a [Quint]) -> Read<Vec<Path>> {
        let [test, then, otherwise] = args else {
            return self.refuse("`if`", "an `if` without both branches");
        };
        let test = self.pure(test)?;
        let mut all = Vec::new();
        for (branch, holds) in [
            (then, test.clone()),
            (otherwise, Expr::Apply(Op::Not, vec![test])),
        ] {
            for mut path in self.paths(branch)? {
                path.guards.insert(0, holds.clone());
                all.push(path);
            }
        }
        Self::checked(all)
    }

    fn picked_from(&mut self, opdef: &'a Def) -> Read<(Domain, Vec<Value>)> {
        let feature = format!("`nondet {}`", opdef.name);
        let Quint::App { opcode, args, .. } = &opdef.expr else {
            return self.refuse(&feature, "a `nondet` whose value is not `oneOf` a set");
        };
        let [source] = args.as_slice() else {
            return self.refuse(&feature, "a `nondet` whose value is not `oneOf` a set");
        };
        if builtin(opcode) != Some(Builtin::OneOf) {
            return self.refuse(&feature, "a `nondet` whose value is not `oneOf` a set");
        }
        let computed = "`oneOf` over a set computed from the state or from another `nondet`";
        if let Quint::App { opcode, args, .. } = source
            && builtin(opcode) == Some(Builtin::Range)
        {
            let ends = self.arguments(args)?;
            let mut numbers = Vec::with_capacity(2);
            for each in &ends {
                match constant(each, self.slots)? {
                    Some(Value::Int(number)) => numbers.push(number),
                    _ => return self.refuse(&feature, computed),
                }
            }
            let [lo, hi] = numbers.as_slice() else {
                return self.refuse("`to`", "a range without both ends");
            };
            let domain = Domain::Int { lo: *lo, hi: *hi };
            if domain.count() > MOST_VALUES_LISTED {
                return self.refuse(
                    &feature,
                    &format!("a range of more than {MOST_VALUES_LISTED} values"),
                );
            }
            let values = domain.values();
            return Ok((domain, values));
        }
        let set = self.pure(source)?;
        match constant(&set, self.slots)? {
            Some(Value::Set(items)) => {
                if u128::try_from(items.len()).unwrap_or(u128::MAX) > MOST_VALUES_LISTED {
                    return self.refuse(
                        &feature,
                        &format!("a set of more than {MOST_VALUES_LISTED} values"),
                    );
                }
                let values: Vec<Value> = items.iter().cloned().collect();
                Ok((Domain::OneOf(values.clone()), values))
            }
            Some(_) => self.refuse(&feature, "a `nondet` whose value is not `oneOf` a set"),
            None => self.refuse(&feature, computed),
        }
    }

    fn nondet(&mut self, opdef: &'a Def, body: &'a Quint) -> Read<Vec<Path>> {
        let (domain, values) = self.picked_from(opdef)?;
        let slot = self.slots;
        self.slots += 1;
        let mut paths = self.with_bound(opdef.id, Bound::Value(Expr::Local(slot)), |reader| {
            reader.paths(body)
        })?;
        let name = self
            .parameters
            .get(&slot)
            .cloned()
            .unwrap_or_else(|| opdef.name.clone());
        let choice = Choice {
            name,
            slot,
            domain,
            values,
        };
        for path in &mut paths {
            path.choices.insert(0, choice.clone());
        }
        Ok(paths)
    }
}
