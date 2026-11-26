use crate::json::{Def, Entry, Expr as Quint, Param};
use crate::reader::{Choice, Path, Read, Reader, Refusal, all_of, constant, reads_state};
use earnest_ir::{
    Action, Alone, Code, Eventually, Expr, FairStep, Fairness, Frame, Input, Op, Pick, Rule, truth,
};
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
const MOST_ITEMS: usize = 1000;
const INFIX: [(&str, &str); 15] = [
    ("iadd", "+"),
    ("isub", "-"),
    ("imul", "*"),
    ("idiv", "/"),
    ("imod", "%"),
    ("ipow", "^"),
    ("eq", "=="),
    ("neq", "!="),
    ("ilt", "<"),
    ("ilte", "<="),
    ("igt", ">"),
    ("igte", ">="),
    ("and", "and"),
    ("or", "or"),
    ("implies", "implies"),
];
const ONLY_WEAK_FAIRNESS: &str = "only `weakFair(A, vars)` is read as fairness";

pub(crate) struct Temporal {
    pub(crate) eventually: Vec<Eventually>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) fair_steps: Vec<FairStep>,
}

type Named<'a> = Vec<(usize, &'a str)>;

enum Fair<'a> {
    Step,
    Actions {
        def: &'a Def,
        given: Vec<Value>,
        part: Vec<(usize, Vec<(usize, Value)>)>,
    },
}

pub(crate) struct Step<'s> {
    pub(crate) defs: Vec<u64>,
    pub(crate) parts: &'s HashMap<String, Vec<usize>>,
    pub(crate) fields: &'s [String],
    pub(crate) paths: &'s [Path],
    pub(crate) starts: &'s [Vec<Value>],
}

enum Goal {
    Reaches { after: Option<Expr>, reaches: Expr },
    Always(Expr),
    Initially(Expr),
}

struct Properties<'r, 'a> {
    reader: &'r mut Reader<'a>,
    table: &'a HashMap<String, Entry>,
    defs: &'r HashMap<u64, &'a Def>,
    fair_steps: Vec<FairStep>,
}

fn conjuncts(expr: &Quint) -> &[Quint] {
    match expr {
        Quint::App { opcode, args, .. } if opcode == "and" => args,
        other => std::slice::from_ref(other),
    }
}

