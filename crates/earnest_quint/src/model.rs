use crate::Runs as Chosen;
use crate::json::{Compiled, Declaration, Def, Entry, Expr as Quint};
use crate::names::Short;
use crate::notation::notation;
use crate::reach::variables_reached;
use crate::reader::{MOST_ACTIONS, Path, Read, Reader, Refusal, all_of, refusal, too_many_actions};
use crate::runs::{Variables, scripts};
use crate::scan::refused;
use crate::starts::start_states;
use crate::temporal::{Step, temporal};
use earnest_ir::{
    Action, Code, End, Eventually, Expr, Field, Input, Model, Pick, Rule, Runs, Script, View,
};
use earnest_value::{Domain, Symbols, Value};
use std::collections::{HashMap, HashSet};

const INIT: &str = "q::init";
const STEP: &str = "q::step";
const INVARIANTS: &str = "q::inv";
const PROPERTIES: &str = "q::temporalProps";
const WRITTEN_INVARIANT: &str = "invariant";
const NOT_BOOLEAN: [&str; 22] = [
    "iadd",
    "isub",
    "imul",
    "idiv",
    "imod",
    "ipow",
    "iuminus",
    "Set",
    "List",
    "Map",
    "Rec",
    "Tup",
    "to",
    "range",
    "size",
    "length",
    "keys",
    "map",
    "filter",
    "union",
    "intersect",
    "exclude",
];
const STOPS_ANYWHERE: &str = "stops_anywhere";
const STOPS_ANYWHERE_SAYS: &str =
    "A run may stop in any state, as in Quint's TLC runs, which do not check deadlock";
const DECLARED_ON: u32 = 1;

struct Variable<'a> {
    id: u64,
    name: &'a str,
    domain: Domain,
}

pub(crate) fn assigned(path: &Path, fields: &[String], within: &str) -> Read<Vec<Expr>> {
    let mut values: Vec<Option<Expr>> = vec![None; fields.len()];
    for (position, expr) in &path.assigns {
        let name = fields.get(*position).map_or("", String::as_str);
        match values.get_mut(*position) {
            Some(Some(_)) => {
                return Err(refusal(
                    &format!("`{name}'`"),
                    &format!("one path through `{within}` assigns this variable twice"),
                )
                .within(within));
            }
            Some(slot) => *slot = Some(expr.clone()),
            None => {}
        }
    }
    values
        .into_iter()
        .zip(fields)
        .map(|(value, name)| {
            value.ok_or_else(|| {
                refusal(
                    &format!("`{name}`"),
                    &format!(
                        "a path through `{within}` leaves this variable unassigned, which Quint's \
                         TLC runs stop on"
                    ),
                )
                .within(within)
            })
        })
        .collect()
}

fn named<'a>(defs: &HashMap<u64, &'a Def>, name: &str) -> Option<&'a Def> {
    defs.values().find(|def| def.name == name).copied()
}

fn unsearched(name: &str) -> Vec<Refusal> {
    let shown = name.strip_prefix("q::").unwrap_or(name);
    vec![refusal(
        &format!("`{shown}`"),
        &format!(
            "the check lists invariants or temporal properties, which are checked on the states \
             that `init` and `step` reach, and the module has no `{shown}`: name the module's \
             `init` and `step` in the check, or list only `runs`"
        ),
    )]
}

fn missing(name: &str) -> Vec<Refusal> {
    vec![refusal(
        &format!("`{name}`"),
        "the compiled module has no such definition: compile it with `quint compile --target json`",
    )]
}

pub(crate) fn ids(paths: &[Path], fallback: &str) -> Vec<String> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    paths
        .iter()
        .map(|path| {
            let name = path.name.clone().unwrap_or_else(|| fallback.to_owned());
            let count = seen.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                name
            } else {
                format!("{name}#{count}")
            }
        })
        .collect()
}

