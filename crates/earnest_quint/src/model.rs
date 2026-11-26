use crate::json::{Compiled, Declaration, Def, Expr as Quint};
use crate::reader::{Path, Read, Reader, Refusal, all_of, refusal};
use crate::scan::refused;
use crate::starts::start_states;
use earnest_ir::{Action, Code, End, Expr, Field, Input, Model, Rule, Runs, View};
use earnest_value::{Domain, Symbols, Value};
use std::collections::HashMap;

const INIT: &str = "q::init";
const STEP: &str = "q::step";
const INVARIANTS: &str = "q::inv";
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
                ));
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
            })
        })
        .collect()
}

fn named<'a>(defs: &HashMap<u64, &'a Def>, name: &str) -> Option<&'a Def> {
    defs.values().find(|def| def.name == name).copied()
}

fn missing(name: &str) -> Vec<Refusal> {
    vec![refusal(
        &format!("`{name}`"),
        "the compiled module has no such definition: compile it with `quint compile --target json`",
    )]
}

fn ids(paths: &[Path], fallback: &str) -> Vec<String> {
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

fn action(id: String, path: &Path, fields: &[String]) -> Read<Action> {
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
        .unwrap_or(0);
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

fn rules<'a>(reader: &mut Reader<'a>, defs: &HashMap<u64, &'a Def>) -> Read<Vec<Rule>> {
    let Some(invariants) = named(defs, INVARIANTS) else {
        return Ok(Vec::new());
    };
    INVARIANTS.clone_into(&mut reader.within);
    let conjuncts: &[Quint] = match &invariants.expr {
        Quint::App { opcode, args, .. } if opcode == "and" => args,
        other => std::slice::from_ref(other),
    };
    let mut rules = Vec::with_capacity(conjuncts.len());
    for conjunct in conjuncts {
        let Quint::Name { name, .. } = conjunct else {
            return reader.refuse(
                "an invariant",
                "an invariant that is not the name of a definition",
            );
        };
        let holds = reader.pure(conjunct)?;
        rules.push(Rule {
            id: name.clone(),
            says: format!("The Quint invariant `{name}` holds"),
            line: DECLARED_ON,
            text: name.clone(),
            holds: Code::new(holds),
            slots: 0,
        });
    }
    Ok(rules)
}

fn unfolded<'a>(reader: &mut Reader<'a>, root: &'a Def) -> Read<Vec<Path>> {
    reader.unnamed.push(root.id);
    if let Quint::Name { name, .. } = &root.expr
        && let Some(found) = reader.top(name)
    {
        reader.unnamed.push(found.id);
    }
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

type Parts = (Vec<Vec<Value>>, Vec<Action>, Vec<Rule>);

fn parts<'a>(
    reader: &mut Reader<'a>,
    defs: &HashMap<u64, &'a Def>,
    (init, step): (&'a Def, &'a Def),
    names: &[String],
) -> Read<Parts> {
    let rules = rules(reader, defs)?;
    reader.in_init = true;
    let starting = unfolded(reader, init)?;
    reader.in_init = false;
    let starts = start_states(&starting, names)?;
    let stepping = unfolded(reader, step)?;
    let fallback = match &step.expr {
        Quint::Name { name, .. } => name.clone(),
        _ => step.name.clone(),
    };
    let actions = ids(&stepping, &fallback)
        .into_iter()
        .zip(&stepping)
        .map(|(id, path)| action(id, path, names))
        .collect::<Read<Vec<Action>>>()?;
    Ok((starts, actions, rules))
}

pub(crate) fn model(path: &str, compiled: &Compiled) -> Result<Model, Vec<Refusal>> {
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
    let variables = variables(&module.declarations);
    let names: Vec<String> = variables.iter().map(|each| each.name.to_owned()).collect();
    let positions = variables
        .iter()
        .enumerate()
        .map(|(position, each)| (each.id, position))
        .collect();
    let (Some(init), Some(step)) = (named(&defs, INIT), named(&defs, STEP)) else {
        return Err(missing(if named(&defs, INIT).is_none() {
            INIT
        } else {
            STEP
        }));
    };
    let mut reader = Reader::new(&compiled.table, defs.clone(), positions);
    let (starts, actions, rules) =
        parts(&mut reader, &defs, (init, step), &names).map_err(|refusal| vec![refusal])?;
    let mut symbols = Symbols::default();
    let fields = variables
        .into_iter()
        .enumerate()
        .map(|(position, variable)| {
            let mut values: Vec<Value> = starts
                .iter()
                .filter_map(|state| state.get(position).cloned())
                .collect();
            values.sort();
            values.dedup();
            Field {
                name: variable.name.to_owned(),
                sym: symbols.intern(variable.name),
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
            ..Runs::default()
        },
    })
}
