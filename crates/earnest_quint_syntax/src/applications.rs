use crate::flatten::{Table, field_mut, id_of, put, text};
use crate::json::Json;
use crate::names::Entry;
use std::collections::HashMap;

pub(crate) struct Applications {
    fresh: HashMap<String, u64>,
}

enum Substitution {
    Kind(String, Json),
    Row(String, Json),
}

fn same(left: &Json, right: &Json) -> bool {
    match (left, right) {
        (Json::Object(left_fields), Json::Object(right_fields)) => {
            left_fields.len() == right_fields.len()
                && left_fields
                    .iter()
                    .all(|(key, value)| right.get(key).is_some_and(|other| same(value, other)))
        }
        (Json::List(left_items), Json::List(right_items)) => {
            left_items.len() == right_items.len()
                && left_items
                    .iter()
                    .zip(right_items)
                    .all(|(left, right)| same(left, right))
        }
        _ => left == right,
    }
}

fn rebuilt(kind: &str, fields: Vec<(&str, Json)>, id: Option<&Json>) -> Json {
    let mut all = vec![("kind", Json::text(kind))];
    all.extend(fields);
    if let Some(id) = id {
        all.push(("id", id.clone()));
    }
    Json::object(all)
}

fn substituted(table: &Table, subs: &[Substitution], written: &Json) -> Json {
    let id = written.get("id");
    let at = |key: &str| {
        written
            .get(key)
            .map_or(Json::Null, |inner| substituted(table, subs, inner))
    };
    let kind = text(written, "kind");
    let once = match kind {
        "var" => subs
            .iter()
            .find_map(|sub| match sub {
                Substitution::Kind(name, value) if name == text(written, "name") => {
                    Some(value.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| written.clone()),
        "oper" => {
            let args = match written.get("args") {
                Some(Json::List(args)) => args
                    .iter()
                    .map(|inner| substituted(table, subs, inner))
                    .collect(),
                _ => Vec::new(),
            };
            rebuilt(
                kind,
                vec![("args", Json::List(args)), ("res", at("res"))],
                id,
            )
        }
        "list" | "set" => rebuilt(kind, vec![("elem", at("elem"))], id),
        "fun" => rebuilt(kind, vec![("arg", at("arg")), ("res", at("res"))], id),
        "tup" | "rec" => rebuilt(
            kind,
            vec![(
                "fields",
                written
                    .get("fields")
                    .map_or(Json::Null, |row| substituted_row(table, subs, row)),
            )],
            id,
        ),
        "sum" => {
            let mut copy = written.clone();
            if let Some(row) = written.get("fields") {
                put(&mut copy, "fields", substituted_row(table, subs, row));
            }
            copy
        }
        "app" => {
            let mut copy = written.clone();
            if let Some(Json::List(args)) = written.get("args") {
                let args = args
                    .iter()
                    .map(|inner| substituted(table, subs, inner))
                    .collect();
                put(&mut copy, "args", Json::List(args));
            }
            copy
        }
        _ => written.clone(),
    };
    let rebuilt_without_id =
        id.is_none() && matches!(kind, "oper" | "list" | "set" | "fun" | "tup" | "rec");
    if !rebuilt_without_id && same(&once, written) {
        written.clone()
    } else if same(&once, written) {
        once
    } else {
        substituted(table, subs, &once)
    }
}

fn substituted_row(table: &Table, subs: &[Substitution], row: &Json) -> Json {
    match text(row, "kind") {
        "row" => {
            let fields = match row.get("fields") {
                Some(Json::List(fields)) => fields
                    .iter()
                    .map(|field| {
                        Json::object(vec![
                            (
                                "fieldName",
                                field.get("fieldName").cloned().unwrap_or(Json::Null),
                            ),
                            (
                                "fieldType",
                                field
                                    .get("fieldType")
                                    .map_or(Json::Null, |inner| substituted(table, subs, inner)),
                            ),
                        ])
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let other = row
                .get("other")
                .map_or(Json::Null, |other| substituted_row(table, subs, other));
            Json::object(vec![
                ("kind", Json::text("row")),
                ("fields", Json::List(fields)),
                ("other", other),
            ])
        }
        "var" => subs
            .iter()
            .find_map(|sub| match sub {
                Substitution::Row(name, value) if name == text(row, "name") => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| row.clone()),
        _ => row.clone(),
    }
}

fn renamed_variables(written: &mut Json, names: &HashMap<String, String>) {
    if text(written, "kind") == "var"
        && let Some(renamed) = names.get(text(written, "name"))
    {
        put(written, "name", Json::Text(renamed.clone()));
    }
    for_each_child(written, &mut |inner| renamed_variables(inner, names));
    if let Some(row) = field_mut(written, "fields") {
        renamed_row(row, names);
    }
}

fn renamed_row(row: &mut Json, names: &HashMap<String, String>) {
    match text(row, "kind") {
        "var" => {
            if let Some(renamed) = names.get(text(row, "name")) {
                put(row, "name", Json::Text(renamed.clone()));
            }
        }
        "row" => {
            if let Some(Json::List(fields)) = field_mut(row, "fields") {
                for field in fields {
                    if let Some(inner) = field_mut(field, "fieldType") {
                        renamed_variables(inner, names);
                    }
                }
            }
            if let Some(other) = field_mut(row, "other") {
                renamed_row(other, names);
            }
        }
        _ => {}
    }
}

fn for_each_child(written: &mut Json, visit: &mut impl FnMut(&mut Json)) {
    match text(written, "kind") {
        "set" | "list" => {
            if let Some(inner) = field_mut(written, "elem") {
                visit(inner);
            }
        }
        "fun" => {
            for key in ["arg", "res"] {
                if let Some(inner) = field_mut(written, key) {
                    visit(inner);
                }
            }
        }
        "oper" => {
            if let Some(Json::List(args)) = field_mut(written, "args") {
                for inner in args {
                    visit(inner);
                }
            }
            if let Some(inner) = field_mut(written, "res") {
                visit(inner);
            }
        }
        "app" => {
            if let Some(Json::List(args)) = field_mut(written, "args") {
                for inner in args {
                    visit(inner);
                }
            }
        }
        _ => {}
    }
}

fn for_each_row_type(row: &mut Json, visit: &mut impl FnMut(&mut Json)) {
    if text(row, "kind") != "row" {
        return;
    }
    if let Some(Json::List(fields)) = field_mut(row, "fields") {
        for field in fields {
            if let Some(inner) = field_mut(field, "fieldType") {
                visit(inner);
            }
        }
    }
    if let Some(other) = field_mut(row, "other") {
        for_each_row_type(other, visit);
    }
}

impl Applications {
    pub(crate) fn new() -> Self {
        Self {
            fresh: HashMap::new(),
        }
    }

    fn fresh_name(&mut self, prefix: &str) -> String {
        let counter = self.fresh.entry(prefix.to_owned()).or_insert(0);
        let name = format!("{prefix}{counter}");
        *counter += 1;
        name
    }

    fn application(&mut self, table: &Table, written: &Json) -> Result<Json, String> {
        let constructor = written.get("ctor").map_or("", |ctor| text(ctor, "name"));
        let definition = written
            .get("ctor")
            .and_then(id_of)
            .and_then(|id| table.get(id))
            .filter(|entry| entry.text("kind") == "typedef")
            .ok_or_else(|| format!("the type constructor {constructor} has no type definition"))?;
        let body = definition
            .get("type")
            .ok_or_else(|| format!("the type constructor {constructor} has no type"))?;
        let params: Vec<String> = match definition.get("params") {
            Some(Json::List(params)) => params
                .iter()
                .map(|param| match param {
                    Json::Text(name) => name.clone(),
                    _ => String::new(),
                })
                .collect(),
            _ => Vec::new(),
        };
        let (fresh, body) = if params.is_empty() {
            (Vec::new(), body.clone())
        } else {
            let names: Vec<(String, String)> = params
                .iter()
                .map(|param| (param.clone(), self.fresh_name(param)))
                .collect();
            let by_name: HashMap<String, String> = names.iter().cloned().collect();
            let mut body = body.clone();
            renamed_variables(&mut body, &by_name);
            (names.into_iter().map(|(_, fresh)| fresh).collect(), body)
        };
        let args: &[Json] = match written.get("args") {
            Some(Json::List(args)) => args,
            _ => &[],
        };
        if fresh.len() != args.len() {
            let many_or_few = if fresh.len() > args.len() {
                "few"
            } else {
                "many"
            };
            return Err(format!(
                "applying type constructor {constructor}: too {many_or_few} arguments supplied: {constructor} only accepts {} parameters",
                fresh.len()
            ));
        }
        let mut subs = Vec::new();
        for (param, arg) in fresh.iter().zip(args) {
            let resolved = crate::flatten::resolve_alias(table, arg).clone();
            let kind = text(&resolved, "kind").to_owned();
            let fields = resolved.get("fields").cloned();
            let name = resolved.get("name").cloned();
            subs.push(Substitution::Kind(param.clone(), resolved));
            if kind == "rec" || kind == "tup" {
                subs.push(Substitution::Row(
                    param.clone(),
                    fields.unwrap_or(Json::Null),
                ));
            }
            if kind == "var" {
                subs.push(Substitution::Row(
                    param.clone(),
                    Json::object(vec![
                        ("kind", Json::text("var")),
                        ("name", name.unwrap_or(Json::Null)),
                    ]),
                ));
            }
        }
        Ok(substituted(table, &subs, &body))
    }

    fn mapped(&mut self, table: &Table, written: &Json) -> Result<Json, String> {
        let mut new = if text(written, "kind") == "app" {
            self.application(table, written)?
        } else {
            written.clone()
        };
        let mut failed = None;
        for_each_child(&mut new, &mut |inner| match self.mapped(table, inner) {
            Ok(mapped) => *inner = mapped,
            Err(problem) => failed = Some(problem),
        });
        if matches!(text(&new, "kind"), "tup" | "rec" | "sum")
            && let Some(row) = field_mut(&mut new, "fields")
        {
            for_each_row_type(row, &mut |inner| match self.mapped(table, inner) {
                Ok(mapped) => *inner = mapped,
                Err(problem) => failed = Some(problem),
            });
        }
        match failed {
            Some(problem) => Err(problem),
            None => Ok(new),
        }
    }

    fn kind(&mut self, table: &Table, written: &mut Json) -> Result<(), String> {
        let mut failed = None;
        for_each_child(written, &mut |inner| {
            if let Err(problem) = self.kind(table, inner) {
                failed = Some(problem);
            }
        });
        if matches!(text(written, "kind"), "tup" | "rec" | "sum")
            && let Some(row) = field_mut(written, "fields")
        {
            for_each_row_type(row, &mut |inner| {
                if let Err(problem) = self.kind(table, inner) {
                    failed = Some(problem);
                }
            });
        }
        if let Some(problem) = failed {
            return Err(problem);
        }
        *written = self.mapped(table, written)?;
        Ok(())
    }

    fn definition(&mut self, table: &Table, definition: &mut Json) -> Result<(), String> {
        if definition.get("typeAnnotation").is_some() {
            if let Some(written) = field_mut(definition, "typeAnnotation") {
                self.kind(table, written)?;
            }
        } else if text(definition, "kind") == "typedef"
            && let Some(written) = field_mut(definition, "type")
        {
            self.kind(table, written)?;
        }
        let key = match text(definition, "kind") {
            "def" => "expr",
            "assume" => "assumption",
            _ => return Ok(()),
        };
        if let Some(body) = field_mut(definition, key) {
            self.expression(table, body)?;
        }
        Ok(())
    }

    fn expression(&mut self, table: &Table, expression: &mut Json) -> Result<(), String> {
        match text(expression, "kind") {
            "app" => {
                if let Some(Json::List(args)) = field_mut(expression, "args") {
                    for inner in args {
                        self.expression(table, inner)?;
                    }
                }
            }
            "lambda" => {
                if let Some(Json::List(params)) = field_mut(expression, "params") {
                    for param in params {
                        if let Some(written) = field_mut(param, "typeAnnotation") {
                            self.kind(table, written)?;
                        }
                    }
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(table, body)?;
                }
            }
            "let" => {
                if let Some(definition) = field_mut(expression, "opdef") {
                    self.definition(table, definition)?;
                }
                if let Some(body) = field_mut(expression, "expr") {
                    self.expression(table, body)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn declaration(
        &mut self,
        table: &Table,
        declaration: &mut Json,
    ) -> Result<(), String> {
        if text(declaration, "kind") == "instance" {
            if let Some(Json::List(overrides)) = field_mut(declaration, "overrides") {
                for pair in overrides {
                    if let Json::List(pair) = pair
                        && let Some(value) = pair.get_mut(1)
                    {
                        self.expression(table, value)?;
                    }
                }
            }
            return Ok(());
        }
        if matches!(text(declaration, "kind"), "import" | "export") {
            return Ok(());
        }
        self.definition(table, declaration)
    }

    fn entry(&mut self, table: &Table, entry: &Entry) -> Result<Entry, String> {
        if entry.text("kind") == "param" {
            let Some(written) = entry.get("typeAnnotation") else {
                return Ok(entry.clone());
            };
            let mut written = written.clone();
            self.kind(table, &mut written)?;
            return Ok(entry.with("typeAnnotation", Some(written)));
        }
        let mut resolved = entry.clone();
        for key in ["typeAnnotation", "type", "expr", "assumption"] {
            let Some(value) = entry.get(key) else {
                continue;
            };
            let mut value = value.clone();
            match key {
                "typeAnnotation" => self.kind(table, &mut value)?,
                "type"
                    if entry.get("typeAnnotation").is_none() && entry.text("kind") == "typedef" =>
                {
                    self.kind(table, &mut value)?;
                }
                "expr" if entry.text("kind") == "def" => self.expression(table, &mut value)?,
                "assumption" if entry.text("kind") == "assume" => {
                    self.expression(table, &mut value)?;
                }
                _ => continue,
            }
            resolved.set(key, Some(value));
        }
        Ok(resolved)
    }

    pub(crate) fn table(&mut self, table: &mut Table) -> Result<(), String> {
        let order = table.order.clone();
        for id in order {
            let Some(entry) = table.get(id).cloned() else {
                continue;
            };
            let resolved = self.entry(table, &entry)?;
            table.entries.insert(id, resolved);
        }
        Ok(())
    }
}