pub(crate) fn picks(
    script: Option<usize>,
    position: usize,
    path: &Path,
) -> impl Iterator<Item = Pick> + '_ {
    path.choices
        .iter()
        .enumerate()
        .filter_map(move |(input, choice)| {
            Some(Pick {
                script,
                action: position,
                input,
                from: Code::new(choice.from.clone()?),
            })
        })
}

pub(crate) fn action(id: String, path: &Path, fields: &[String], slots: usize) -> Read<Action> {
    let updates = assigned(path, fields, &id)?;
    let changed: Vec<bool> = updates
        .iter()
        .enumerate()
        .map(|(position, update)| *update != Expr::Field(position))
        .collect();
    let guard = all_of(path.guards.clone());
    let mut prepared = Code::together(
        std::iter::once(guard).chain(
            updates
                .into_iter()
                .zip(&changed)
                .filter(|(_, changes)| **changes)
                .map(|(update, _)| update),
        ),
    )
    .into_iter();
    let guard = prepared
        .next()
        .unwrap_or_else(|| Code::new(Expr::Const(Value::Bool(true))));
    let slots = path
        .choices
        .iter()
        .map(|choice| choice.slot.saturating_add(1))
        .max()
        .unwrap_or(0)
        .max(slots);
    Ok(Action {
        label: id.clone(),
        id,
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
        guard,
        updates: changed
            .into_iter()
            .map(|changes| if changes { prepared.next() } else { None })
            .collect(),
        slots,
    })
}

fn kept<T>(read: Read<T>, refused: &mut Vec<Refusal>) -> Option<T> {
    read.map_err(|refusal| refused.push(refusal)).ok()
}

fn gives_no_boolean(expr: &Quint) -> bool {
    match expr {
        Quint::Int { .. } | Quint::Str { .. } | Quint::Lambda { .. } => true,
        Quint::App { opcode, .. } => NOT_BOOLEAN.contains(&opcode.as_str()),
        Quint::Let { expr, .. } => gives_no_boolean(expr),
        Quint::Bool { .. } | Quint::Name { .. } => false,
    }
}

fn rules<'a>(
    reader: &mut Reader<'a>,
    defs: &HashMap<u64, &'a Def>,
    refused: &mut Vec<Refusal>,
) -> Vec<Rule> {
    let Some(invariants) = named(defs, INVARIANTS) else {
        return Vec::new();
    };
    let conjuncts: &[Quint] = match &invariants.expr {
        Quint::App { opcode, args, .. } if opcode == "and" => args,
        other => std::slice::from_ref(other),
    };
    let mut rules = Vec::with_capacity(conjuncts.len());
    for (position, conjunct) in conjuncts.iter().enumerate() {
        INVARIANTS.clone_into(&mut reader.within);
        let name = match conjunct {
            Quint::Name { name, .. } => name.clone(),
            other if gives_no_boolean(other) => {
                kept(
                    reader.refuse::<()>(
                        "an invariant",
                        "an invariant that gives a value that is not a boolean, which Quint's \
                         TLC runs refuse",
                    ),
                    refused,
                );
                continue;
            }
            _ => format!("{WRITTEN_INVARIANT} {}", position.saturating_add(1)),
        };
        let Some(holds) = kept(reader.pure(conjunct), refused) else {
            continue;
        };
        let slots = reader.slots;
        rules.push(Rule {
            id: name.clone(),
            says: format!("The Quint invariant `{name}` holds"),
            line: DECLARED_ON,
            text: name,
            holds: Code::new(holds),
            slots,
        });
    }
    rules
}

fn aliases<'a>(reader: &Reader<'a>, root: &'a Def) -> Vec<u64> {
    let mut chain = vec![root.id];
    let mut def = root;
    while let Quint::Name { name, .. } = &def.expr
        && let Some(found) = reader.top(name)
        && !chain.contains(&found.id)
    {
        chain.push(found.id);
        def = found;
    }
    chain
}

