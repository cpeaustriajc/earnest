use crate::inference::printing::{leaf, type_text};
use crate::inference::solver::{Context, Errors, Table};
use crate::inference::substitution::{Substitution, apply};
use crate::inference::value::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct Fresh {
    counters: HashMap<String, u64>,
}

impl Fresh {
    pub(crate) fn var(&mut self, prefix: &str) -> String {
        let counter = self.counters.entry(prefix.to_owned()).or_insert(0);
        let name = format!("{prefix}{counter}");
        *counter += 1;
        name
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Hook {
    Exit,
    Enter,
}

#[derive(Debug, Default)]
pub(crate) struct Resolver {
    fresh: Fresh,
    pub(crate) errors: Errors,
}

impl Resolver {
    pub(crate) fn resolve_table(&mut self, context: &mut Context) {
        for id in context.table.ids() {
            let Some(definition) = context.table.get(id).cloned() else {
                continue;
            };
            let resolved = self.lookup_definition(context, &definition);
            context.table.set(id, resolved);
        }
    }

    fn lookup_definition(&mut self, context: &Context, definition: &Value) -> Value {
        match definition.kind() {
            "param" => {
                let annotation = definition.get("typeAnnotation");
                if annotation.truthy() {
                    let resolved = self.resolved_type(context, annotation);
                    definition.spread(vec![("typeAnnotation", resolved)])
                } else {
                    definition.clone()
                }
            }
            _ => self.definition(context, definition),
        }
    }

    pub(crate) fn declaration(&mut self, context: &Context, declaration: &Value) -> Value {
        match declaration.kind() {
            "instance" => {
                let mut copy = declaration.clone();
                let overrides = declaration
                    .items("overrides")
                    .iter()
                    .map(|pair| {
                        let items = pair.as_items();
                        let name = items.first().cloned().unwrap_or(Value::Undefined);
                        let value = items
                            .get(1)
                            .map_or(Value::Undefined, |value| self.expression(context, value));
                        Value::list(vec![name, value])
                    })
                    .collect();
                copy.set("overrides", Value::list(overrides));
                copy
            }
            "import" | "export" => declaration.clone(),
            _ => self.definition(context, declaration),
        }
    }

    fn definition(&mut self, context: &Context, definition: &Value) -> Value {
        let mut copy = definition.clone();
        let annotation = definition.get("typeAnnotation");
        if annotation.is_defined() {
            copy.set("typeAnnotation", self.resolved_type(context, annotation));
        } else if definition.kind() == "typedef" && definition.get("type").truthy() {
            copy.set("type", self.resolved_type(context, definition.get("type")));
        }
        match definition.kind() {
            "def" => copy.set("expr", self.expression(context, definition.get("expr"))),
            "assume" => copy.set(
                "assumption",
                self.expression(context, definition.get("assumption")),
            ),
            _ => {}
        }
        copy
    }

    fn expression(&mut self, context: &Context, expression: &Value) -> Value {
        let mut copy = expression.clone();
        match expression.kind() {
            "app" => {
                let arguments = expression
                    .items("args")
                    .iter()
                    .map(|argument| self.expression(context, argument))
                    .collect();
                copy.set("args", Value::list(arguments));
            }
            "lambda" => {
                let params = expression
                    .items("params")
                    .iter()
                    .map(|param| {
                        let annotation = param.get("typeAnnotation");
                        if annotation.truthy() {
                            let resolved = self.resolved_type(context, annotation);
                            param.spread(vec![("typeAnnotation", resolved)])
                        } else {
                            param.clone()
                        }
                    })
                    .collect();
                copy.set("params", Value::list(params));
                copy.set("expr", self.expression(context, expression.get("expr")));
            }
            "let" => {
                copy.set("opdef", self.definition(context, expression.get("opdef")));
                copy.set("expr", self.expression(context, expression.get("expr")));
            }
            _ => {}
        }
        copy
    }

    fn resolved_type(&mut self, context: &Context, written: &Value) -> Value {
        self.walk_type(context, written, Hook::Exit)
    }

    fn walk_type(&mut self, context: &Context, written: &Value, hook: Hook) -> Value {
        let mut copy = written.clone();
        if hook == Hook::Enter && copy.kind() == "app" {
            copy = self.application(context, &copy);
        }
        let source = copy.clone();
        match source.kind() {
            "set" | "list" => copy.set("elem", self.walk_type(context, source.get("elem"), hook)),
            "fun" => {
                copy.set("arg", self.walk_type(context, source.get("arg"), hook));
                copy.set("res", self.walk_type(context, source.get("res"), hook));
            }
            "oper" => {
                let arguments = self.walk_types(context, source.items("args"), hook);
                copy.set("args", arguments);
                copy.set("res", self.walk_type(context, source.get("res"), hook));
            }
            "tup" | "rec" | "sum" => {
                copy.set("fields", self.walk_row(context, source.get("fields"), hook));
            }
            "app" => {
                let arguments = self.walk_types(context, source.items("args"), hook);
                copy.set("args", arguments);
            }
            _ => {}
        }
        if hook == Hook::Exit {
            copy = self.walk_type(context, &copy, Hook::Enter);
        }
        copy
    }

    fn walk_types(&mut self, context: &Context, types: &[Value], hook: Hook) -> Value {
        Value::list(
            types
                .iter()
                .map(|inner| self.walk_type(context, inner, hook))
                .collect(),
        )
    }

    fn walk_row(&mut self, context: &Context, row: &Value, hook: Hook) -> Value {
        let mut copy = row.clone();
        if row.kind() == "row" {
            let fields = row
                .items("fields")
                .iter()
                .map(|field| {
                    let resolved = self.walk_type(context, field.get("fieldType"), hook);
                    field.spread(vec![("fieldType", resolved)])
                })
                .collect();
            copy.set("fields", Value::list(fields));
            copy.set("other", self.walk_row(context, row.get("other"), hook));
        }
        copy
    }

    fn application(&mut self, context: &Context, written: &Value) -> Value {
        let ctor = written.get("ctor");
        let definition = ctor.id().and_then(|id| context.table.get(id));
        let Some(definition) = definition
            .filter(|definition| definition.kind() == "typedef" && definition.get("type").truthy())
        else {
            context.crash(format!(
                "invalid kind looked up for constructor of type application {}",
                type_text(written)
            ));
            return written.clone();
        };
        let (params, defined) = self.fresh_type(definition);
        let arguments = written.items("args");
        if params.len() != arguments.len() {
            let ctor_text = type_text(ctor);
            let shown: Vec<String> = arguments.iter().map(type_text).collect();
            let many_or_few = if params.len() > arguments.len() {
                "few"
            } else {
                "many"
            };
            let error = leaf(
                &format!(
                    "applying type constructor {ctor_text} to arguments {}",
                    shown.join(", ")
                ),
                &format!(
                    "too {many_or_few} arguments supplied: {ctor_text} only accepts {} parameters",
                    params.len()
                ),
            );
            if let Some(id) = written.id() {
                self.errors.set(id, error);
            }
            return written.clone();
        }
        let mut substitutions = Vec::new();
        for (param, argument) in params.iter().zip(arguments) {
            let resolved = resolve_alias(&context.table, argument);
            substitutions.push(Substitution {
                row: false,
                name: param.clone(),
                value: resolved.clone(),
            });
            match resolved.kind() {
                "rec" | "tup" => substitutions.push(Substitution {
                    row: true,
                    name: param.clone(),
                    value: resolved.get("fields").clone(),
                }),
                "var" => substitutions.push(Substitution {
                    row: true,
                    name: param.clone(),
                    value: Value::object(vec![
                        ("kind", Value::text("var")),
                        ("name", resolved.get("name").clone()),
                    ]),
                }),
                _ => {}
            }
        }
        apply(&substitutions, &defined)
    }

    fn fresh_type(&mut self, definition: &Value) -> (Vec<String>, Value) {
        let params = definition.items("params");
        let defined = definition.get("type");
        if params.is_empty() {
            return (Vec::new(), defined.clone());
        }
        let mut renamed: Vec<(String, String)> = Vec::new();
        for param in params {
            let name = match param {
                Value::Text(name) => name.clone(),
                _ => String::new(),
            };
            let fresh = self.fresh.var(&name);
            match renamed.iter_mut().find(|(known, _)| *known == name) {
                Some((_, slot)) => *slot = fresh,
                None => renamed.push((name, fresh)),
            }
        }
        let defined = rename(defined, &renamed);
        (
            renamed.into_iter().map(|(_, fresh)| fresh).collect(),
            defined,
        )
    }
}

fn renamed_name(name: &str, renamed: &[(String, String)]) -> String {
    renamed
        .iter()
        .find(|(known, _)| known == name)
        .map_or_else(|| name.to_owned(), |(_, fresh)| fresh.clone())
}

fn rename(written: &Value, renamed: &[(String, String)]) -> Value {
    let mut copy = written.clone();
    match written.kind() {
        "var" => copy.set(
            "name",
            Value::Text(renamed_name(written.str("name"), renamed)),
        ),
        "set" | "list" => copy.set("elem", rename(written.get("elem"), renamed)),
        "fun" => {
            copy.set("arg", rename(written.get("arg"), renamed));
            copy.set("res", rename(written.get("res"), renamed));
        }
        "oper" => {
            copy.set(
                "args",
                Value::list(
                    written
                        .items("args")
                        .iter()
                        .map(|inner| rename(inner, renamed))
                        .collect(),
                ),
            );
            copy.set("res", rename(written.get("res"), renamed));
        }
        "tup" | "rec" | "sum" => copy.set("fields", rename_row(written.get("fields"), renamed)),
        "app" => copy.set(
            "args",
            Value::list(
                written
                    .items("args")
                    .iter()
                    .map(|inner| rename(inner, renamed))
                    .collect(),
            ),
        ),
        _ => {}
    }
    copy
}

fn rename_row(row: &Value, renamed: &[(String, String)]) -> Value {
    let mut copy = row.clone();
    match row.kind() {
        "var" => copy.set("name", Value::Text(renamed_name(row.str("name"), renamed))),
        "row" => {
            copy.set(
                "fields",
                Value::list(
                    row.items("fields")
                        .iter()
                        .map(|field| {
                            field.spread(vec![(
                                "fieldType",
                                rename(field.get("fieldType"), renamed),
                            )])
                        })
                        .collect(),
                ),
            );
            copy.set("other", rename_row(row.get("other"), renamed));
        }
        _ => {}
    }
    copy
}

pub(crate) fn resolve_alias(table: &Table, written: &Value) -> Value {
    let mut current = written;
    while current.kind() == "const"
        && let Some(id) = current.id()
        && let Some(alias) = table.get(id)
        && alias.kind() == "typedef"
        && alias.get("type").truthy()
    {
        current = alias.get("type");
    }
    current.clone()
}
