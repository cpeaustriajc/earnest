use crate::builtins::{Builtin, builtin, not_read};
use crate::data::{ANY_OTHER_TAG, TAG, VALUE, data_builtin};
use crate::json::{Compiled, Def, Entry, Expr as Quint, Param, Scheme};
use crate::reach::variables_reached;
use earnest_ir::{Expr, Fault, Frame, Op, eval};
use earnest_value::{Domain, MOST_VALUES_LISTED, Symbols, Value};
use std::collections::{BTreeSet, HashMap};

pub(crate) struct Refusal {
    pub(crate) feature: String,
    pub(crate) why: String,
    pub(crate) id: Option<u64>,
    pub(crate) within: Option<String>,
}

impl Refusal {
    pub(crate) fn at(self, id: u64) -> Self {
        Self {
            id: Some(id),
            ..self
        }
    }

    pub(crate) fn within(self, name: &str) -> Self {
        Self {
            within: Some(name.to_owned()),
            ..self
        }
    }
}

pub(crate) type Read<T> = Result<T, Refusal>;

pub(crate) fn refusal(feature: &str, why: &str) -> Refusal {
    Refusal {
        feature: feature.to_owned(),
        why: why.to_owned(),
        id: None,
        within: None,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Choice {
    pub(crate) name: String,
    pub(crate) slot: usize,
    pub(crate) domain: Domain,
    pub(crate) values: Vec<Value>,
    pub(crate) from: Option<Expr>,
}

pub(crate) enum Picked {
    Listed(Domain, Vec<Value>),
    Computed(Expr),
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
const MATCH: &str = "matchVariant";
const ALL_ACTION: &str = "an `all { … }` with an assignment in it is an action, not a condition";
const ANY_ACTION: &str = "an `any { … }` with an assignment in it is an action, not a condition";
const ASSIGNMENT: &str = "an assignment is an action, not a condition";
const ACTION_AS_A_VALUE: &str = ", and it stands where Quint wants a value, such as a `val`, a \
                                 `def`, an invariant or a guard: an action is read as a part of \
                                 `init`, `step` or a run, of `all`, `any`, `if` or a `match`, or \
                                 as the argument of an action; write the condition without the \
                                 assignment, or move the assignment into an action";
const NAMED_ACTION: &str = "an action, used where Quint wants a value, such as a `val`, a `def`, \
                            an invariant or a guard: an action is read as a part of `init`, \
                            `step` or a run, of `all`, `any`, `if` or a `match`, or as the \
                            argument of an action; for a condition, write a `def` or a `val`";
const DEBUG: &str = "q::debug";
const GENERATED: &str = "q::";
pub(crate) const READ_BEFORE_SET: &str = "`init` reads a variable before it has a value";
const ACTION: &str = "action";
const LAMBDA: &str = "def";
const OPERATOR_AS_VALUE: &str = "an operator used as a value, which the checker does not hold: \
                                 call it, or give it to a definition or a builtin that calls it";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Acting {
    AllAndAny,
    Assignments,
}

#[derive(Clone)]
pub(crate) enum Bound<'a> {
    Value(Expr),
    Operator(&'a Def),
    Lambda(&'a [Param], &'a Quint),
    Action(&'a Quint),
}

const BOOLEANS: &str = "Bool";
const ENDLESS: [(&str, &str); 2] = [
    ("Nat", "the set of every integer from 0 up"),
    ("Int", "the set of every integer"),
];

pub(crate) fn endless(name: &str) -> Option<&'static str> {
    ENDLESS
        .iter()
        .find(|(each, _)| *each == name)
        .map(|(_, what)| *what)
}

fn written_out(value: &serde_json::Number) -> String {
    value
        .as_f64()
        .filter(|number| number.is_finite())
        .map_or_else(|| value.to_string(), |number| format!("{number:.0}"))
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
    trail: Vec<String>,
    types: &'a HashMap<String, Scheme>,
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
            ..
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

pub(crate) fn too_many_actions() -> Refusal {
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
        compiled: &'a Compiled,
        defs: HashMap<u64, &'a Def>,
        fields: HashMap<u64, usize>,
        symbols: Symbols,
    ) -> Self {
        Self {
            table: &compiled.table,
            defs,
            fields,
            bound: HashMap::new(),
            parameters: HashMap::new(),
            slots: 0,
            symbols,
            in_init: false,
            unnamed: Vec::new(),
            within: String::new(),
            trail: Vec::new(),
            types: &compiled.types,
        }
    }

    pub(crate) fn top(&self, name: &str) -> Option<&'a Def> {
        self.defs.values().find(|def| def.name == name).copied()
    }

    pub(crate) fn refuse<T>(&self, feature: &str, why: &str) -> Read<T> {
        let shown: Vec<String> = self
            .trail
            .iter()
            .filter(|name| {
                !name.is_empty() && !name.starts_with(GENERATED) && **name != self.within
            })
            .map(|name| format!("`{name}`"))
            .collect();
        let reached = match shown.split_first() {
            None => String::new(),
            Some((first, [])) => format!(", which {first} uses"),
            Some((first, through)) => {
                format!(", which {first} reaches through {}", through.join(", "))
            }
        };
        Err(refusal(
            feature,
            &format!("{why}, in the definition `{}`{reached}", self.within),
        )
        .within(&self.within))
    }

    pub(crate) fn type_of(&self, id: u64) -> Option<&'a Scheme> {
        self.types.get(&id.to_string())
    }

    pub(crate) fn variables_reached(&self, roots: &[&Quint]) -> BTreeSet<u64> {
        variables_reached(self.table, &self.defs, roots)
    }

    pub(crate) fn place_fields(&mut self, fields: HashMap<u64, usize>) -> HashMap<u64, usize> {
        std::mem::replace(&mut self.fields, fields)
    }

    pub(crate) fn variable_id(&self, position: usize) -> Option<u64> {
        self.fields
            .iter()
            .find(|(_, each)| **each == position)
            .map(|(id, _)| *id)
    }

    pub(crate) fn entry(&self, id: u64, name: &str) -> Read<&'a Entry> {
        match self.table.get(&id.to_string()) {
            Some(entry) => Ok(entry),
            None => self.refuse(
                &format!("`{name}`"),
                "the reader does not know this name: it is not a definition of the compiled \
                 module, nor a builtin operator of Quint that earnest reads",
            ),
        }
    }

    fn builtin_set(&self, name: &str) -> Read<Expr> {
        match endless(name) {
            None => Ok(Expr::Const(Value::set(
                [Value::Bool(false), Value::Bool(true)]
                    .into_iter()
                    .collect(),
            ))),
            Some(what) => self.refuse(
                &format!("`{name}`"),
                &format!(
                    "`{name}` is {what}, a set without end, which the checker cannot hold: it is \
                     read only in `x.in({name})`, `S.contains(x)` and `S.subseteq({name})`; \
                     elsewhere write a range such as `0.to(9)`"
                ),
            ),
        }
    }

    pub(crate) fn is_builtin_set(&self, id: u64, name: &str) -> bool {
        !self.table.contains_key(&id.to_string()) && (name == BOOLEANS || endless(name).is_some())
    }

    pub(crate) fn operator(&self, entry: &Entry, name: &str) -> Read<&'a Def> {
        match self.bound.get(&entry.id) {
            Some(Bound::Operator(def)) => return Ok(def),
            Some(Bound::Lambda(..)) => {
                return self.refuse(
                    &format!("`{name}`"),
                    "an operator given as an argument, used where a definition of the module \
                     is read",
                );
            }
            _ => {}
        }
        match self.defs.get(&entry.id) {
            Some(def) if entry.kind == "def" => Ok(def),
            _ => self.refuse(
                &format!("`{name}`"),
                "the compiled file does not hold this definition",
            ),
        }
    }

    pub(crate) fn definition(&self, id: u64, name: &str) -> Read<&'a Def> {
        let entry = self.entry(id, name)?;
        self.operator(entry, name)
    }

    pub(crate) fn hold(&mut self, id: u64, value: Expr) -> Option<Bound<'a>> {
        self.bound.insert(id, Bound::Value(value))
    }

    pub(crate) fn release(&mut self, id: u64, was: Option<Bound<'a>>) {
        match was {
            Some(was) => self.bound.insert(id, was),
            None => self.bound.remove(&id),
        };
    }

    fn enter<T>(&mut self, def: &Def, read: impl FnOnce(&mut Self) -> Read<T>) -> Read<T> {
        let top = self.defs.contains_key(&def.id);
        let outer = if top {
            self.trail.push(self.within.clone());
            std::mem::replace(&mut self.within, def.name.clone())
        } else {
            self.within.clone()
        };
        let result = read(self);
        if top {
            self.trail.pop();
        }
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

    fn bind_parameters(
        &mut self,
        (name, qualifier): (&str, &str),
        parameters: &'a [Param],
        arguments: Vec<Bound<'a>>,
    ) -> Read<Vec<(u64, Option<Bound<'a>>)>> {
        if parameters.len() != arguments.len() {
            return self.refuse(
                &format!("`{name}`"),
                &format!(
                    "it takes {} arguments and is given {}",
                    parameters.len(),
                    arguments.len()
                ),
            );
        }
        let mut outer = Vec::with_capacity(parameters.len());
        for (parameter, argument) in parameters.iter().zip(arguments) {
            if qualifier == ACTION
                && let Bound::Value(Expr::Local(slot)) = argument
            {
                self.parameters
                    .insert(slot, base_name(&parameter.name).to_owned());
            }
            outer.push((parameter.id, self.bound.insert(parameter.id, argument)));
        }
        Ok(outer)
    }

    fn unbind(&mut self, outer: Vec<(u64, Option<Bound<'a>>)>) {
        for (id, was) in outer {
            match was {
                Some(was) => self.bound.insert(id, was),
                None => self.bound.remove(&id),
            };
        }
    }

    fn with_arguments<T>(
        &mut self,
        def: &'a Def,
        arguments: Vec<Bound<'a>>,
        read: impl FnOnce(&mut Self, &'a Quint) -> Read<T>,
    ) -> Read<T> {
        let (parameters, body): (&[_], &Quint) = match &def.expr {
            Quint::Lambda { params, expr } => (params, expr),
            body => (&[], body),
        };
        let outer = self.bind_parameters((&def.name, &def.qualifier), parameters, arguments)?;
        let result = self.enter(def, |reader| read(reader, body));
        self.unbind(outer);
        result
    }

    fn called_lambda(
        &mut self,
        name: &str,
        (parameters, body): (&'a [Param], &'a Quint),
        args: &'a [Quint],
    ) -> Read<Expr> {
        let arguments = self.passed(args)?;
        let outer = self.bind_parameters((name, LAMBDA), parameters, arguments)?;
        let result = self.pure(body);
        self.unbind(outer);
        result
    }

    fn called_action(
        &mut self,
        name: &str,
        (parameters, body): (&'a [Param], &'a Quint),
        args: &'a [Quint],
    ) -> Read<Vec<Path>> {
        let arguments = self.passed(args)?;
        let outer = self.bind_parameters((name, LAMBDA), parameters, arguments)?;
        let result = self.paths(body);
        self.unbind(outer);
        result
    }

    fn passed(&mut self, args: &'a [Quint]) -> Read<Vec<Bound<'a>>> {
        args.iter()
            .map(|each| match each {
                Quint::Lambda { params, expr } => Ok(Bound::Lambda(params, expr)),
                Quint::Name { id, .. } => match self.callable(*id) {
                    Some(operator) => Ok(operator),
                    None if self.is_action(each) => Ok(Bound::Action(each)),
                    None => self.pure(each).map(Bound::Value),
                },
                _ if self.is_action(each) => Ok(Bound::Action(each)),
                _ => self.pure(each).map(Bound::Value),
            })
            .collect()
    }

    fn callable(&self, id: u64) -> Option<Bound<'a>> {
        let entry = self.table.get(&id.to_string())?;
        match self.bound.get(&entry.id) {
            Some(Bound::Value(_)) => None,
            Some(operator) => Some(operator.clone()),
            None => {
                let def = self.defs.get(&entry.id).copied()?;
                (entry.kind == "def"
                    && def.qualifier != "action"
                    && matches!(def.expr, Quint::Lambda { .. }))
                .then_some(Bound::Operator(def))
            }
        }
    }

    pub(crate) fn given_lambda(&self, id: u64) -> Option<(&'a [Param], &'a Quint)> {
        let entry = self.table.get(&id.to_string())?;
        match self.bound.get(&entry.id) {
            Some(Bound::Lambda(parameters, body)) => Some((*parameters, *body)),
            _ => None,
        }
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
                    &format!("`{}`", written_out(value)),
                    "an integer beyond 64 bits, which the checker does not hold: it holds \
                     integers from -2^63 to 2^63 - 1, so write a smaller number",
                ),
            },
            Quint::Bool { value } => Ok(Expr::Const(Value::Bool(*value))),
            Quint::Str { value } => Ok(Expr::Const(Value::Str(self.text(value)))),
            Quint::Lambda { .. } => self.refuse(
                "an operator as a value",
                "an operator kept as a value, which the checker does not hold: an operator is \
                 read where it is called, or given as an argument to a definition or a builtin \
                 such as `map` or `filter`",
            ),
            Quint::Let { opdef, expr } => self.local(opdef, |reader| reader.pure(expr)),
            Quint::Name { id, name } => self.name(*id, name),
            Quint::App { id, opcode, args } => self.apply(*id, opcode, args),
        }
    }

    fn name(&mut self, id: u64, name: &str) -> Read<Expr> {
        if self.is_builtin_set(id, name) {
            return self.builtin_set(name);
        }
        let entry = self.entry(id, name)?;
        match entry.kind.as_str() {
            "var" => match self.fields.get(&entry.id) {
                Some(_) if self.in_init => self.refuse(&format!("`{name}`"), READ_BEFORE_SET),
                Some(position) => Ok(Expr::Field(*position)),
                None => self.refuse(&format!("`{name}`"), "an unknown variable"),
            },
            "const" => self.refuse(
                &format!("`{name}`"),
                "a constant without a value: compile the module that gives it one",
            ),
            _ => {
                match self.bound.get(&entry.id) {
                    Some(Bound::Value(value)) => return Ok(value.clone()),
                    Some(Bound::Action(given)) => return self.pure(given),
                    Some(Bound::Lambda(..)) => {
                        return self.refuse(&format!("`{name}`"), OPERATOR_AS_VALUE);
                    }
                    _ => {}
                }
                let def = self.operator(entry, name)?;
                if def.qualifier == "action" {
                    return self.refuse(&format!("`{name}`"), NAMED_ACTION);
                }
                if matches!(def.expr, Quint::Lambda { .. }) {
                    return self.refuse(&format!("`{name}`"), OPERATOR_AS_VALUE);
                }
                self.with_arguments(def, Vec::new(), Self::pure)
            }
        }
    }

    fn apply(&mut self, id: u64, opcode: &str, args: &'a [Quint]) -> Read<Expr> {
        if let Some(data) = data_builtin(opcode) {
            return self.data(data, opcode, args);
        }
        if opcode == DEBUG
            && let [_, shown] = args
        {
            return self.pure(shown);
        }
        let Some(found) = builtin(opcode) else {
            if let Some(why) = not_read(opcode) {
                return self
                    .refuse(&format!("`{opcode}`"), why)
                    .map_err(|refused| refused.at(id));
            }
            if let Some(lambda) = self.given_lambda(id) {
                return self.called_lambda(opcode, lambda, args);
            }
            let entry = self.entry(id, opcode)?;
            let def = self.operator(entry, opcode)?;
            if def.qualifier == "action" {
                return self.refuse(&format!("`{opcode}`"), NAMED_ACTION);
            }
            let arguments = self.passed(args)?;
            return self.with_arguments(def, arguments, Self::pure);
        };
        if let Some(refused) = self.action_as_a_value(found, args) {
            return Err(refused);
        }
        let mut arguments = self.arguments(args)?;
        let built = match (found, arguments.len()) {
            (Builtin::Unary(op), 1) | (Builtin::Binary(op), 2) => Expr::Apply(op, arguments),
            (Builtin::And | Builtin::ActionAll, _) => all_of(arguments),
            (Builtin::Or | Builtin::ActionAny, _) => fold(arguments, Value::Bool(false), Expr::Or),
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

    fn action_as_a_value(&self, found: Builtin, args: &[Quint]) -> Option<Refusal> {
        let (written, what) = match (found, args) {
            (Builtin::ActionAll | Builtin::ActionAny, _)
                if !args.iter().any(|each| self.assigns(each)) =>
            {
                return None;
            }
            (Builtin::ActionAll, _) => ("`all { … }`".to_owned(), ALL_ACTION),
            (Builtin::ActionAny, _) => ("`any { … }`".to_owned(), ANY_ACTION),
            (Builtin::Assign, [Quint::Name { name, .. }, ..]) => {
                (format!("`{}' = …`", base_name(name)), ASSIGNMENT)
            }
            (Builtin::Assign, _) => ("an assignment".to_owned(), ASSIGNMENT),
            _ => return None,
        };
        self.refuse::<()>(&written, &format!("{what}{ACTION_AS_A_VALUE}"))
            .err()
    }

    fn arguments(&mut self, args: &'a [Quint]) -> Read<Vec<Expr>> {
        args.iter().map(|each| self.pure(each)).collect()
    }

    pub(crate) fn is_action(&self, expr: &Quint) -> bool {
        self.acts(expr, Acting::AllAndAny)
    }

    fn assigns(&self, expr: &Quint) -> bool {
        self.acts(expr, Acting::Assignments)
    }

    fn acts(&self, expr: &Quint, counted: Acting) -> bool {
        match expr {
            Quint::App { opcode, args, .. } if opcode == MATCH => {
                args.iter().skip(1).any(|case| match case {
                    Quint::Lambda { expr, .. } => self.acts(expr, counted),
                    _ => self.acts(case, counted),
                })
            }
            Quint::App { id, opcode, args } => match builtin(opcode) {
                Some(Builtin::Assign) => true,
                Some(Builtin::ActionAll | Builtin::ActionAny) if counted == Acting::AllAndAny => {
                    true
                }
                Some(Builtin::ActionAll | Builtin::ActionAny | Builtin::And | Builtin::Or) => {
                    args.iter().any(|each| self.acts(each, counted))
                }
                Some(Builtin::Ite) => args.iter().skip(1).any(|each| self.acts(each, counted)),
                Some(_) => false,
                None => self.is_action_name(*id),
            },
            Quint::Name { id, .. } => self.is_action_name(*id),
            Quint::Let { opdef, expr } => opdef.qualifier == "nondet" || self.acts(expr, counted),
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
            Some(Bound::Action(_)) => return true,
            Some(Bound::Lambda(_, body)) => return self.is_action(body),
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
            Quint::App { opcode, args, .. } if opcode == MATCH => self.cases(args),
            Quint::App { id, opcode, args } => match builtin(opcode) {
                Some(Builtin::ActionAll | Builtin::And) => self.all_paths(args),
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
                    if let Some(lambda) = self.given_lambda(*id) {
                        return self.called_action(opcode, lambda, args);
                    }
                    let entry = self.entry(*id, opcode)?;
                    let def = self.operator(entry, opcode)?;
                    let arguments = self.passed(args)?;
                    self.action(def, arguments)
                }
            },
            Quint::Name { id, name } => {
                let entry = self.entry(*id, name)?;
                if let Some(Bound::Action(given)) = self.bound.get(&entry.id) {
                    return self.paths(given);
                }
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

    pub(crate) fn alternatives(&mut self, def: &'a Def) -> Vec<(&'a Quint, Read<Vec<Path>>)> {
        self.enter(def, |reader| {
            let alias = match &def.expr {
                Quint::Name { name, .. } => reader
                    .top(name)
                    .filter(|found| found.qualifier == ACTION && found.id != def.id),
                _ => None,
            };
            Ok(match (alias, &def.expr) {
                (Some(alias), _) => reader.alternatives(alias),
                (None, Quint::App { opcode, args, .. })
                    if builtin(opcode) == Some(Builtin::ActionAny) =>
                {
                    args.iter().map(|each| (each, reader.paths(each))).collect()
                }
                (None, body) => vec![(body, reader.paths(body))],
            })
        })
        .unwrap_or_default()
    }

    pub(crate) fn all_paths(&mut self, args: &'a [Quint]) -> Read<Vec<Path>> {
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

    fn action(&mut self, def: &'a Def, arguments: Vec<Bound<'a>>) -> Read<Vec<Path>> {
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
            let written = base_name(name);
            return self.refuse(
                &format!("`{written}' = …`"),
                &format!(
                    "an assignment to `{written}`, which here is not a state variable: a \
                     parameter, `nondet` or `val` of that name hides the variable, and Quint \
                     assigns that instead, so rename it"
                ),
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

    fn cases(&mut self, args: &'a [Quint]) -> Read<Vec<Path>> {
        let Some((subject, cases)) = args.split_first() else {
            return self.refuse("`match`", "a `match` without a value");
        };
        let subject = self.pure(subject)?;
        let (tag_name, value_name) = (self.text(TAG), self.text(VALUE));
        let tag = Expr::Get(Box::new(subject.clone()), tag_name);
        let value = Expr::Get(Box::new(subject), value_name);
        let mut earlier: Vec<Expr> = Vec::new();
        let mut all = Vec::new();
        for case in cases.chunks(2) {
            let [
                Quint::Str { value: name },
                Quint::Lambda { params, expr: body },
            ] = case
            else {
                return self.refuse(
                    "`match`",
                    "a case that is not a tag and `value => ...`, as `quint compile` writes one",
                );
            };
            let [bound] = params.as_slice() else {
                return self.refuse("`match`", "a case that binds more than one value");
            };
            let others_fail = earlier
                .iter()
                .map(|each| Expr::Apply(Op::Differ, vec![tag.clone(), each.clone()]))
                .collect();
            let holds = if name == ANY_OTHER_TAG {
                all_of(others_fail)
            } else {
                let this = Expr::Const(Value::Str(self.text(name)));
                if earlier.contains(&this) {
                    continue;
                }
                earlier.push(this.clone());
                Expr::Apply(Op::Equal, vec![tag.clone(), this])
            };
            let paths =
                self.with_values(vec![(bound.id, value.clone())], |reader| reader.paths(body))?;
            for mut path in paths {
                path.guards.insert(0, holds.clone());
                all.push(path);
            }
            if all.len() > MOST_ACTIONS {
                return Err(too_many_actions());
            }
            if name == ANY_OTHER_TAG {
                break;
            }
        }
        Ok(all)
    }

    pub(crate) fn picked_from(&mut self, opdef: &'a Def) -> Read<Picked> {
        let id = match &opdef.expr {
            Quint::App { id, .. } => *id,
            _ => opdef.id,
        };
        self.picked_from_set(opdef)
            .map_err(|refused| match refused.id {
                Some(_) => refused,
                None => refused.at(id),
            })
    }

    fn picked_from_set(&mut self, opdef: &'a Def) -> Read<Picked> {
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
        if let Quint::Name { name, .. } = source {
            match name.as_str() {
                "Bool" => {
                    let values = vec![Value::Bool(false), Value::Bool(true)];
                    return Ok(Picked::Listed(Domain::Bool, values));
                }
                "Int" | "Nat" => {
                    return self.refuse(
                        &feature,
                        &format!(
                            "`{name}.oneOf()` picks among numbers without end, which the checker \
                             cannot list: pick from a range such as `0.to(9)`"
                        ),
                    );
                }
                _ => {}
            }
        }
        if let Quint::App { opcode, args, .. } = source
            && builtin(opcode) == Some(Builtin::Range)
        {
            let ends = self.arguments(args)?;
            let mut numbers = Vec::with_capacity(2);
            for each in &ends {
                match constant(each, self.slots)? {
                    Some(Value::Int(number)) => numbers.push(number),
                    Some(_) => {
                        return self.refuse("`to`", "a range whose ends are not integers");
                    }
                    None => {
                        return Ok(Picked::Computed(Expr::Apply(Op::Interval, ends)));
                    }
                }
            }
            let [lo, hi] = numbers.as_slice() else {
                return self.refuse("`to`", "a range without both ends");
            };
            let domain = Domain::Int { lo: *lo, hi: *hi };
            if domain.count() > MOST_VALUES_LISTED {
                return self.refuse(
                    &feature,
                    &format!(
                        "a range of more than {MOST_VALUES_LISTED} values, more than the checker \
                         tries in one step: pick from a smaller range"
                    ),
                );
            }
            let values = domain.values();
            return Ok(Picked::Listed(domain, values));
        }
        let set = self.pure(source)?;
        match constant(&set, self.slots)? {
            Some(Value::Set(items)) => {
                if u128::try_from(items.len()).unwrap_or(u128::MAX) > MOST_VALUES_LISTED {
                    return self.refuse(
                        &feature,
                        &format!(
                            "a set of more than {MOST_VALUES_LISTED} values, more than the \
                             checker tries in one step: pick from a smaller set"
                        ),
                    );
                }
                let listed = Expr::Apply(
                    Op::InTlcOrder,
                    vec![Expr::Const(Value::Set(std::sync::Arc::clone(&items)))],
                );
                let values = match constant(&listed, self.slots)? {
                    Some(Value::Seq(values)) => values.to_vec(),
                    _ => items.iter().cloned().collect(),
                };
                Ok(Picked::Listed(Domain::OneOf(values.clone()), values))
            }
            Some(_) => self.refuse(&feature, "a `nondet` whose value is not `oneOf` a set"),
            None => Ok(Picked::Computed(set)),
        }
    }

    fn nondet(&mut self, opdef: &'a Def, body: &'a Quint) -> Read<Vec<Path>> {
        let (domain, values, from) = match self.picked_from(opdef)? {
            Picked::Listed(domain, values) => (domain, values, None),
            Picked::Computed(set) => (Domain::Any, Vec::new(), Some(set)),
        };
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
            from,
        };
        for path in &mut paths {
            path.choices.insert(0, choice.clone());
        }
        Ok(paths)
    }
}