fn unfolded<'a>(reader: &mut Reader<'a>, root: &'a Def) -> Read<Vec<Path>> {
    let chain = aliases(reader, root);
    reader.unnamed.extend(chain);
    reader.within.clone_from(&root.name);
    reader.paths(&root.expr)
}

fn variables(declarations: &[Declaration]) -> Vec<Variable<'_>> {
    declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Var {
                id,
                name,
                type_annotation,
            } => Some(Variable {
                id: *id,
                name,
                domain: if type_annotation.kind == "bool" {
                    Domain::Bool
                } else {
                    Domain::Any
                },
            }),
            _ => None,
        })
        .collect()
}

type Parts = (
    Vec<Vec<Value>>,
    Vec<Action>,
    Vec<Rule>,
    Vec<Eventually>,
    Vec<Pick>,
);

fn stepped<'a>(
    reader: &mut Reader<'a>,
    step: &'a Def,
    refused: &mut Vec<Refusal>,
) -> (Vec<Path>, Vec<String>) {
    let chain = aliases(reader, step);
    reader.unnamed.extend(chain);
    reader.within.clone_from(&step.name);
    let mut stepping = Vec::new();
    let mut unread = Vec::new();
    for (written, read) in reader.alternatives(step) {
        match read {
            Ok(paths) => stepping.extend(paths),
            Err(refusal) => {
                refused.push(refusal);
                if let Quint::Name { name, .. } = written {
                    unread.push(name.clone());
                }
            }
        }
    }
    if stepping.len() > MOST_ACTIONS {
        refused.push(too_many_actions());
    }
    (stepping, unread)
}

fn parts<'a>(
    reader: &mut Reader<'a>,
    table: &'a HashMap<String, Entry>,
    defs: &HashMap<u64, &'a Def>,
    (init, step): (&'a Def, &'a Def),
    names: &[String],
) -> Result<Parts, Vec<Refusal>> {
    let mut refused = Vec::new();
    let rules = rules(reader, defs, &mut refused);
    reader.in_init = true;
    let starting = kept(unfolded(reader, init), &mut refused);
    reader.in_init = false;
    let starts = starting
        .and_then(|starting| kept(start_states(&starting, names, reader.slots), &mut refused));
    let (stepping, unread) = stepped(reader, step, &mut refused);
    let fallback = match &step.expr {
        Quint::Name { name, .. } => name.clone(),
        _ => step.name.clone(),
    };
    let actions: Vec<Action> = ids(&stepping, &fallback)
        .into_iter()
        .zip(&stepping)
        .filter_map(|(id, path)| kept(action(id, path, names, reader.slots), &mut refused))
        .collect();
    let mut parts: HashMap<String, Vec<usize>> = HashMap::new();
    for (position, path) in stepping.iter().enumerate() {
        if let Some(name) = &path.name {
            parts.entry(name.clone()).or_default().push(position);
        }
    }
    for name in unread {
        parts.entry(name).or_default();
    }
    let step = Step {
        defs: aliases(reader, step),
        parts: &parts,
        fields: names,
    };
    let mut rules = rules;
    let read = temporal(reader, table, defs, &step).map_err(|mut found| refused.append(&mut found));
    let (Some(starts), Ok(read), true) = (starts, read, refused.is_empty()) else {
        return Err(refused);
    };
    rules.extend(read.rules);
    let picks = stepping
        .iter()
        .enumerate()
        .flat_map(|(position, path)| picks(None, position, path))
        .collect();
    Ok((starts, actions, rules, read.eventually, picks))
}

fn both<A, B>(
    first: Result<A, Vec<Refusal>>,
    second: Result<B, Vec<Refusal>>,
    defs: &HashMap<u64, &Def>,
) -> Result<(A, B), Vec<Refusal>> {
    match (first, second) {
        (Ok(first), Ok(second)) => Ok((first, second)),
        (first, second) => {
            let refused = first
                .err()
                .into_iter()
                .chain(second.err())
                .flatten()
                .collect();
            Err(listed(refused, defs))
        }
    }
}