fn written(expr: &Quint) -> String {
    match expr {
        Quint::Name { name, .. } => name.clone(),
        Quint::Int { value } => value.to_string(),
        Quint::Bool { value } => value.to_string(),
        Quint::Str { value } => format!("\"{value}\""),
        Quint::App { opcode, args, .. } => {
            let each: Vec<String> = args
                .iter()
                .map(|arg| match arg {
                    Quint::App { opcode, args, .. }
                        if args.len() == 2 && INFIX.iter().any(|(name, _)| name == opcode) =>
                    {
                        format!("({})", written(arg))
                    }
                    _ => written(arg),
                })
                .collect();
            let sign = INFIX
                .iter()
                .find(|(name, _)| name == opcode)
                .map(|(_, sign)| *sign);
            match (opcode.as_str(), sign, args.as_slice(), each.as_slice()) {
                (_, Some(sign), [_, _], [left, right]) => format!("{left} {sign} {right}"),
                ("item", _, [_, Quint::Int { value }], [tuple, _]) => format!("{tuple}._{value}"),
                ("field", _, [_, Quint::Str { value }], [record, _]) => format!("{record}.{value}"),
                ("Tup", _, _, _) => format!("({})", each.join(", ")),
                ("Rec", _, _, _) => {
                    let fields: Vec<String> = each
                        .chunks(2)
                        .map(|pair| match pair {
                            [key, value] => format!("{}: {value}", key.trim_matches('"')),
                            _ => pair.join(""),
                        })
                        .collect();
                    format!("{{ {} }}", fields.join(", "))
                }
                ("iuminus", _, _, [inner]) => format!("-{inner}"),
                ("not", _, _, [inner]) => format!("not({inner})"),
                (_, _, _, [first, rest @ ..]) if opcode.starts_with(char::is_lowercase) => {
                    format!("{first}.{opcode}({})", rest.join(", "))
                }
                _ => format!("{opcode}({})", each.join(", ")),
            }
        }
        Quint::Lambda { .. } | Quint::Let { .. } => "…".to_owned(),
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

    fn property(
        &mut self,
        expr: &'a Quint,
        step: &Step<'_>,
        fair: Option<&Fairness>,
        parts: &mut Vec<(Fairness, Goal)>,
    ) -> Read<()> {
        let expr = self.unfolded(expr);
        if let Quint::App { opcode, args, .. } = expr {
            match (opcode.as_str(), args.as_slice()) {
                ("and", _) => {
                    for each in args {
                        self.property(each, step, fair, parts)?;
                    }
                    return Ok(());
                }
                ("implies", [given, goals]) if self.is_fairness(given) => {
                    let mut fairness = fair.cloned().unwrap_or_default();
                    self.fairness(given, step, &mut fairness)?;
                    return self.property(goals, step, Some(&fairness), parts);
                }
                ("implies", [given, goals])
                    if self.first_temporal(given, 0).is_none()
                        && self.first_temporal(goals, 0).is_some() =>
                {
                    if self.holds_at_the_start(given, step)? {
                        return self.property(goals, step, fair, parts);
                    }
                    return Ok(());
                }
                ("forall", [set, Quint::Lambda { params, expr: body }])
                    if self.first_temporal(body, 0).is_some() =>
                {
                    if let Some(items) = self.items_of(set, params)? {
                        for (id, item) in items {
                            let was = self.reader.hold(id, Expr::Const(item));
                            let read = self.property(body, step, fair, parts);
                            self.reader.release(id, was);
                            read?;
                        }
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        let goal = self.goal(expr)?;
        parts.push((fair.cloned().unwrap_or_default(), goal));
        Ok(())
    }

    fn holds_at_the_start(&mut self, given: &'a Quint, step: &Step<'_>) -> Read<bool> {
        let condition = self.reader.pure(given)?;
        let mut found = Vec::with_capacity(step.starts.len());
        for state in step.starts {
            let mut frame = Frame::new(state, self.reader.slots);
            match truth(&condition, &mut frame) {
                Ok(holds) => found.push(holds),
                Err(fault) => {
                    return self.reader.refuse(
                        "`implies`",
                        &format!(
                            "the condition before `implies`, judged in the start states as TLC \
                             judges it, {}",
                            fault.describe()
                        ),
                    );
                }
            }
        }
        if found.iter().all(|holds| *holds) {
            return Ok(true);
        }
        if found.iter().all(|holds| !*holds) {
            return Ok(false);
        }
        let what = written(given);
        self.reader.refuse(
            "`implies`",
            &format!(
                "`{what}` before `implies` is judged in the start states only, as TLC judges it, \
                 and it holds in some start states and not in others, which the checker does not \
                 read: write `always(({what}) implies eventually(…))` to judge it in every \
                 state, or give `init` the start states it is about"
            ),
        )
    }

    fn items_of(&mut self, set: &'a Quint, params: &[Param]) -> Read<Option<Vec<(u64, Value)>>> {
        let [param] = params else {
            return Ok(None);
        };
        let read = self.reader.pure(set)?;
        let Some(Value::Set(items)) = constant(&read, self.reader.slots)? else {
            return Ok(None);
        };
        if items.len() > MOST_ITEMS {
            return self.reader.refuse(
                "`forall`",
                &format!(
                    "a `forall` over more than {MOST_ITEMS} items in a temporal property, more \
                     than the checker reads as one property each"
                ),
            );
        }
        Ok(Some(
            items.iter().map(|item| (param.id, item.clone())).collect(),
        ))
    }

    fn is_fairness(&self, expr: &Quint) -> bool {
        match self.unfolded(expr) {
            Quint::App { opcode, args, .. } => match (opcode.as_str(), args.as_slice()) {
                ("weakFair" | "strongFair", _) => true,
                ("and", _) => !args.is_empty() && args.iter().all(|each| self.is_fairness(each)),
                ("forall" | "exists", [_, Quint::Lambda { expr, .. }]) => self.is_fairness(expr),
                _ => false,
            },
            _ => false,
        }
    }

    fn fairness(&mut self, expr: &'a Quint, step: &Step<'_>, fairness: &mut Fairness) -> Read<()> {
        let Quint::App { opcode, args, .. } = self.unfolded(expr) else {
            return self.reader.refuse("fairness", ONLY_WEAK_FAIRNESS);
        };
        match (opcode.as_str(), args.as_slice()) {
            ("and", _) => {
                for each in args {
                    self.fairness(each, step, fairness)?;
                }
                return Ok(());
            }
            ("forall", [set, Quint::Lambda { params, expr: body }]) => {
                let Some(items) = self.items_of(set, params)? else {
                    return self.reader.refuse(
                        "`forall`",
                        "fairness under a `forall` over a set that changes with the state is \
                         not read: the set must have a value without the state",
                    );
                };
                for (id, item) in items {
                    let was = self.reader.hold(id, Expr::Const(item));
                    let read = self.fairness(body, step, fairness);
                    self.reader.release(id, was);
                    read?;
                }
                return Ok(());
            }
            ("exists", _) => {
                return self.reader.refuse(
                    "`exists`",
                    "fairness under `exists` is not read: a run must then be fair to one item \
                     the run does not name. Write `forall`, or the fairness of each item",
                );
            }
            ("weakFair", _) => {}
            _ => {
                return self.reader.refuse(
                    &format!("`{opcode}`"),
                    &format!("{ONLY_WEAK_FAIRNESS}, not strong fairness"),
                );
            }
        }
        let [action, vars] = args.as_slice() else {
            return self.reader.refuse("`weakFair`", ONLY_WEAK_FAIRNESS);
        };
        let (listed, over) = self.variables_of(vars, step.fields)?;
        let part = match self.fair_part(action, step)? {
            Fair::Step if over.is_empty() => {
                fairness.step = true;
                return Ok(());
            }
            Fair::Step => (0..step.paths.len())
                .map(|action| FairStep {
                    action,
                    inputs: Vec::new(),
                    over: over.clone(),
                    alone: None,
                })
                .collect(),
            Fair::Actions { def, given, part } => {
                let paths = self.assigns_every_variable(def, &given, &listed)?;
                vec![FairStep {
                    action: part.first().map_or(0, |(action, _)| *action),
                    inputs: Vec::new(),
                    over,
                    alone: Some(self.alone(def, &given, &paths, step.fields.len())),
                }]
            }
        };
        let actions = step.paths.len();
        let part: Vec<usize> = part
            .into_iter()
            .map(|wanted| self.fair_step(wanted, actions))
            .collect();
        if !fairness.actions.contains(&part) {
            fairness.actions.push(part);
        }
        Ok(())
    }

    fn fair_step(&mut self, wanted: FairStep, actions: usize) -> usize {
        let known = self.fair_steps.iter().position(|known| *known == wanted);
        let position = known.unwrap_or_else(|| {
            self.fair_steps.push(wanted);
            self.fair_steps.len().saturating_sub(1)
        });
        actions.saturating_add(position)
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

    fn variables_of(
        &mut self,
        vars: &'a Quint,
        fields: &[String],
    ) -> Read<(Named<'a>, Vec<usize>)> {
        let mut named = Vec::new();
        self.mentioned(vars, &mut named, 0)?;
        let mut over: Vec<usize> = Vec::with_capacity(named.len());
        for (position, _) in &named {
            if !over.contains(position) {
                over.push(*position);
            }
        }
        if over.len() == fields.len() {
            over.clear();
        }
        Ok((named, over))
    }

    fn mentioned(
        &mut self,
        expr: &'a Quint,
        named: &mut Vec<(usize, &'a str)>,
        depth: usize,
    ) -> Read<()> {
        let expr = self.named_value(expr);
        let parts: &'a [Quint] = match expr {
            Quint::App { opcode, args, .. } => match (opcode.as_str(), args.as_slice()) {
                ("Tup" | "List", _) | ("Set", [_]) => args,
                ("Rec", _) => {
                    for value in args.iter().skip(1).step_by(2) {
                        self.mentioned(value, named, depth.saturating_add(1))?;
                    }
                    return Ok(());
                }
                _ => return self.not_one_to_one(expr),
            },
            Quint::Name { name, .. } => {
                if let Ok(Expr::Field(position)) = self.reader.pure(expr) {
                    named.push((position, name.as_str()));
                    return Ok(());
                }
                return self.not_one_to_one(expr);
            }
            _ => return self.not_one_to_one(expr),
        };
        if depth > self.defs.len() {
            return self.not_one_to_one(expr);
        }
        for each in parts {
            self.mentioned(each, named, depth.saturating_add(1))?;
        }
        Ok(())
    }

    fn not_one_to_one<T>(&mut self, over: &Quint) -> Read<T> {
        let what = written(over);
        self.reader.refuse(
            "`weakFair`",
            &format!(
                "fairness over `{what}`, which is not the state variables and can stay the same \
                 while one of them changes, is not read: write `weakFair(A, vars)` with `vars` \
                 a tuple of the variables, such as `(x, y)`, or a `val` that is one"
            ),
        )
    }

    fn alone(&self, def: &Def, given: &[Value], paths: &[Path], fields: usize) -> Alone {
        let shown: Vec<String> = given
            .iter()
            .map(|value| self.reader.symbols.show(value))
            .collect();
        let mut picks = Vec::new();
        let actions = paths
            .iter()
            .enumerate()
            .map(|(position, path)| {
                let mut updates = vec![None; fields];
                for (field, expr) in &path.assigns {
                    if let Some(slot @ None) = updates.get_mut(*field) {
                        *slot = Some(Code::new(expr.clone()));
                    }
                }
                for (input, choice) in path.choices.iter().enumerate() {
                    if let Some(from) = &choice.from {
                        picks.push(Pick {
                            script: None,
                            action: position,
                            input,
                            from: Code::new(from.clone()),
                        });
                    }
                }
                Action {
                    id: def.name.clone(),
                    label: def.name.clone(),
                    line: DECLARED_ON,
                    inputs: path
                        .choices
                        .iter()
                        .map(|choice| Input {
                            name: choice.name.clone(),
                            slot: choice.slot,
                            domain: choice.domain.clone(),
                            values: choice.values.clone(),
                        })
                        .collect(),
                    guard: Code::new(all_of(path.guards.clone())),
                    updates,
                    slots: self.reader.slots,
                }
            })
            .collect();
        Alone {
            named: format!("{}({})", def.name, shown.join(", ")),
            actions,
            picks,
        }
    }

    fn assigns_every_variable(
        &mut self,
        def: &'a Def,
        given: &[Value],
        listed: &[(usize, &str)],
    ) -> Read<Vec<Path>> {
        let name = &def.name;
        let paths = match &def.expr {
            Quint::Lambda { params, expr } => {
                let values = params
                    .iter()
                    .zip(given)
                    .map(|(param, value)| (param.id, Expr::Const(value.clone())))
                    .collect();
                self.reader
                    .with_values(values, |reader| reader.paths(expr))?
            }
            other => self.reader.paths(other)?,
        };
        for path in &paths {
            let open = listed
                .iter()
                .find(|(position, _)| path.assigns.iter().all(|(given, _)| given != position));
            if let Some((_, variable)) = open {
                let (name, variable) = (self.reader.short(name), self.reader.short(variable));
                return self.reader.refuse(
                    "`weakFair`",
                    &format!(
                        "`weakFair({name}, …)`, where `{name}` alone does not give `{variable}'`, \
                         is not read: Quint's TLC runs stop on such a formula. Make `{name}` \
                         assign every variable the fairness names, or make the whole step fair"
                    ),
                );
            }
        }
        Ok(paths)
    }

    fn fair_part(&mut self, action: &'a Quint, step: &Step<'_>) -> Read<Fair<'a>> {
        let (id, name, args): (u64, &str, &'a [Quint]) = match action {
            Quint::Name { id, name } => (*id, name, &[]),
            Quint::App { id, opcode, args } => (*id, opcode, args),
            _ => {
                return self.reader.refuse(
                    "`weakFair`",
                    "fairness of something that is not the step or one of its top-level \
                     actions: name the step or one of its actions, given its arguments",
                );
            }
        };
        let Some(def) = self.def_of(id) else {
            return self.reader.refuse(
                &format!("`{name}`"),
                "fairness of something that is not the step or one of its top-level actions",
            );
        };
        if step.defs.contains(&def.id) && args.is_empty() {
            return Ok(Fair::Step);
        }
        let takes = match &def.expr {
            Quint::Lambda { params, .. } => params.len(),
            _ => 0,
        };
        if takes != args.len() {
            return self.reader.refuse(
                &format!("`{name}`"),
                "fairness of an action that takes arguments, without them, is not read: give \
                 it its arguments, as `weakFair(A(c), vars)`",
            );
        }
        let Some(paths) = step.parts.get(&def.name) else {
            return self.reader.refuse(
                &format!("`{name}`"),
                "fairness of something that is not the step or one of its top-level actions",
            );
        };
        let mut given = Vec::with_capacity(args.len());
        for arg in args {
            let read = self.reader.pure(arg)?;
            let Some(value) = constant(&read, self.reader.slots)? else {
                return self.reader.refuse(
                    &format!("`{name}`"),
                    "fairness of an action given an argument that changes with the state is \
                     not read: give it values that do not, such as a constant or an item of a \
                     `forall` over a set that does not change",
                );
            };
            given.push(value);
        }
        let mut part = Vec::with_capacity(paths.len());
        for position in paths {
            let Some(path) = step.paths.get(*position) else {
                continue;
            };
            for inputs in self.inputs_of(path, &given, name)? {
                part.push((*position, inputs));
            }
        }
        if part.is_empty() && !paths.is_empty() {
            return self.reader.refuse(
                &format!("`{name}`"),
                &format!(
                    "fairness of `{name}` given arguments the step never gives it is not read: \
                     TLC holds such a step possible where the step never takes it"
                ),
            );
        }
        Ok(Fair::Actions { def, given, part })
    }

    fn inputs_of(
        &mut self,
        path: &Path,
        given: &[Value],
        name: &str,
    ) -> Read<Vec<Vec<(usize, Value)>>> {
        let mut allowed: Vec<(usize, Vec<Value>)> = Vec::new();
        for (at, value) in given.iter().enumerate() {
            let passed = path.given.get(at).cloned().flatten();
            let picked = passed.as_ref().and_then(|expr| {
                path.choices
                    .iter()
                    .enumerate()
                    .find(|(_, choice)| !reads_state(expr, &[choice.slot]))
                    .filter(|_| reads_state(expr, &[]))
            });
            if let (Some(expr), Some((input, choice))) = (&passed, picked) {
                let values = self.picked_giving(expr, choice, value, name)?;
                match allowed.iter_mut().find(|(each, _)| *each == input) {
                    Some((_, known)) => known.retain(|each| values.contains(each)),
                    None => allowed.push((input, values)),
                }
                continue;
            }
            match passed.map(|expr| constant(&expr, self.reader.slots)) {
                Some(Ok(Some(fixed))) if fixed == *value => {}
                Some(Ok(Some(_))) => return Ok(Vec::new()),
                _ => {
                    return self.reader.refuse(
                        &format!("`{name}`"),
                        &format!(
                            "fairness of `{name}` given arguments is read only where the step \
                             calls `{name}` with a `nondet` it picks from a set that does not \
                             change, a part of one (`p._1`, `p.who`) or a value that does not \
                             change, so that its steps can be told apart"
                        ),
                    );
                }
            }
        }
        allowed.sort_by_key(|(input, _)| *input);
        let mut combinations: Vec<Vec<(usize, Value)>> = vec![Vec::new()];
        for (input, values) in allowed {
            let grown = combinations.len().saturating_mul(values.len());
            if grown > MOST_ITEMS {
                return self.reader.refuse(
                    &format!("`{name}`"),
                    &format!(
                        "fairness of `{name}` that more than {MOST_ITEMS} picks of the step \
                         give is not read"
                    ),
                );
            }
            let mut next = Vec::with_capacity(grown);
            for known in &combinations {
                for value in &values {
                    let mut one = known.clone();
                    one.push((input, value.clone()));
                    next.push(one);
                }
            }
            combinations = next;
        }
        Ok(combinations)
    }

    fn picked_giving(
        &mut self,
        passed: &Expr,
        choice: &Choice,
        value: &Value,
        name: &str,
    ) -> Read<Vec<Value>> {
        if choice.from.is_some() {
            return self.reader.refuse(
                &format!("`{name}`"),
                &format!(
                    "fairness of `{name}` given an argument the step picks from a set that \
                     changes with the state is not read: pick `{}` from a set that does not \
                     change",
                    choice.name
                ),
            );
        }
        if *passed == Expr::Local(choice.slot) {
            if !choice.values.contains(value) {
                return self.reader.refuse(
                    &format!("`{name}`"),
                    &format!(
                        "fairness of `{name}` given a value the step never picks for `{}` is \
                         not read: TLC holds such a step possible where the step never takes it",
                        choice.name
                    ),
                );
            }
            return Ok(vec![value.clone()]);
        }
        let mut giving = Vec::new();
        for each in &choice.values {
            let bound = Expr::Bind {
                slot: choice.slot,
                value: Box::new(Expr::Const(each.clone())),
                body: Box::new(passed.clone()),
            };
            match constant(&bound, self.reader.slots) {
                Ok(Some(gives)) if gives == *value => giving.push(each.clone()),
                Ok(Some(_)) => {}
                _ => {
                    return self.reader.refuse(
                        &format!("`{name}`"),
                        &format!(
                            "fairness of `{name}` given an argument computed from `{}`, which the \
                             step picks, is read only where it is `{}` or a part of it (`{}._1`, \
                             `{}.field`)",
                            choice.name, choice.name, choice.name, choice.name
                        ),
                    );
                }
            }
        }
        Ok(giving)
    }

    fn goal(&mut self, each: &'a Quint) -> Read<Goal> {
        if self.first_temporal(each, 0).is_none() {
            return Ok(Goal::Initially(self.reader.pure(each)?));
        }
        let Quint::App { opcode, args, .. } = each else {
            return self.not_a_goal(each);
        };
        Ok(match (opcode.as_str(), args.as_slice()) {
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
        })
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
        fair_steps: Vec::new(),
    };
    let Some(listed) = defs.values().find(|def| def.name == PROPERTIES) else {
        return Ok(read);
    };
    let mut properties = Properties {
        reader,
        table,
        defs,
        fair_steps: Vec::new(),
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
        let mut parts = Vec::new();
        if let Err(refusal) = properties.property(&def.expr, step, None, &mut parts) {
            refused.push(refusal);
            continue;
        }
        if parts.is_empty() {
            parts.push((
                Fairness::default(),
                Goal::Always(Expr::Const(Value::Bool(true))),
            ));
        }
        let slots = properties.reader.slots;
        let count = parts.len();
        for (index, (fair, goal)) in parts.into_iter().enumerate() {
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
                    slots,
                    fair: Some(fair),
                }),
                Goal::Initially(holds) => read.eventually.push(Eventually {
                    id,
                    says: format!("{says} in every start state"),
                    line: DECLARED_ON,
                    after: None,
                    reaches: Code::new(holds),
                    slots,
                    fair: Some(Fairness::default()),
                }),
                Goal::Always(holds) => read.rules.push(Rule {
                    id,
                    says,
                    line: DECLARED_ON,
                    text: name.clone(),
                    holds: Code::new(holds),
                    slots,
                }),
            }
        }
    }
    read.fair_steps = properties.fair_steps;
    if refused.is_empty() {
        Ok(read)
    } else {
        Err(refused)
    }
}

#[cfg(test)]
mod tests;