fn listed(mut refused: Vec<Refusal>, defs: &HashMap<u64, &Def>) -> Vec<Refusal> {
    let mut seen = HashSet::new();
    refused.retain(|each| {
        let within = each.id.is_none().then(|| each.within.clone());
        seen.insert((each.feature.clone(), each.id, within))
    });
    refused.sort_by_key(|each| {
        each.id
            .or_else(|| {
                let within = each.within.as_deref()?;
                named(defs, within).map(|def| def.id)
            })
            .unwrap_or(u64::MAX)
    });
    refused
}

fn imported_runs(table: &HashMap<String, Entry>, main: &str) -> Vec<String> {
    let flattened = format!("{main}::");
    let mut found: Vec<String> = table
        .values()
        .filter(|entry| entry.kind == "def" && entry.qualifier.as_deref() == Some("run"))
        .filter_map(|entry| entry.name.as_deref())
        .filter_map(|name| name.strip_prefix(&flattened))
        .filter_map(|rest| rest.rsplit_once("::").map(|(_, last)| last.to_owned()))
        .collect();
    found.sort_unstable();
    found.dedup();
    found
}

fn shorten(
    actions: &mut [Action],
    rules: &mut [Rule],
    eventually: &mut [Eventually],
    scripts: &mut [Script],
) {
    let played = scripts.iter().flat_map(|script| &script.actions);
    let short_actions = Short::of_ids(actions.iter().chain(played).map(|each| each.id.as_str()));
    for action in actions.iter_mut() {
        short_actions.rename_action(action);
    }
    for action in scripts.iter_mut().flat_map(|script| &mut script.actions) {
        short_actions.rename_action(action);
    }
    let short_rules = Short::of_ids(
        rules
            .iter()
            .map(|each| each.id.as_str())
            .chain(eventually.iter().map(|each| each.id.as_str()))
            .chain(scripts.iter().map(|each| each.id.as_str())),
    );
    for rule in rules {
        short_rules.rename_rule(rule);
    }
    for rule in eventually {
        short_rules.rename_eventually(rule);
    }
    for script in scripts {
        short_rules.rename_script(script);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wanted {
    Search,
    Runs,
}

pub(crate) fn model(
    path: &str,
    compiled: &Compiled,
    runs: &Chosen,
    wanted: Wanted,
) -> Result<Model, Vec<Refusal>> {
    let met = built(path, compiled, Symbols::default(), (runs, wanted))?;
    let mut read = built(path, compiled, met.symbols.alphabetical(), (runs, wanted))?;
    read.symbols = std::mem::take(&mut read.symbols).written_in(notation(compiled));
    Ok(read)
}

type Layout<'a> = (Vec<(u64, String, Domain)>, Vec<(Variable<'a>, String)>);

fn layout<'a>(
    compiled: &Compiled,
    defs: &HashMap<u64, &'a Def>,
    declarations: &'a [Declaration],
    wanted: Wanted,
) -> Layout<'a> {
    let every_variable = variables(declarations);
    let short_fields = Short::of(every_variable.iter().map(|each| each.name));
    let checked: Vec<&Quint> = [INIT, STEP, INVARIANTS, PROPERTIES]
        .iter()
        .filter(|_| wanted == Wanted::Search)
        .filter_map(|name| named(defs, name))
        .map(|def| &def.expr)
        .collect();
    let reached = variables_reached(&compiled.table, defs, &checked);
    let every = every_variable
        .iter()
        .map(|each| (each.id, short_fields.name(each.name), each.domain.clone()))
        .collect();
    let searched = every_variable
        .into_iter()
        .filter(|each| reached.contains(&each.id))
        .map(|each| {
            let name = short_fields.name(each.name);
            (each, name)
        })
        .collect();
    (every, searched)
}

type Searched<'a> = Option<(&'a Def, &'a Def)>;

fn search_of<'a>(
    defs: &HashMap<u64, &'a Def>,
    (runs, wanted): (&Chosen, Wanted),
) -> Result<Searched<'a>, Vec<Refusal>> {
    match (named(defs, INIT), named(defs, STEP)) {
        _ if wanted == Wanted::Runs => Ok(None),
        (Some(init), Some(step)) => Ok(Some((init, step))),
        (init, _) => {
            let absent = if init.is_none() { INIT } else { STEP };
            if named(defs, INVARIANTS).is_some() || named(defs, PROPERTIES).is_some() {
                return Err(unsearched(absent));
            }
            if matches!(runs, Chosen::None) {
                return Err(missing(absent));
            }
            Ok(None)
        }
    }
}

fn built(
    path: &str,
    compiled: &Compiled,
    symbols: Symbols,
    (runs, wanted): (&Chosen, Wanted),
) -> Result<Model, Vec<Refusal>> {
    if !compiled.errors.is_empty() {
        return Err(vec![refusal(
            "the compiled module",
            "Quint reported errors when it compiled the module",
        )]);
    }
    let Some(module) = compiled
        .modules
        .iter()
        .find(|module| module.name == compiled.main)
    else {
        return Err(missing(&compiled.main));
    };
    let refusals = refused(&module.declarations);
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let defs: HashMap<u64, &Def> = module
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Def(def) => Some((def.id, def)),
            _ => None,
        })
        .collect();
    let (every, variables) = layout(compiled, &defs, &module.declarations, wanted);
    let searched: Vec<u64> = variables.iter().map(|(each, _)| each.id).collect();
    let names: Vec<String> = variables.iter().map(|(_, name)| name.clone()).collect();
    let positions = searched
        .iter()
        .enumerate()
        .map(|(position, id)| (*id, position))
        .collect();
    let search = search_of(&defs, (runs, wanted))?;
    let mut reader = Reader::new(compiled, defs.clone(), positions, symbols);
    let read = match search {
        Some(search) => parts(&mut reader, &compiled.table, &defs, search, &names),
        None => Ok(Default::default()),
    };
    let imported = imported_runs(&compiled.table, &compiled.main);
    let played = scripts(
        &mut reader,
        (&module.declarations, &imported),
        (search.map(|(init, _)| init), runs),
        &Variables {
            every: &every,
            searched: &searched,
        },
    );
    let ((starts, mut actions, mut rules, mut eventually, picks), (mut scripts, script_picks)) =
        both(read, played, &defs)?;
    let picks = picks.into_iter().chain(script_picks).collect();
    shorten(&mut actions, &mut rules, &mut eventually, &mut scripts);
    let mut symbols = std::mem::take(&mut reader.symbols);
    let fields = variables
        .into_iter()
        .enumerate()
        .map(|(position, (variable, name))| {
            let mut values: Vec<Value> = starts
                .iter()
                .filter_map(|state| state.get(position).cloned())
                .collect();
            values.sort();
            values.dedup();
            Field {
                sym: symbols.intern(&name),
                name,
                domain: variable.domain,
                starts: values,
            }
        })
        .collect();
    Ok(Model {
        name: compiled.main.clone(),
        file: path.to_owned(),
        line: DECLARED_ON,
        symbols,
        fields,
        actions,
        rules,
        ends: vec![End {
            id: STOPS_ANYWHERE.to_owned(),
            says: STOPS_ANYWHERE_SAYS.to_owned(),
            when: Code::new(Expr::Const(Value::Bool(true))),
            slots: 0,
        }],
        view: View::default(),
        runs: Runs {
            starts: Some(starts),
            eventually,
            scripts,
            picks,
            ..Runs::default()
        },
    })
}
