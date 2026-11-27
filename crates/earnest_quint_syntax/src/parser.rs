use std::cell::Cell;

use crate::ids::{Ids, Place};
use crate::json::Json;
use crate::lexer::{Kind, Token};
use crate::problem::Problem;
use crate::types::{type_text, unbound_type_variables};

type Span = (usize, usize);

const SEARCHES: usize = 20_000;

pub(crate) const NESTING: usize = 2_000;

const DOT: u8 = 31;
const INDEX: u8 = 28;
const POWER: u8 = 27;
const NEGATION: u8 = 26;
const PRODUCT: u8 = 25;
const SUM: u8 = 24;
const RELATION: u8 = 23;
const ASSIGNMENT: u8 = 22;
const MISTAKEN_EQUALITY: u8 = 21;
const CONJUNCTION: u8 = 19;
const DISJUNCTION: u8 = 17;
const EQUIVALENCE: u8 = 16;
const LEADS: u8 = 15;
const IMPLICATION: u8 = 14;
const PAIR: u8 = 7;
const ELSE_BRANCH: u8 = 4;
const LET_BODY: u8 = 3;

const FUNCTION_TYPE: u8 = 16;
const OPERATOR_TYPE: u8 = 15;
const OPERATOR_RESULT: u8 = 14;

const DECLARATION_FOLLOWERS: [Kind; 15] = [
    Kind::RightBrace,
    Kind::DocComment,
    Kind::Const,
    Kind::Var,
    Kind::Assume,
    Kind::Val,
    Kind::Pure,
    Kind::Type,
    Kind::Def,
    Kind::Action,
    Kind::Run,
    Kind::Temporal,
    Kind::Nondet,
    Kind::Import,
    Kind::Export,
];

const QUALIFIERS: [Kind; 7] = [
    Kind::Val,
    Kind::Def,
    Kind::Pure,
    Kind::Action,
    Kind::Run,
    Kind::Temporal,
    Kind::Nondet,
];

const NAMES: [Kind; 6] = [
    Kind::LowerName,
    Kind::UpperName,
    Kind::From,
    Kind::As,
    Kind::Set,
    Kind::List,
];

const RESERVED: [Kind; 8] = [
    Kind::And,
    Kind::Or,
    Kind::Iff,
    Kind::Implies,
    Kind::LeadsTo,
    Kind::Match,
    Kind::Import,
    Kind::Export,
];

const INFIX_NAMES: [Kind; 5] = [Kind::And, Kind::Or, Kind::Iff, Kind::Implies, Kind::LeadsTo];

const CALL_NAMES: [Kind; 7] = [
    Kind::And,
    Kind::Or,
    Kind::Iff,
    Kind::Implies,
    Kind::LeadsTo,
    Kind::Set,
    Kind::List,
];

pub(crate) struct Parsed {
    pub(crate) modules: Vec<Json>,
    pub(crate) listened: Option<Problem>,
}

#[derive(Clone)]
pub(crate) struct Reported {
    pub(crate) problem: Problem,
    pub(crate) token: usize,
    pub(crate) seen: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fallback {
    Allowed,
    Not,
}

#[derive(Clone, Copy)]
enum Within {
    Module,
    Let,
}

impl Within {
    fn follow(self) -> Follow {
        match self {
            Self::Module => Follow::Declaration,
            Self::Let => Follow::Body,
        }
    }
}

#[derive(Clone, Copy)]
enum Unfollowed {
    TakenBeyond(usize),
    Refused,
}

#[derive(Default)]
struct Found {
    following: Option<Vec<usize>>,
    parsing: Option<Vec<usize>>,
}

enum Follow {
    Declaration,
    Body,
    Closing(Vec<Kind>),
}

#[derive(Clone, Copy)]
enum Pattern {
    Tuple,
    Record,
}

struct Destructured {
    pattern: Pattern,
    names: Vec<String>,
}

enum Definition {
    Plain(Json),
    Destructured(Json, Destructured),
}

struct Variant {
    span: Span,
    label: Span,
    payload: Option<Span>,
    name: String,
    field_type: Json,
}

pub(crate) fn parse(tokens: &[Token], ids: &mut Ids, source: usize) -> Result<Parsed, Reported> {
    let mut parser = Parser {
        tokens,
        at: 0,
        ids,
        source,
        declarations: Vec::new(),
        modules: Vec::new(),
        reported: None,
        listened: None,
        edge: false,
        reached: Cell::new(0),
        searches: 0,
        exhausted: None,
        nesting: 0,
        too_deep: None,
        root: false,
        follows: Vec::new(),
        candidates: Vec::new(),
        stops: Vec::new(),
        forced: None,
        doubtful: Vec::new(),
    };
    let outcome = parser.file();
    if let Some(refusal) = parser.too_deep.take() {
        return Err(refusal);
    }
    if let Some(refusal) = parser.exhausted.take() {
        return Err(refusal);
    }
    match (parser.reported.take(), outcome) {
        (Some(first), _) | (None, Err(first)) => Err(first),
        (None, Ok(())) => {
            let doubtful = std::mem::take(&mut parser.doubtful);
            let mut points: Vec<usize> = doubtful.iter().map(|doubt| doubt.at).collect();
            points.sort_unstable();
            points.dedup();
            if let Some(&point) = points
                .iter()
                .find(|&&point| reads_with_a_name_at(tokens, source, point))
            {
                let value = doubtful
                    .iter()
                    .rev()
                    .find(|doubt| doubt.at == point)
                    .and_then(|doubt| doubt.value);
                return Err(two_readings(tokens, point, value));
            }
            Ok(Parsed {
                modules: parser.modules,
                listened: parser.listened,
            })
        }
    }
}

fn reads_with_a_name_at(tokens: &[Token], source: usize, doubt: usize) -> bool {
    let mut ids = Ids::default();
    let mut parser = Parser {
        tokens,
        at: 0,
        ids: &mut ids,
        source,
        declarations: Vec::new(),
        modules: Vec::new(),
        reported: None,
        listened: None,
        edge: false,
        reached: Cell::new(0),
        searches: 0,
        exhausted: None,
        nesting: 0,
        too_deep: None,
        root: false,
        follows: Vec::new(),
        candidates: Vec::new(),
        stops: Vec::new(),
        forced: Some(doubt),
        doubtful: Vec::new(),
    };
    let outcome = parser.file();
    outcome.is_ok()
        && parser.too_deep.is_none()
        && parser.exhausted.is_none()
        && parser.reported.is_some_and(|reported| {
            reported.token == doubt + 1 && reported.problem.code == "QNT008"
        })
}

fn two_readings(tokens: &[Token], doubt: usize, value: Option<Span>) -> Reported {
    let place = |at: usize| {
        let token = &tokens[at.min(tokens.len() - 1)];
        (token.line, token.column + 1)
    };
    let token = &tokens[doubt.min(tokens.len() - 1)];
    let advice = value.map_or_else(
        || "put the value of the definition before it in parentheses".to_owned(),
        |(start, end)| {
            let (from_line, from_column) = place(start);
            let (to_line, to_column) = place(end);
            let to_column = to_column + tokens[end.min(tokens.len() - 1)].text.chars().count();
            format!(
                "put that definition's value, from {from_line}:{from_column} to before {to_line}:{to_column}, in parentheses"
            )
        },
    );
    Reported {
        problem: Problem::coded(
            "unreadable",
            format!(
                "Quint's grammar reads this `{}` two ways: as written here, or as a name, which Quint refuses with QNT008; earnest can't tell which of the two Quint takes, so {advice}",
                token.text
            ),
            token.line - 1,
            token.column,
        ),
        token: doubt,
        seen: doubt,
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    at: usize,
    ids: &'a mut Ids,
    source: usize,
    declarations: Vec<Json>,
    modules: Vec<Json>,
    reported: Option<Reported>,
    listened: Option<Problem>,
    edge: bool,
    reached: Cell<usize>,
    searches: usize,
    exhausted: Option<Reported>,
    nesting: usize,
    too_deep: Option<Reported>,
    root: bool,
    follows: Vec<Follow>,
    candidates: Vec<usize>,
    stops: Vec<usize>,
    forced: Option<usize>,
    doubtful: Vec<Doubt>,
}

struct Doubt {
    at: usize,
    value: Option<Span>,
}

struct Mark {
    at: usize,
    ids: (u64, usize),
    reported: Option<Reported>,
    listened: Option<Problem>,
    candidates: usize,
    stops: usize,
    declarations: usize,
}

fn id(value: u64) -> Json {
    Json::number(value)
}

fn app(id_value: u64, opcode: &str, args: Vec<Json>) -> Json {
    Json::object(vec![
        ("id", id(id_value)),
        ("kind", Json::text("app")),
        ("opcode", Json::text(opcode)),
        ("args", Json::List(args)),
    ])
}

fn literal(id_value: u64, kind: &str, value: Json) -> Json {
    Json::object(vec![
        ("id", id(id_value)),
        ("kind", Json::text(kind)),
        ("value", value),
    ])
}

fn name_expr(id_value: u64, name: &str) -> Json {
    Json::object(vec![
        ("id", id(id_value)),
        ("kind", Json::text("name")),
        ("name", Json::text(name)),
    ])
}

fn param(id_value: u64, name: &str) -> Json {
    Json::object(vec![("id", id(id_value)), ("name", Json::text(name))])
}

fn lambda(id_value: u64, params: Vec<Json>, qualifier: &str, body: Json) -> Json {
    Json::object(vec![
        ("id", id(id_value)),
        ("kind", Json::text("lambda")),
        ("params", Json::List(params)),
        ("qualifier", Json::text(qualifier)),
        ("expr", body),
    ])
}

fn let_in(id_value: u64, definition: Json, body: Json) -> Json {
    Json::object(vec![
        ("id", id(id_value)),
        ("kind", Json::text("let")),
        ("opdef", definition),
        ("expr", body),
    ])
}

fn def(id_value: u64, name: &str, qualifier: &str, body: Json) -> Json {
    Json::object(vec![
        ("id", id(id_value)),
        ("kind", Json::text("def")),
        ("name", Json::text(name)),
        ("qualifier", Json::text(qualifier)),
        ("expr", body),
    ])
}

fn unit(id_value: u64) -> Json {
    app(id_value, "Tup", Vec::new())
}

fn empty_row() -> Json {
    Json::object(vec![("kind", Json::text("empty"))])
}

fn row(fields: Vec<Json>, other: Json) -> Json {
    Json::object(vec![
        ("kind", Json::text("row")),
        ("fields", Json::List(fields)),
        ("other", other),
    ])
}

fn type_node(id_value: u64, kind: &str, rest: Vec<(&str, Json)>) -> Json {
    let mut fields = vec![("id", id(id_value)), ("kind", Json::text(kind))];
    fields.extend(rest);
    Json::object(fields)
}

fn is_unit_type(value: &Json) -> bool {
    value.get("kind") == Some(&Json::text("tup"))
        && value.get("fields").and_then(|fields| fields.get("kind")) == Some(&Json::text("row"))
        && matches!(
            value.get("fields").and_then(|fields| fields.get("fields")),
            Some(Json::List(list)) if list.is_empty()
        )
        && value
            .get("fields")
            .and_then(|fields| fields.get("other"))
            .and_then(|other| other.get("kind"))
            == Some(&Json::text("empty"))
}

fn attach_doc(mut declaration: Json, doc: &str) -> Json {
    if let Json::Object(fields) = &mut declaration {
        let mut documented = vec![("doc".to_owned(), Json::text(doc))];
        documented.append(fields);
        *fields = documented;
    }
    declaration
}

fn doc_text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| {
            let units: Vec<u16> = token.text.encode_utf16().collect();
            let end = units.len().saturating_sub(1);
            units
                .get(4..end)
                .map_or_else(String::new, String::from_utf16_lossy)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl Parser<'_> {
    fn kind(&self) -> Kind {
        self.kind_at(self.at)
    }

    fn kind_at(&self, at: usize) -> Kind {
        self.tokens.get(at).map_or(Kind::End, |token| token.kind)
    }

    fn token(&self, at: usize) -> &Token {
        &self.tokens[at.min(self.tokens.len() - 1)]
    }

    fn last(&self) -> usize {
        self.at.saturating_sub(1)
    }

    fn span(&self, start: usize) -> Span {
        (start, self.last())
    }

    fn text(&self, (start, stop): Span) -> String {
        self.tokens[start..=stop.max(start).min(self.tokens.len() - 1)]
            .iter()
            .filter(|token| token.kind != Kind::End)
            .map(|token| token.text.as_str())
            .collect()
    }

    fn place(&self, (start, stop): Span) -> Place {
        Place::of(self.source, self.token(start), self.token(stop))
    }

    fn placed(&mut self, span: Span) -> u64 {
        let place = self.place(span);
        self.ids.placed(place)
    }

    fn advance(&mut self) -> usize {
        let at = self.at;
        if self.kind() != Kind::End {
            self.at += 1;
        }
        at
    }

    fn failure(&self, message: String) -> Reported {
        let token = self.token(self.at);
        self.reached.set(self.reached.get().max(self.at));
        Reported {
            problem: Problem::syntax(message, token.line - 1, token.column),
            token: self.at,
            seen: self.reached.get(),
        }
    }

    fn unexpected<T>(&self, expected: &str) -> Result<T, Reported> {
        let shown = self
            .token(self.at)
            .text
            .trim_end_matches(['\r', '\n'])
            .replace('\r', "\\r")
            .replace('\n', "\\n");
        Err(self.failure(format!("unexpected '{shown}', expected {expected}")))
    }

    fn expect(&mut self, kind: Kind, shown: &str) -> Result<usize, Reported> {
        if self.kind() == kind {
            Ok(self.advance())
        } else {
            self.unexpected(&format!("'{shown}'"))
        }
    }

    fn eat(&mut self, kind: Kind) -> bool {
        if self.kind() == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn report(&mut self, code: &str, message: String) {
        if self.reported.is_none() {
            let token = self.token(self.at);
            self.reported = Some(Reported {
                problem: Problem::coded(code, message, token.line - 1, token.column),
                token: self.at,
                seen: self.reached.get().max(self.at),
            });
        }
    }

    fn listen(&mut self, code: &str, message: String, at: u64) {
        if self.listened.is_none() {
            let place = self.ids.place(at).copied();
            let (line, column) = place.map_or((0, 0), |place| {
                (
                    place.start.line,
                    usize::try_from(place.start.column).unwrap_or(0),
                )
            });
            self.listened = Some(Problem::coded(code, message, line, column));
        }
    }

    fn mark(&self) -> Mark {
        Mark {
            at: self.at,
            ids: self.ids.mark(),
            reported: self.reported.clone(),
            listened: self.listened.clone(),
            candidates: self.candidates.len(),
            stops: self.stops.len(),
            declarations: self.declarations.len(),
        }
    }

    fn rewind(&mut self, mark: &Mark) {
        self.at = mark.at;
        self.ids.rewind(mark.ids);
        self.reported.clone_from(&mark.reported);
        self.listened.clone_from(&mark.listened);
        self.candidates.truncate(mark.candidates);
        self.stops.truncate(mark.stops);
        self.declarations.truncate(mark.declarations);
    }

    fn goes_on(&mut self, at: usize) -> bool {
        self.decides(at * 2)
    }

    fn reads_keyword(&mut self, at: usize) -> bool {
        self.decides(at * 2 + 1)
    }

    fn decides(&mut self, decision: usize) -> bool {
        if !self.edge {
            return true;
        }
        if self.stops.contains(&decision) {
            return false;
        }
        self.candidates.push(decision);
        true
    }

    fn doubts(&mut self, at: usize) -> bool {
        if self.forced == Some(at) {
            return true;
        }
        self.doubtful.push(Doubt { at, value: None });
        false
    }

    fn definition_value(&mut self, within: Within) -> Result<Json, Reported> {
        let start = self.at;
        let recorded = self.doubtful.len();
        let value = self.followed_expr(within.follow());
        let end = self.last();
        for doubt in self.doubtful.iter_mut().skip(recorded) {
            doubt.value.get_or_insert((start, end));
        }
        value
    }

    fn may_start_body(&self, kind: Kind) -> bool {
        let next = self.kind_at(self.at + 1);
        match kind {
            Kind::Minus | Kind::LeftBracket => true,
            Kind::And | Kind::Or => matches!(next, Kind::LeftBrace | Kind::LeftParen),
            Kind::Iff | Kind::Implies | Kind::LeadsTo => next == Kind::LeftParen,
            _ => false,
        }
    }

    fn attempt<T>(&mut self, parse: impl FnOnce(&mut Self) -> Result<T, Reported>) -> Option<T> {
        let mark = self.mark();
        let parsed = parse(self).ok();
        if parsed.is_none() {
            self.rewind(&mark);
        }
        parsed
    }

    fn contest<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, Reported>,
        other: usize,
        other_parse: fn(&mut Self) -> Result<Json, Reported>,
    ) -> Result<Option<T>, Reported> {
        let mark = self.mark();
        let failed = match parse(self) {
            Ok(parsed) => return Ok(Some(parsed)),
            Err(failed) => failed,
        };
        let reported = self.reported.take();
        self.rewind(&mark);
        if !self.searching() {
            return Ok(None);
        }
        self.at = other;
        let reach =
            other_parse(self).map_or_else(|other_failed| other_failed.token, |_| usize::MAX);
        self.rewind(&mark);
        if reach >= failed.token {
            Ok(None)
        } else {
            self.reported = reported;
            Err(failed)
        }
    }

    fn searching(&mut self) -> bool {
        self.searches += 1;
        let within = self.searches <= SEARCHES;
        if !within && self.exhausted.is_none() {
            let token = self.token(self.at);
            self.exhausted = Some(Reported {
                problem: Problem::coded(
                    "unreadable",
                    format!(
                        "this file needs more than {SEARCHES} attempts to read as Quint reads it, and earnest gives up rather than guess: put the values of nested `let`s in parentheses"
                    ),
                    token.line - 1,
                    token.column,
                ),
                token: self.at,
                seen: self.at,
            });
        }
        within
    }

    fn parenthesized_here(&mut self) -> Result<Json, Reported> {
        let at = self.at;
        self.parenthesized(at)
    }

    fn braces_here(&mut self) -> Result<Json, Reported> {
        let at = self.at;
        self.braces(at)
    }

    fn operand_starts(&mut self) -> bool {
        let mark = self.mark();
        self.advance();
        let operand = self.mark();
        let mut started = self.primary().is_ok();
        let mut candidates = self
            .candidates
            .get(operand.candidates..)
            .unwrap_or_default()
            .to_vec();
        candidates.sort_unstable();
        candidates.dedup();
        for candidate in candidates.into_iter().rev() {
            if started || !self.searching() {
                break;
            }
            self.rewind(&operand);
            self.stops.push(candidate);
            started = self.primary().is_ok();
        }
        self.rewind(&mark);
        started
    }

    fn plain_name(&mut self, start: usize) -> Result<Json, Reported> {
        let name = self.qualified_name()?;
        let name_id = self.placed(self.span(start));
        Ok(name_expr(name_id, &self.text(name)))
    }

    fn root_expr(&mut self) -> Result<Json, Reported> {
        self.root = true;
        self.expr(0)
    }

    fn enclosed(&mut self, closers: &[Kind]) -> Result<Json, Reported> {
        let edge = std::mem::replace(&mut self.edge, false);
        let value = self.followed_expr(Follow::Closing(closers.to_vec()));
        self.edge = edge;
        value
    }

    fn followed_expr(&mut self, follow: Follow) -> Result<Json, Reported> {
        self.follows.push(follow);
        let value = self.root_expr();
        self.follows.pop();
        value
    }

    fn can_follow(&self) -> bool {
        let kind = self.kind();
        match self.follows.last() {
            None => true,
            Some(Follow::Declaration) => DECLARATION_FOLLOWERS.contains(&kind),
            Some(Follow::Closing(closers)) => closers.contains(&kind),
            Some(Follow::Body) => kind == Kind::Semicolon || self.starts_expression(),
        }
    }

    fn starts_expression(&self) -> bool {
        let kind = self.kind();
        self.is_identifier(self.at)
            || QUALIFIERS.contains(&kind)
            || CALL_NAMES.contains(&kind)
            || matches!(
                kind,
                Kind::Minus
                    | Kind::If
                    | Kind::All
                    | Kind::Any
                    | Kind::LeftBracket
                    | Kind::LeftBrace
                    | Kind::LeftParen
                    | Kind::Hole
                    | Kind::Number
                    | Kind::Truth
                    | Kind::Text
            )
    }

    fn file(&mut self) -> Result<(), Reported> {
        self.eat(Kind::Hashbang);
        loop {
            self.module()?;
            if !matches!(self.kind(), Kind::DocComment | Kind::Module) {
                break;
            }
        }
        if self.kind() == Kind::End {
            Ok(())
        } else {
            self.unexpected("'module' or the end of the file")
        }
    }

    fn docs(&mut self) -> String {
        let start = self.at;
        while self.kind() == Kind::DocComment {
            self.advance();
        }
        doc_text(&self.tokens[start..self.at])
    }

    fn module(&mut self) -> Result<(), Reported> {
        let start = self.at;
        let doc = self.docs();
        self.expect(Kind::Module, "module")?;
        let name = self.qualified_name()?;
        self.expect(Kind::LeftBrace, "{")?;
        while self.kind() != Kind::RightBrace {
            if self.kind() == Kind::End {
                return self.unexpected("'}' or a declaration");
            }
            self.documented_declaration()?;
        }
        self.advance();
        let module_id = self.placed(self.span(start));
        let declarations = std::mem::take(&mut self.declarations);
        let mut fields = vec![
            ("id", id(module_id)),
            ("name", Json::text(&self.text(name))),
            ("declarations", Json::List(declarations)),
        ];
        if !doc.is_empty() {
            fields.push(("doc", Json::text(&doc)));
        }
        self.modules.push(Json::object(fields));
        Ok(())
    }

    fn documented_declaration(&mut self) -> Result<(), Reported> {
        let doc = self.docs();
        self.declaration()?;
        if !doc.is_empty()
            && let Some(last) = self.declarations.pop()
        {
            self.declarations.push(attach_doc(last, &doc));
        }
        Ok(())
    }

    fn declaration(&mut self) -> Result<(), Reported> {
        let start = self.at;
        match self.kind() {
            Kind::Const | Kind::Var => {
                let kind = if self.kind() == Kind::Const {
                    "const"
                } else {
                    "var"
                };
                self.advance();
                let name = self.qualified_name()?;
                self.expect(Kind::Colon, ":")?;
                let annotation = self.type_expr(0)?;
                let declaration_id = self.placed(self.span(start));
                self.declarations.push(Json::object(vec![
                    ("kind", Json::text(kind)),
                    ("name", Json::text(&self.text(name))),
                    ("typeAnnotation", annotation),
                    ("id", id(declaration_id)),
                ]));
            }
            Kind::Assume => {
                self.advance();
                let name = self.ident_or_hole()?;
                self.expect(Kind::Assign, "=")?;
                let assumption = self.followed_expr(Follow::Declaration)?;
                let declaration_id = self.placed(self.span(start));
                self.declarations.push(Json::object(vec![
                    ("id", id(declaration_id)),
                    ("kind", Json::text("assume")),
                    ("name", Json::text(&self.text(name))),
                    ("assumption", assumption),
                ]));
            }
            Kind::Import => {
                if self.kind_at(self.after_qualified(self.at + 1)) == Kind::LeftParen {
                    self.instance()?;
                } else {
                    self.import_or_export()?;
                }
            }
            Kind::Export => self.import_or_export()?,
            Kind::Type => self.type_definition()?,
            kind if QUALIFIERS.contains(&kind) => match self.operator_definition(Within::Module)? {
                Definition::Plain(definition) => self.declarations.push(definition),
                Definition::Destructured(temporary, _) => self.declarations.push(temporary),
            },
            _ => return self.unexpected("a declaration"),
        }
        Ok(())
    }

    fn after_qualified(&self, start: usize) -> usize {
        let mut at = start;
        if !self.is_identifier(at) {
            return at;
        }
        at += 1;
        while self.kind_at(at) == Kind::Path && self.is_identifier(at + 1) {
            at += 2;
        }
        at
    }

    fn is_identifier(&self, at: usize) -> bool {
        let kind = self.kind_at(at);
        NAMES.contains(&kind) || RESERVED.contains(&kind)
    }

    fn identifier(&mut self) -> Result<(), Reported> {
        let kind = self.kind();
        if NAMES.contains(&kind) {
            self.advance();
            Ok(())
        } else if RESERVED.contains(&kind) {
            let shown = self.token(self.at).text.clone();
            self.advance();
            self.report(
                "QNT008",
                format!("Reserved keyword '{shown}' cannot be used as an identifier."),
            );
            Ok(())
        } else {
            self.unexpected("a name")
        }
    }

    fn qualified_name(&mut self) -> Result<Span, Reported> {
        let start = self.at;
        self.identifier()?;
        while self.kind() == Kind::Path {
            self.advance();
            self.identifier()?;
        }
        Ok(self.span(start))
    }

    fn simple_name(&mut self, context: &str) -> Result<Span, Reported> {
        let start = self.at;
        if self.kind_at(self.at + 1) == Kind::Path {
            let span = self.qualified_name()?;
            let shown = self.text(span);
            self.report(
                "QNT008",
                format!("Identifiers in a {context} cannot be qualified with '::'. Found {shown}."),
            );
            return Ok(span);
        }
        self.identifier()?;
        Ok(self.span(start))
    }

    fn ident_or_hole(&mut self) -> Result<Span, Reported> {
        if self.kind() == Kind::Hole {
            let at = self.advance();
            Ok((at, at))
        } else {
            self.qualified_name()
        }
    }

    fn source_clause(&mut self) -> Result<Option<String>, Reported> {
        if self.eat(Kind::From) {
            let at = self.expect(Kind::Text, "a string")?;
            let text: Vec<char> = self.token(at).text.chars().collect();
            Ok(Some(text[1..text.len() - 1].iter().collect()))
        } else {
            Ok(None)
        }
    }

    fn import_or_export(&mut self) -> Result<(), Reported> {
        let import = self.kind() == Kind::Import;
        let start = self.advance();
        let proto = self.qualified_name()?;
        let mut definition = None;
        let mut qualifier = None;
        if self.eat(Kind::Dot) {
            if self.eat(Kind::Star) {
                definition = Some("*".to_owned());
            } else {
                let name = self.qualified_name()?;
                definition = Some(self.text(name));
            }
        } else if self.eat(Kind::As) {
            qualifier = Some(self.qualified_name()?);
        }
        let source = if import { self.source_clause()? } else { None };
        let declaration_id = self.placed(self.span(start));
        let mut fields = vec![
            ("id", id(declaration_id)),
            ("kind", Json::text(if import { "import" } else { "export" })),
        ];
        if let Some(definition) = definition {
            fields.push(("defName", Json::Text(definition)));
        }
        fields.push(("protoName", Json::text(&self.text(proto))));
        if let Some(qualifier) = qualifier {
            fields.push(("qualifiedName", Json::text(&self.text(qualifier))));
        }
        if let Some(source) = source {
            fields.push(("fromSource", Json::Text(source)));
        }
        self.declarations.push(Json::object(fields));
        Ok(())
    }

    fn instance(&mut self) -> Result<(), Reported> {
        let start = self.advance();
        let proto = self.qualified_name()?;
        self.expect(Kind::LeftParen, "(")?;
        let mut overrides = Vec::new();
        loop {
            let name = self.qualified_name()?;
            self.expect(Kind::Assign, "=")?;
            let value = self.followed_expr(Follow::Closing(vec![Kind::Comma, Kind::RightParen]))?;
            overrides.push((name, value));
            if self.kind() == Kind::Comma && self.kind_at(self.at + 1) != Kind::RightParen {
                self.advance();
            } else {
                break;
            }
        }
        self.eat(Kind::Comma);
        self.expect(Kind::RightParen, ")")?;
        let mut qualifier = None;
        let identity = if self.eat(Kind::Dot) {
            self.expect(Kind::Star, "*")?;
            true
        } else {
            self.expect(Kind::As, "as")?;
            qualifier = Some(self.qualified_name()?);
            false
        };
        let source = self.source_clause()?;
        let mut written = Vec::new();
        for (name, value) in overrides {
            let name_id = self.placed(name);
            written.push(Json::List(vec![param(name_id, &self.text(name)), value]));
        }
        let declaration_id = self.placed(self.span(start));
        let mut fields = vec![("id", id(declaration_id)), ("kind", Json::text("instance"))];
        if let Some(qualifier) = qualifier {
            fields.push(("qualifiedName", Json::text(&self.text(qualifier))));
        }
        fields.push(("protoName", Json::text(&self.text(proto))));
        fields.push(("overrides", Json::List(written)));
        fields.push(("identityOverride", Json::Bool(identity)));
        if let Some(source) = source {
            fields.push(("fromSource", Json::Text(source)));
        }
        self.declarations.push(Json::object(fields));
        Ok(())
    }

    fn type_definition(&mut self) -> Result<(), Reported> {
        let start = self.advance();
        let name = self.qualified_name()?;
        if !matches!(self.kind(), Kind::Assign | Kind::LeftBracket) {
            let definition_id = self.placed(self.span(start));
            self.check_type_name(definition_id, name);
            self.declarations.push(Json::object(vec![
                ("id", id(definition_id)),
                ("kind", Json::text("typedef")),
                ("name", Json::text(&self.text(name))),
            ]));
            return Ok(());
        }
        let mut params = Vec::new();
        if self.eat(Kind::LeftBracket) {
            loop {
                let at = self.expect(Kind::LowerName, "a type variable")?;
                params.push(self.token(at).text.clone());
                if !self.eat(Kind::Comma) {
                    break;
                }
            }
            self.expect(Kind::RightBracket, "]")?;
        }
        self.expect(Kind::Assign, "=")?;
        if self.is_sum_definition() {
            self.sum_definition(start, name, &params)
        } else {
            let aliased = self.type_expr(0)?;
            let definition_id = self.placed(self.span(start));
            self.check_type_name(definition_id, name);
            let mut fields = vec![
                ("id", id(definition_id)),
                ("kind", Json::text("typedef")),
                ("name", Json::text(&self.text(name))),
                ("type", aliased),
            ];
            if !params.is_empty() {
                fields.push((
                    "params",
                    Json::List(params.iter().map(|param| Json::text(param)).collect()),
                ));
            }
            let definition = Json::object(fields);
            self.check_type_variables(definition_id, &definition, &params);
            self.declarations.push(definition);
            Ok(())
        }
    }

    fn check_type_name(&mut self, at: u64, name: Span) {
        let written = self.text(name);
        let last = written.rsplit("::").next().unwrap_or_default();
        if last
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        {
            self.listen(
                "QNT007",
                "type names must start with an uppercase letter".to_owned(),
                at,
            );
        }
    }

    fn check_type_variables(&mut self, at: u64, definition: &Json, params: &[String]) {
        if let Some(defined) = definition.get("type") {
            let unbound: Vec<String> = unbound_type_variables(defined)
                .into_iter()
                .filter(|name| !params.contains(name))
                .collect();
            if !unbound.is_empty() {
                self.listen(
                    "QNT014",
                    format!(
                        "the type variables {} are unbound.\nE.g., in\n\n   type T = List[a]\n\ntype variable 'a' is unbound. To fix it, write\n\n   type T[a] = List[a]",
                        unbound.join(", ")
                    ),
                    at,
                );
            }
        }
    }

    fn is_sum_definition(&self) -> bool {
        let mut at = self.at;
        if self.kind_at(at) == Kind::Pipe {
            at += 1;
        }
        loop {
            let after = self.after_qualified(at);
            if after == at {
                return false;
            }
            at = after;
            if self.kind_at(at) == Kind::LeftParen {
                match self.closing(at) {
                    Some((close, false)) => at = close + 1,
                    _ => return false,
                }
            }
            if self.kind_at(at) == Kind::Pipe {
                at += 1;
            } else {
                return DECLARATION_FOLLOWERS.contains(&self.kind_at(at));
            }
        }
    }

    fn closing(&self, open: usize) -> Option<(usize, bool)> {
        let mut depth = 0usize;
        let mut comma = false;
        let mut at = open;
        loop {
            match self.kind_at(at) {
                Kind::LeftParen | Kind::LeftBracket | Kind::LeftBrace => depth += 1,
                Kind::RightParen | Kind::RightBracket | Kind::RightBrace => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some((at, comma));
                    }
                }
                Kind::Comma if depth == 1 => comma = true,
                Kind::End => return None,
                _ => {}
            }
            at += 1;
        }
    }

    fn variant(&mut self) -> Result<Variant, Reported> {
        let start = self.at;
        let label = self.simple_name("variant label")?;
        let mut payload = None;
        if self.eat(Kind::LeftParen) {
            let type_start = self.at;
            let written = self.type_expr(0)?;
            payload = Some((self.span(type_start), written));
            self.expect(Kind::RightParen, ")")?;
        }
        let span = self.span(start);
        let (payload, field_type) = if let Some((type_span, written)) = payload {
            (Some(type_span), written)
        } else {
            let unit_id = self.placed(span);
            let unit_type = type_node(
                unit_id,
                "tup",
                vec![("fields", row(Vec::new(), empty_row()))],
            );
            (None, unit_type)
        };
        Ok(Variant {
            span,
            label,
            payload,
            name: self.text(label),
            field_type,
        })
    }

    fn sum_definition(
        &mut self,
        start: usize,
        name: Span,
        params: &[String],
    ) -> Result<(), Reported> {
        let separator = self.eat(Kind::Pipe);
        let mut variants = vec![self.variant()?];
        while self.eat(Kind::Pipe) {
            variants.push(self.variant()?);
        }
        let span = self.span(start);
        let definition_id = self.placed(span);
        let type_name = self.text(name);
        let fields: Vec<Json> = variants
            .iter()
            .map(|variant| {
                Json::object(vec![
                    ("fieldName", Json::text(&variant.name)),
                    ("fieldType", variant.field_type.clone()),
                ])
            })
            .collect();
        let sum = type_node(
            definition_id,
            "sum",
            vec![("fields", row(fields, empty_row()))],
        );
        let mut definition_fields = vec![
            ("id", id(definition_id)),
            ("kind", Json::text("typedef")),
            ("name", Json::text(&type_name)),
            ("type", sum),
        ];
        if !params.is_empty() {
            definition_fields.push((
                "params",
                Json::List(params.iter().map(|param| Json::text(param)).collect()),
            ));
        }
        let definition = Json::object(definition_fields);
        self.check_type_variables(definition_id, &definition, params);
        let constant_id = self.placed(span);
        let constant = type_node(constant_id, "const", vec![("name", Json::text(&type_name))]);
        let returned = if params.is_empty() {
            constant
        } else {
            let mut args = Vec::new();
            for param in params {
                let var_id = self.placed(span);
                args.push(type_node(var_id, "var", vec![("name", Json::text(param))]));
            }
            type_node(
                definition_id,
                "app",
                vec![("ctor", constant), ("args", Json::List(args))],
            )
        };
        if variants.len() == 1 && is_unit_type(&variants[0].field_type) && !separator {
            let variant = &variants[0];
            let alias_id = self.placed(variant.span);
            let written = self.text(variant.span);
            self.declarations.push(Json::object(vec![
                ("id", id(definition_id)),
                ("kind", Json::text("typedef")),
                ("name", Json::text(&type_name)),
                (
                    "type",
                    type_node(alias_id, "const", vec![("name", Json::Text(written))]),
                ),
            ]));
            return Ok(());
        }
        self.declarations.push(definition);
        for variant in &variants {
            let constructor = self.constructor(variant, &returned);
            self.declarations.push(constructor);
        }
        Ok(())
    }

    fn constructor(&mut self, variant: &Variant, returned: &Json) -> Json {
        let label_id = self.placed(variant.span);
        let label = literal(label_id, "str", Json::text(&variant.name));
        let (qualifier, annotation, body) = if is_unit_type(&variant.field_type) {
            let wrapped_id = self.placed(variant.label);
            let variant_id = self.placed(variant.span);
            let body = app(variant_id, "variant", vec![label, unit(wrapped_id)]);
            ("val", returned.clone(), body)
        } else {
            let param_name = format!("__{}Param", variant.name);
            let param_id = self.placed(variant.payload.unwrap_or(variant.span));
            let wrapped_id = self.placed(variant.label);
            let variant_id = self.placed(variant.span);
            let made = app(
                variant_id,
                "variant",
                vec![
                    label,
                    Json::object(vec![
                        ("kind", Json::text("name")),
                        ("name", Json::text(&param_name)),
                        ("id", id(wrapped_id)),
                    ]),
                ],
            );
            let annotation = Json::object(vec![
                ("kind", Json::text("oper")),
                ("args", Json::List(vec![variant.field_type.clone()])),
                ("res", returned.clone()),
            ]);
            let lambda_id = self.placed(variant.span);
            let body = lambda(lambda_id, vec![param(param_id, &param_name)], "def", made);
            ("def", annotation, body)
        };
        let constructor_id = self.placed(variant.span);
        Json::object(vec![
            ("id", id(constructor_id)),
            ("kind", Json::text("def")),
            ("name", Json::text(&variant.name)),
            ("qualifier", Json::text(qualifier)),
            ("typeAnnotation", annotation),
            ("expr", body),
        ])
    }

    fn qualifier(&mut self) -> Result<String, Reported> {
        let start = self.at;
        match self.kind() {
            Kind::Pure => {
                self.advance();
                if !matches!(self.kind(), Kind::Val | Kind::Def) {
                    return self.unexpected("'val' or 'def'");
                }
                self.advance();
            }
            kind if QUALIFIERS.contains(&kind) => {
                self.advance();
            }
            _ => return self.unexpected("a qualifier"),
        }
        Ok(self.text(self.span(start)))
    }

    fn is_destructuring(&self) -> bool {
        let pattern = |at: usize| matches!(self.kind_at(at), Kind::LeftParen | Kind::LeftBrace);
        match self.kind() {
            Kind::Val => pattern(self.at + 1),
            Kind::Pure => self.kind_at(self.at + 1) == Kind::Val && pattern(self.at + 2),
            _ => false,
        }
    }

    fn operator_definition(&mut self, within: Within) -> Result<Definition, Reported> {
        let start = self.at;
        if self.is_destructuring() {
            return self.destructuring(start, within);
        }
        let qualifier = self.qualifier()?;
        let call_start = self.at;
        if CALL_NAMES.contains(&self.kind()) {
            self.advance();
        } else {
            self.qualified_name()?;
        }
        let call_name = self.span(call_start);
        let annotated = self.kind() == Kind::LeftParen && {
            let after = if self.kind_at(self.at + 1) == Kind::Hole {
                self.at + 2
            } else {
                self.after_qualified(self.at + 1)
            };
            after > self.at + 1 && self.kind_at(after) == Kind::Colon
        };
        let mut params = Vec::new();
        let mut annotation = None;
        if annotated {
            self.advance();
            let mut argument_types = Vec::new();
            loop {
                let param_start = self.at;
                let name = self.ident_or_hole()?;
                self.expect(Kind::Colon, ":")?;
                let param_type = self.type_expr(0)?;
                let param_id = self.placed(self.span(param_start));
                argument_types.push(param_type.clone());
                params.push(Json::object(vec![
                    ("id", id(param_id)),
                    ("name", Json::text(&self.text(name))),
                    ("typeAnnotation", param_type),
                ]));
                if self.kind() == Kind::Comma && self.kind_at(self.at + 1) != Kind::RightParen {
                    self.advance();
                } else {
                    break;
                }
            }
            self.eat(Kind::Comma);
            self.expect(Kind::RightParen, ")")?;
            self.expect(Kind::Colon, ":")?;
            let result = self.type_expr(0)?;
            annotation = Some(Json::object(vec![
                ("kind", Json::text("oper")),
                ("args", Json::List(argument_types)),
                ("res", result),
            ]));
        } else {
            params = self.unannotated_parameters()?;
            if self.eat(Kind::Colon) {
                annotation = Some(self.type_expr(0)?);
            }
        }
        let body = if self.eat(Kind::Assign) {
            Some(self.definition_value(within)?)
        } else {
            None
        };
        self.eat(Kind::Semicolon);
        let span = self.span(start);
        let body = if let Some(body) = body {
            body
        } else {
            let header_id = self.placed(span);
            literal(header_id, "bool", Json::Bool(true))
        };
        let body = if params.is_empty() {
            body
        } else {
            let lambda_id = self.placed(span);
            lambda(lambda_id, params, &qualifier, body)
        };
        let definition_id = self.placed(call_name);
        let mut definition = def(definition_id, &self.text(call_name), &qualifier, body);
        if let (Some(annotation), Json::Object(fields)) = (annotation, &mut definition) {
            fields.push(("typeAnnotation".to_owned(), annotation));
        }
        Ok(Definition::Plain(definition))
    }

    fn destructuring(&mut self, start: usize, within: Within) -> Result<Definition, Reported> {
        let qualifier = if self.eat(Kind::Pure) {
            "pureval"
        } else {
            "val"
        };
        self.expect(Kind::Val, "val")?;
        let pattern;
        let mut names = Vec::new();
        if self.eat(Kind::LeftParen) {
            pattern = Pattern::Tuple;
            let first = self.ident_or_hole()?;
            names.push(self.text(first));
            loop {
                self.expect(Kind::Comma, ",")?;
                let next = self.ident_or_hole()?;
                names.push(self.text(next));
                if self.kind() != Kind::Comma {
                    break;
                }
            }
            self.expect(Kind::RightParen, ")")?;
        } else {
            self.expect(Kind::LeftBrace, "{")?;
            pattern = Pattern::Record;
            loop {
                let field = self.simple_name("record field")?;
                names.push(self.text(field));
                if !self.eat(Kind::Comma) {
                    break;
                }
            }
            self.expect(Kind::RightBrace, "}")?;
        }
        self.expect(Kind::Assign, "=")?;
        let value = self.definition_value(within)?;
        self.eat(Kind::Semicolon);
        let temporary_id = self.placed(self.span(start));
        let temporary_name = format!("quintDestructTemp{temporary_id}");
        let temporary = def(temporary_id, &temporary_name, qualifier, value);
        let destructured = Destructured { pattern, names };
        if matches!(within, Within::Let) {
            return Ok(Definition::Destructured(temporary, destructured));
        }
        self.declarations.push(temporary);
        for (index, name) in destructured.names.iter().enumerate() {
            if name != "_" {
                let field = self.destructured_field(
                    destructured.pattern,
                    &temporary_name,
                    index,
                    name,
                    qualifier,
                );
                self.declarations.push(field);
            }
        }
        let last = self.declarations.pop();
        Ok(Definition::Plain(last.unwrap_or(Json::Null)))
    }

    fn destructured_field(
        &mut self,
        pattern: Pattern,
        temporary: &str,
        index: usize,
        name: &str,
        qualifier: &str,
    ) -> Json {
        let definition_id = self.ids.unplaced();
        let app_id = self.ids.unplaced();
        let name_id = self.ids.unplaced();
        let accessor_id = self.ids.unplaced();
        let access = match pattern {
            Pattern::Tuple => app(
                app_id,
                "item",
                vec![
                    name_expr(name_id, temporary),
                    literal(accessor_id, "int", Json::Number((index + 1).to_string())),
                ],
            ),
            Pattern::Record => app(
                app_id,
                "field",
                vec![
                    name_expr(name_id, temporary),
                    literal(accessor_id, "str", Json::text(name)),
                ],
            ),
        };
        def(definition_id, name, qualifier, access)
    }

    fn nested<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, Reported>,
    ) -> Result<T, Reported> {
        if self.nesting >= NESTING {
            let (line, column) = {
                let token = self.token(self.at);
                (token.line, token.column)
            };
            let at = self.at;
            let refusal = self.too_deep.get_or_insert_with(|| Reported {
                problem: Problem::coded(
                    "unreadable",
                    format!(
                        "expressions or types here are nested more than {NESTING} deep, deeper than earnest reads"
                    ),
                    line - 1,
                    column,
                ),
                token: at,
                seen: at,
            });
            return Err(refusal.clone());
        }
        self.nesting += 1;
        let parsed = parse(self);
        self.nesting -= 1;
        parsed
    }

    fn expr(&mut self, precedence: u8) -> Result<Json, Reported> {
        self.nested(|parser| parser.expression_at(precedence))
    }

    fn expression_at(&mut self, precedence: u8) -> Result<Json, Reported> {
        let root = std::mem::replace(&mut self.root, false);
        let fallback = if root {
            Fallback::Allowed
        } else {
            Fallback::Not
        };
        let start = self.at;
        let mut left = self.primary()?;
        loop {
            let kind = self.kind();
            let (level, opcode, right_level) = match kind {
                Kind::Dot if DOT >= precedence => {
                    left = self.dot_call(start, left, fallback)?;
                    continue;
                }
                Kind::LeftBracket if INDEX >= precedence => {
                    if self.may_start_body(kind) && !self.goes_on(self.at) {
                        break;
                    }
                    let indexed = left.clone();
                    let index = |parser: &mut Self| {
                        parser.advance();
                        let index = parser.enclosed(&[Kind::RightBracket])?;
                        parser.expect(Kind::RightBracket, "]")?;
                        let app_id = parser.placed(parser.span(start));
                        Ok(app(app_id, "nth", vec![indexed, index]))
                    };
                    if fallback == Fallback::Not {
                        left = index(self)?;
                    } else if let Some(applied) = self.contest(index, self.at, Self::primary)? {
                        left = applied;
                    } else {
                        break;
                    }
                    continue;
                }
                Kind::Caret => (POWER, "ipow", POWER),
                Kind::Star => (PRODUCT, "imul", PRODUCT + 1),
                Kind::Slash => (PRODUCT, "idiv", PRODUCT + 1),
                Kind::Percent => (PRODUCT, "imod", PRODUCT + 1),
                Kind::Plus => (SUM, "iadd", SUM + 1),
                Kind::Minus => (SUM, "isub", SUM + 1),
                Kind::Greater => (RELATION, "igt", RELATION + 1),
                Kind::Less => (RELATION, "ilt", RELATION + 1),
                Kind::GreaterOrEqual => (RELATION, "igte", RELATION + 1),
                Kind::LessOrEqual => (RELATION, "ilte", RELATION + 1),
                Kind::Equal => (RELATION, "eq", RELATION + 1),
                Kind::Unequal => (RELATION, "neq", RELATION + 1),
                Kind::Assign => (MISTAKEN_EQUALITY, "eq", MISTAKEN_EQUALITY + 1),
                Kind::And => (CONJUNCTION, "and", CONJUNCTION + 1),
                Kind::Or => (DISJUNCTION, "or", DISJUNCTION + 1),
                Kind::Iff => (EQUIVALENCE, "iff", EQUIVALENCE + 1),
                Kind::LeadsTo => (LEADS, "leadsTo", LEADS + 1),
                Kind::Implies => (IMPLICATION, "implies", IMPLICATION + 1),
                Kind::Arrow => (PAIR, "Tup", PAIR + 1),
                _ => break,
            };
            if level < precedence || (self.may_start_body(kind) && !self.goes_on(self.at)) {
                break;
            }
            if kind == Kind::Minus && fallback == Fallback::Allowed && !self.operand_starts() {
                break;
            }
            if INFIX_NAMES.contains(&kind)
                && self.edge
                && !self.may_start_body(kind)
                && self.doubts(self.at)
            {
                break;
            }
            self.advance();
            let right = self.expr(right_level)?;
            if kind == Kind::Assign {
                self.report("QNT006", "unexpected '=', did you mean '=='?".to_owned());
            }
            let app_id = self.placed(self.span(start));
            left = app(app_id, opcode, vec![left, right]);
        }
        Ok(left)
    }

    fn application(&mut self, span: Span, name: &str, args: Vec<Json>) -> Json {
        let app_id = self.placed(span);
        if name == "q::debug" && args.len() == 1 {
            let shown_id = self.ids.unplaced();
            let shown = literal(shown_id, "str", Json::text(&self.text(span)));
            let mut args = args;
            args.insert(0, shown);
            return app(app_id, name, args);
        }
        app(app_id, name, args)
    }

    fn dot_call(
        &mut self,
        start: usize,
        callee: Json,
        fallback: Fallback,
    ) -> Result<Json, Reported> {
        self.advance();
        let name_start = self.at;
        if INFIX_NAMES.contains(&self.kind()) {
            self.advance();
        } else {
            self.qualified_name()?;
        }
        let name = self.text(self.span(name_start));
        if self.kind() == Kind::LeftParen && self.goes_on(self.at) {
            let receiver = callee.clone();
            let call = |parser: &mut Self| {
                parser.advance();
                let mut args = vec![receiver];
                if parser.kind() != Kind::RightParen {
                    args.extend(parser.arguments()?);
                }
                parser.expect(Kind::RightParen, ")")?;
                Ok(parser.application(parser.span(start), &name, args))
            };
            if fallback == Fallback::Not {
                return call(self);
            }
            if let Some(called) = self.contest(call, self.at, Self::parenthesized_here)? {
                return Ok(called);
            }
        }
        let span = self.span(start);
        let accessor_id = self.placed(span);
        let index = name
            .strip_prefix('_')
            .filter(|digits| {
                let bytes = digits.as_bytes();
                matches!(bytes.len(), 1 | 2)
                    && bytes[0].is_ascii_digit()
                    && bytes[0] != b'0'
                    && bytes.iter().all(u8::is_ascii_digit)
            })
            .map(str::to_owned);
        let (opcode, accessor) = if let Some(digits) = index {
            ("item", literal(accessor_id, "int", Json::Number(digits)))
        } else {
            ("field", literal(accessor_id, "str", Json::text(&name)))
        };
        Ok(self.application(span, opcode, vec![callee, accessor]))
    }

    fn arguments(&mut self) -> Result<Vec<Json>, Reported> {
        let mut args = vec![self.enclosed(&[Kind::Comma, Kind::RightParen])?];
        while self.kind() == Kind::Comma && !matches!(self.kind_at(self.at + 1), Kind::RightParen) {
            self.advance();
            args.push(self.enclosed(&[Kind::Comma, Kind::RightParen])?);
        }
        Ok(args)
    }

    fn listed(&mut self, close: Kind, shown: &str) -> Result<Vec<Json>, Reported> {
        let mut values = vec![self.enclosed(&[Kind::Comma, close])?];
        while self.kind() == Kind::Comma && self.kind_at(self.at + 1) != close {
            self.advance();
            values.push(self.enclosed(&[Kind::Comma, close])?);
        }
        self.eat(Kind::Comma);
        self.expect(close, shown)?;
        Ok(values)
    }

    fn lambda_params(&self, at: usize) -> Option<usize> {
        let mut at = at;
        loop {
            let after = if self.kind_at(at) == Kind::Hole {
                at + 1
            } else {
                self.after_qualified(at)
            };
            if after == at {
                return None;
            }
            at = after;
            match self.kind_at(at) {
                Kind::Comma => at += 1,
                Kind::RightParen => return Some(at),
                _ => return None,
            }
        }
    }

    fn unannotated_parameters(&mut self) -> Result<Vec<Json>, Reported> {
        let mut params = Vec::new();
        let listed = self.kind() == Kind::LeftParen
            && self.is_parameter_list(self.at + 1)
            && self.goes_on(self.at);
        if !listed {
            return Ok(params);
        }
        self.advance();
        if self.eat(Kind::RightParen) {
            return Ok(params);
        }
        loop {
            let param_start = self.at;
            let name = self.ident_or_hole()?;
            let param_id = self.placed(self.span(param_start));
            params.push(param(param_id, &self.text(name)));
            if self.kind() == Kind::Comma && self.kind_at(self.at + 1) != Kind::RightParen {
                self.advance();
            } else {
                break;
            }
        }
        self.eat(Kind::Comma);
        self.expect(Kind::RightParen, ")")?;
        Ok(params)
    }

    fn is_parameter_list(&self, start: usize) -> bool {
        if self.kind_at(start) == Kind::RightParen {
            return true;
        }
        let mut at = start;
        loop {
            let after = if self.kind_at(at) == Kind::Hole {
                at + 1
            } else {
                self.after_qualified(at)
            };
            if after == at {
                return false;
            }
            at = after;
            match (self.kind_at(at), self.kind_at(at + 1)) {
                (Kind::RightParen, _) | (Kind::Comma, Kind::RightParen) => return true,
                (Kind::Comma, _) => at += 1,
                _ => return false,
            }
        }
    }

    fn parameter(&mut self) -> Result<Json, Reported> {
        let start = self.at;
        let name = self.ident_or_hole()?;
        let param_id = self.placed(self.span(start));
        Ok(param(param_id, &self.text(name)))
    }

    fn parameters(&mut self) -> Result<Vec<Json>, Reported> {
        let mut params = vec![self.parameter()?];
        while self.eat(Kind::Comma) {
            params.push(self.parameter()?);
        }
        Ok(params)
    }

    fn primary(&mut self) -> Result<Json, Reported> {
        let start = self.at;
        match self.kind() {
            Kind::Minus => {
                self.advance();
                let operand = self.expr(NEGATION)?;
                Ok(self.application(self.span(start), "iuminus", vec![operand]))
            }
            Kind::If => {
                self.advance();
                self.expect(Kind::LeftParen, "(")?;
                let condition = self.enclosed(&[Kind::RightParen])?;
                self.expect(Kind::RightParen, ")")?;
                let then = self.enclosed(&[Kind::Else])?;
                self.expect(Kind::Else, "else")?;
                let otherwise = self.expr(ELSE_BRANCH)?;
                Ok(self.application(self.span(start), "ite", vec![condition, then, otherwise]))
            }
            kind if QUALIFIERS.contains(&kind) => self.let_expression(start),
            Kind::All | Kind::Any if self.kind_at(self.at + 1) == Kind::LeftBrace => {
                let opcode = if self.kind() == Kind::All {
                    "actionAll"
                } else {
                    "actionAny"
                };
                self.advance();
                self.advance();
                let args = self.listed(Kind::RightBrace, "}")?;
                Ok(self.application(self.span(start), opcode, args))
            }
            Kind::And | Kind::Or
                if self.kind_at(self.at + 1) == Kind::LeftBrace && self.reads_keyword(self.at) =>
            {
                self.keyword_list(start)
            }
            kind if CALL_NAMES.contains(&kind)
                && self.kind_at(self.at + 1) == Kind::LeftParen
                && self.reads_keyword(self.at) =>
            {
                let called = self.contest(
                    |parser| {
                        parser.advance();
                        parser.call(start)
                    },
                    self.at + 1,
                    Self::parenthesized_here,
                )?;
                match called {
                    Some(called) => Ok(called),
                    None => self.plain_name(start),
                }
            }
            Kind::Match if self.kind_at(self.at + 1) == Kind::LeftParen && self.doubts(self.at) => {
                self.named(start)
            }
            Kind::Match if self.reads_keyword(self.at) => {
                match self.attempt(|parser| parser.match_expression(start)) {
                    Some(matched) => Ok(matched),
                    None => self.plain_name(start),
                }
            }
            Kind::LeftBracket => {
                self.advance();
                let args = if self.kind() == Kind::RightBracket {
                    self.advance();
                    Vec::new()
                } else if self.kind() == Kind::Comma
                    && self.kind_at(self.at + 1) == Kind::RightBracket
                {
                    self.advance();
                    self.advance();
                    Vec::new()
                } else {
                    self.listed(Kind::RightBracket, "]")?
                };
                Ok(self.application(self.span(start), "List", args))
            }
            Kind::LeftBrace => self.braces(start),
            Kind::LeftParen => self.parenthesized(start),
            Kind::Hole if self.kind_at(self.at + 1) == Kind::FatArrow => {
                let params = vec![self.parameter()?];
                self.lambda_rest(start, params)
            }
            Kind::Number | Kind::Truth | Kind::Text => Ok(self.literal_value(start)),
            _ if self.is_identifier(self.at) => self.named(start),
            _ => self.unexpected("an expression"),
        }
    }

    fn literal_value(&mut self, start: usize) -> Json {
        let kind = self.kind();
        self.advance();
        let literal_id = self.placed(self.span(start));
        let written = &self.token(start).text;
        match kind {
            Kind::Number => literal(
                literal_id,
                "int",
                Json::Number(crate::number::decimal(written)),
            ),
            Kind::Truth => literal(literal_id, "bool", Json::Bool(written == "true")),
            _ => {
                let chars: Vec<char> = written.chars().collect();
                let value: String = chars[1..chars.len() - 1].iter().collect();
                literal(literal_id, "str", Json::Text(value))
            }
        }
    }

    fn keyword_list(&mut self, start: usize) -> Result<Json, Reported> {
        let opcode = if self.kind() == Kind::And {
            "and"
        } else {
            "or"
        };
        let listed = self.contest(
            |parser| {
                parser.advance();
                parser.advance();
                let args = parser.listed(Kind::RightBrace, "}")?;
                Ok(parser.application(parser.span(start), opcode, args))
            },
            self.at + 1,
            Self::braces_here,
        )?;
        match listed {
            Some(listed) => Ok(listed),
            None => self.plain_name(start),
        }
    }

    fn named(&mut self, start: usize) -> Result<Json, Reported> {
        if self.edge && self.kind() != Kind::Match && self.stops.contains(&(self.at * 2 + 1)) {
            return self.plain_name(start);
        }
        let after = self.after_qualified(self.at);
        match self.kind_at(after) {
            Kind::FatArrow => {
                let params = vec![self.parameter()?];
                self.lambda_rest(start, params)
            }
            Kind::LeftParen if self.goes_on(after) => {
                let called = self.contest(
                    |parser| {
                        parser.qualified_name()?;
                        parser.call(start)
                    },
                    after,
                    Self::parenthesized_here,
                )?;
                match called {
                    Some(called) => Ok(called),
                    None => self.plain_name(start),
                }
            }
            Kind::Prime => {
                let name = self.qualified_name()?;
                self.advance();
                self.expect(Kind::Assign, "=")?;
                let value = self.expr(ASSIGNMENT)?;
                let span = self.span(start);
                let name_id = self.placed(span);
                let target = name_expr(name_id, &self.text(name));
                Ok(self.application(span, "assign", vec![target, value]))
            }
            _ => {
                let name = self.qualified_name()?;
                let name_id = self.placed(self.span(start));
                Ok(name_expr(name_id, &self.text(name)))
            }
        }
    }

    fn call(&mut self, start: usize) -> Result<Json, Reported> {
        let name = self.text(self.span(start));
        self.expect(Kind::LeftParen, "(")?;
        let mut args = Vec::new();
        if !matches!(self.kind(), Kind::RightParen | Kind::Comma) {
            args = self.arguments()?;
        }
        self.eat(Kind::Comma);
        self.expect(Kind::RightParen, ")")?;
        Ok(self.application(self.span(start), &name, args))
    }

    fn lambda_rest(&mut self, start: usize, params: Vec<Json>) -> Result<Json, Reported> {
        self.expect(Kind::FatArrow, "=>")?;
        let body = self.expr(0)?;
        let lambda_id = self.placed(self.span(start));
        Ok(lambda(lambda_id, params, "def", body))
    }

    fn parenthesized(&mut self, start: usize) -> Result<Json, Reported> {
        if let Some(close) = self.lambda_params(self.at + 1)
            && self.kind_at(close + 1) == Kind::FatArrow
        {
            self.advance();
            let params = self.parameters()?;
            self.expect(Kind::RightParen, ")")?;
            return self.lambda_rest(start, params);
        }
        if self.kind_at(self.at + 1) == Kind::LeftParen
            && let Some(close) = self.lambda_params(self.at + 2)
            && (self.at + 2..close).any(|at| self.kind_at(at) == Kind::Comma)
            && self.kind_at(close + 1) == Kind::RightParen
            && self.kind_at(close + 2) == Kind::FatArrow
        {
            self.advance();
            self.advance();
            let params = self.parameters()?;
            self.expect(Kind::RightParen, ")")?;
            self.expect(Kind::RightParen, ")")?;
            self.expect(Kind::FatArrow, "=>")?;
            let body = self.expr(0)?;
            return Ok(self.tupled_lambda(self.span(start), params, body));
        }
        let reach = self.lambda_reach(start);
        match self.tuple_or_paren(start) {
            Err(failure) if reach > failure.token => {
                self.at = reach;
                Err(self.failure("no viable alternative".to_owned()))
            }
            parsed => parsed,
        }
    }

    fn tuple_or_paren(&mut self, start: usize) -> Result<Json, Reported> {
        self.advance();
        if self.eat(Kind::RightParen) {
            let unit_id = self.placed(self.span(start));
            return Ok(unit(unit_id));
        }
        let first = self.enclosed(&[Kind::RightParen, Kind::Comma])?;
        if self.eat(Kind::RightParen) {
            return Ok(first);
        }
        self.expect(Kind::Comma, ",")?;
        let mut items = vec![first];
        items.extend(self.listed(Kind::RightParen, ")")?);
        Ok(self.application(self.span(start), "Tup", items))
    }

    fn parameters_reach(&self, start: usize, at_least: usize) -> Result<usize, usize> {
        let mut at = start;
        let mut count = 0;
        loop {
            let after = if self.kind_at(at) == Kind::Hole {
                at + 1
            } else {
                self.after_qualified(at)
            };
            if after == at {
                return Err(at);
            }
            count += 1;
            at = after;
            match self.kind_at(at) {
                Kind::Comma => at += 1,
                Kind::RightParen if count >= at_least => return Ok(at),
                _ => return Err(at),
            }
        }
    }

    fn lambda_reach(&self, open: usize) -> usize {
        let plain = |parser: &Self, open: usize| match parser.parameters_reach(open + 1, 1) {
            Ok(close) if parser.kind_at(close + 1) == Kind::FatArrow => usize::MAX,
            Ok(close) => close + 1,
            Err(at) => at,
        };
        let mut reach = plain(self, open);
        if self.kind_at(open + 1) == Kind::LeftParen {
            reach = reach.max(plain(self, open + 1));
            let sugared = match self.parameters_reach(open + 2, 2) {
                Ok(close) if self.kind_at(close + 1) == Kind::RightParen => {
                    if self.kind_at(close + 2) == Kind::FatArrow {
                        usize::MAX
                    } else {
                        close + 2
                    }
                }
                Ok(close) => close + 1,
                Err(at) => at,
            };
            reach = reach.max(sugared);
        }
        if reach == usize::MAX { 0 } else { reach }
    }

    fn tupled_lambda(&mut self, span: Span, params: Vec<Json>, body: Json) -> Json {
        let fresh_id = self.placed(span);
        let fresh_name = format!("quintTupledLambdaParam{fresh_id}");
        let mut body = body;
        for (index, written) in params.into_iter().enumerate() {
            let let_id = self.placed(span);
            let app_id = self.placed(span);
            let name_id = self.placed(span);
            let index_id = self.placed(span);
            let param_id = written.get("id").cloned().unwrap_or(Json::Null);
            let param_name = written.get("name").cloned().unwrap_or(Json::Null);
            let access = app(
                app_id,
                "item",
                vec![
                    name_expr(name_id, &fresh_name),
                    literal(index_id, "int", Json::Number((index + 1).to_string())),
                ],
            );
            let definition = Json::object(vec![
                ("id", param_id),
                ("kind", Json::text("def")),
                ("name", param_name),
                ("qualifier", Json::text("pureval")),
                ("expr", access),
            ]);
            body = let_in(let_id, definition, body);
        }
        let lambda_id = self.placed(span);
        lambda(lambda_id, vec![param(fresh_id, &fresh_name)], "def", body)
    }

    fn braces(&mut self, start: usize) -> Result<Json, Reported> {
        let next = self.at + 1;
        let is_record = self.kind_at(next) == Kind::Spread
            || (self.is_identifier(next)
                && self.kind_at(self.after_qualified(next)) == Kind::Colon);
        self.advance();
        if !is_record {
            let inner = self.enclosed(&[Kind::RightBrace])?;
            self.expect(Kind::RightBrace, "}")?;
            return Ok(inner);
        }
        let mut pairs: Vec<(Json, Json)> = Vec::new();
        let mut spreads = Vec::new();
        loop {
            let element_start = self.at;
            if self.eat(Kind::Spread) {
                spreads.push(self.enclosed(&[Kind::Comma, Kind::RightBrace])?);
            } else {
                let field = self.simple_name("record")?;
                self.expect(Kind::Colon, ":")?;
                let value = self.enclosed(&[Kind::Comma, Kind::RightBrace])?;
                let field_id = self.placed(self.span(element_start));
                pairs.push((
                    literal(field_id, "str", Json::text(&self.text(field))),
                    value,
                ));
            }
            if self.kind() == Kind::Comma && self.kind_at(self.at + 1) != Kind::RightBrace {
                self.advance();
            } else {
                break;
            }
        }
        self.eat(Kind::Comma);
        self.expect(Kind::RightBrace, "}")?;
        let span = self.span(start);
        let mut seen: Vec<String> = Vec::new();
        let mut duplicated: Vec<String> = Vec::new();
        for (field, _) in &pairs {
            if let Some(Json::Text(name)) = field.get("value") {
                if seen.contains(name) {
                    if !duplicated.contains(name) {
                        duplicated.push(name.clone());
                    }
                } else {
                    seen.push(name.clone());
                }
            }
        }
        if !duplicated.is_empty() {
            let error_id = self.placed(span);
            let plural = if duplicated.len() > 1 {
                "Fields"
            } else {
                "Field"
            };
            self.listen(
                "QNT016",
                format!(
                    "{plural} {} cannot be defined more than once",
                    duplicated.join(", ")
                ),
                error_id,
            );
        }
        if spreads.is_empty() {
            let args = pairs
                .into_iter()
                .flat_map(|(field, value)| [field, value])
                .collect();
            return Ok(self.application(span, "Rec", args));
        }
        if spreads.len() > 1 {
            let error_id = self.placed(span);
            self.listen(
                "QNT012",
                "... may be used once in { ...record, <fields> }".to_owned(),
                error_id,
            );
            return Ok(Json::Null);
        }
        let mut record = spreads.remove(0);
        for (field, value) in pairs {
            let with_id = self.placed(span);
            record = app(with_id, "with", vec![record, field, value]);
        }
        Ok(record)
    }

    fn match_expression(&mut self, start: usize) -> Result<Json, Reported> {
        self.advance();
        let scrutinee = self.enclosed(&[Kind::LeftBrace])?;
        self.expect(Kind::LeftBrace, "{")?;
        self.eat(Kind::Pipe);
        let mut cases = Vec::new();
        loop {
            let case_start = self.at;
            let (label, binding) = if self.eat(Kind::Hole) {
                ("_".to_owned(), "_".to_owned())
            } else {
                let label = self.simple_name("variant label")?;
                let mut binding = "_".to_owned();
                if self.eat(Kind::LeftParen) {
                    if !self.eat(Kind::Hole) {
                        let name = self.simple_name("match case parameter")?;
                        binding = self.text(name);
                    }
                    self.expect(Kind::RightParen, ")")?;
                }
                (self.text(label), binding)
            };
            self.expect(Kind::FatArrow, "=>")?;
            let body = self.enclosed(&[Kind::Pipe, Kind::RightBrace])?;
            cases.push((self.span(case_start), label, binding, body));
            if !self.eat(Kind::Pipe) {
                break;
            }
        }
        self.expect(Kind::RightBrace, "}")?;
        let match_id = self.placed(self.span(start));
        let mut args = vec![scrutinee];
        for (span, label, binding, body) in cases {
            let label_id = self.placed(span);
            let eliminator_id = self.placed(span);
            let param_id = self.placed(span);
            args.push(literal(label_id, "str", Json::Text(label)));
            args.push(Json::object(vec![
                ("id", id(eliminator_id)),
                ("kind", Json::text("lambda")),
                ("qualifier", Json::text("def")),
                ("expr", body),
                (
                    "params",
                    Json::List(vec![Json::object(vec![
                        ("name", Json::text(&binding)),
                        ("id", id(param_id)),
                    ])]),
                ),
            ]));
        }
        Ok(app(match_id, "matchVariant", args))
    }

    fn let_parts(
        &mut self,
        since: usize,
    ) -> Result<(Definition, Json), (Box<Reported>, Vec<usize>)> {
        let outer_edge = std::mem::replace(&mut self.edge, true);
        let outer = std::mem::take(&mut self.candidates);
        let definition = self.operator_definition(Within::Let);
        let mut mine = std::mem::replace(&mut self.candidates, outer);
        self.edge = outer_edge;
        let definition = match definition {
            Ok(definition) => definition,
            Err(failure) => return Err((Box::new(failure), mine)),
        };
        match self.expr(LET_BODY) {
            Ok(body) => {
                self.candidates.extend(mine);
                Ok((definition, body))
            }
            Err(failure) => {
                mine.extend(self.candidates.get(since..).unwrap_or_default());
                Err((Box::new(failure), mine))
            }
        }
    }

    fn choose(
        &mut self,
        mark: &Mark,
        candidates: Vec<usize>,
        unfollowed: Unfollowed,
    ) -> Option<(Definition, Json)> {
        let mut found = Found::default();
        self.search(mark, &[], candidates, unfollowed, &mut found);
        self.rewind(mark);
        let chosen = match (found.following, unfollowed) {
            (Some(stops), _) => stops,
            (None, Unfollowed::TakenBeyond(_)) => found.parsing?,
            (None, Unfollowed::Refused) => return None,
        };
        self.stops.extend(chosen);
        self.let_parts(mark.candidates).ok()
    }

    fn search(
        &mut self,
        mark: &Mark,
        earlier: &[usize],
        mut candidates: Vec<usize>,
        unfollowed: Unfollowed,
        found: &mut Found,
    ) {
        candidates.sort_unstable();
        candidates.dedup();
        for candidate in candidates.into_iter().rev() {
            if found.following.is_some() || !self.searching() {
                return;
            }
            let mut stops = earlier.to_vec();
            stops.push(candidate);
            self.rewind(mark);
            self.stops.extend(stops.iter().copied());
            let reached = self.reached.replace(0);
            let tried = self.let_parts(mark.candidates);
            let reach = self.reached.get().max(self.at);
            self.reached.set(reached.max(self.reached.get()));
            let pushed = match tried {
                Err((_, pushed)) => pushed,
                Ok(_) if self.can_follow() => {
                    found.following = Some(stops);
                    return;
                }
                Ok(_) => {
                    if matches!(unfollowed, Unfollowed::TakenBeyond(failed) if reach >= failed)
                        && earlier.is_empty()
                        && found.parsing.is_none()
                    {
                        found.parsing = Some(stops.clone());
                    }
                    self.candidates
                        .get(mark.candidates..)
                        .unwrap_or_default()
                        .to_vec()
                }
            };
            let at = candidate / 2;
            let called = self.kind_at(at + 1) == Kind::LeftParen;
            let later: Vec<usize> = pushed
                .into_iter()
                .filter(|later| later / 2 > at || (called && *later > candidate))
                .collect();
            self.search(mark, &stops, later, unfollowed, found);
        }
    }

    fn let_expression(&mut self, start: usize) -> Result<Json, Reported> {
        let mark = self.mark();
        let (definition, body) = match self.let_parts(mark.candidates) {
            Ok(parts) if self.can_follow() => parts,
            Ok(_) => {
                let candidates = self
                    .candidates
                    .get(mark.candidates..)
                    .unwrap_or_default()
                    .to_vec();
                if let Some(parts) = self.choose(&mark, candidates, Unfollowed::Refused) {
                    parts
                } else {
                    self.rewind(&mark);
                    self.let_parts(mark.candidates)
                        .map_err(|(failure, _)| *failure)?
                }
            }
            Err((failure, candidates)) => {
                let Some(parts) = self.choose(
                    &mark,
                    candidates.clone(),
                    Unfollowed::TakenBeyond(failure.token),
                ) else {
                    self.rewind(&mark);
                    self.candidates.extend(candidates);
                    return Err(*failure);
                };
                if mark.reported.is_none()
                    && self.at < failure.token
                    && !self.can_follow()
                    && self.reported.as_ref().is_some_and(|reported| {
                        reported.problem.code == "QNT006" && reported.token == self.at
                    })
                {
                    self.reported = Some(*failure);
                }
                parts
            }
        };
        let span = self.span(start);
        match definition {
            Definition::Plain(definition) => {
                let let_id = self.placed(span);
                Ok(let_in(let_id, definition, body))
            }
            Definition::Destructured(temporary, destructured) => {
                let temporary_name = match temporary.get("name") {
                    Some(Json::Text(name)) => name.clone(),
                    _ => String::new(),
                };
                let qualifier = match temporary.get("qualifier") {
                    Some(Json::Text(qualifier)) => qualifier.clone(),
                    _ => String::new(),
                };
                let mut wrapped = body;
                for (index, name) in destructured.names.iter().enumerate() {
                    if name != "_" {
                        let field = self.destructured_field(
                            destructured.pattern,
                            &temporary_name,
                            index,
                            name,
                            &qualifier,
                        );
                        let let_id = self.placed(span);
                        wrapped = let_in(let_id, field, wrapped);
                    }
                }
                let let_id = self.placed(span);
                Ok(let_in(let_id, temporary, wrapped))
            }
        }
    }

    fn type_expr(&mut self, precedence: u8) -> Result<Json, Reported> {
        self.nested(|parser| parser.type_at(precedence))
    }

    fn type_at(&mut self, precedence: u8) -> Result<Json, Reported> {
        let start = self.at;
        let mut left = self.type_primary()?;
        loop {
            match self.kind() {
                Kind::Arrow if FUNCTION_TYPE >= precedence => {
                    self.advance();
                    let result = self.type_expr(FUNCTION_TYPE)?;
                    let type_id = self.placed(self.span(start));
                    left = type_node(type_id, "fun", vec![("arg", left), ("res", result)]);
                }
                Kind::FatArrow if OPERATOR_TYPE >= precedence => {
                    self.advance();
                    let result = self.type_expr(OPERATOR_TYPE)?;
                    let type_id = self.placed(self.span(start));
                    left = type_node(
                        type_id,
                        "oper",
                        vec![("args", Json::List(vec![left])), ("res", result)],
                    );
                }
                _ => return Ok(left),
            }
        }
    }

    fn matching_paren(&self, open: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut at = open;
        loop {
            match self.kind_at(at) {
                Kind::LeftParen => depth += 1,
                Kind::RightParen => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(at);
                    }
                }
                Kind::End => return None,
                _ => {}
            }
            at += 1;
        }
    }

    fn type_primary(&mut self) -> Result<Json, Reported> {
        let start = self.at;
        match self.kind() {
            Kind::LeftParen => self.parenthesized_type(start),
            Kind::Set | Kind::List
                if self.kind_at(self.at + 1) == Kind::LeftBracket
                    && self.closing(self.at + 1).is_some_and(|(_, comma)| !comma) =>
            {
                let kind = if self.kind() == Kind::Set {
                    "set"
                } else {
                    "list"
                };
                self.advance();
                self.advance();
                let element = self.type_expr(0)?;
                self.expect(Kind::RightBracket, "]")?;
                let type_id = self.placed(self.span(start));
                Ok(type_node(type_id, kind, vec![("elem", element)]))
            }
            Kind::LeftBrace => {
                self.advance();
                let fields = if self.kind() == Kind::RightBrace {
                    empty_row()
                } else {
                    self.row()?
                };
                self.expect(Kind::RightBrace, "}")?;
                let type_id = self.placed(self.span(start));
                Ok(type_node(type_id, "rec", vec![("fields", fields)]))
            }
            Kind::IntType | Kind::StrType | Kind::BoolType => {
                let kind = match self.kind() {
                    Kind::IntType => "int",
                    Kind::StrType => "str",
                    _ => "bool",
                };
                self.advance();
                let type_id = self.placed(self.span(start));
                Ok(type_node(type_id, kind, Vec::new()))
            }
            _ if self.is_identifier(self.at) => {
                let after = self.after_qualified(self.at);
                match self.kind_at(after) {
                    Kind::LeftBracket | Kind::LeftParen => self.type_application(start, after),
                    _ => {
                        let single = after == self.at + 1 && self.kind() == Kind::LowerName;
                        let name = self.qualified_name()?;
                        let type_id = self.placed(self.span(start));
                        let kind = if single { "var" } else { "const" };
                        Ok(type_node(
                            type_id,
                            kind,
                            vec![("name", Json::text(&self.text(name)))],
                        ))
                    }
                }
            }
            _ => self.unexpected("a type"),
        }
    }

    fn parenthesized_type(&mut self, start: usize) -> Result<Json, Reported> {
        if self
            .matching_paren(self.at)
            .is_some_and(|close| self.kind_at(close + 1) == Kind::FatArrow)
        {
            self.advance();
            let mut args = Vec::new();
            if !matches!(self.kind(), Kind::RightParen | Kind::Comma) {
                args.push(self.type_expr(0)?);
                while self.kind() == Kind::Comma && self.kind_at(self.at + 1) != Kind::RightParen {
                    self.advance();
                    args.push(self.type_expr(0)?);
                }
            }
            self.eat(Kind::Comma);
            self.expect(Kind::RightParen, ")")?;
            self.expect(Kind::FatArrow, "=>")?;
            let result = self.type_expr(OPERATOR_RESULT)?;
            let type_id = self.placed(self.span(start));
            return Ok(type_node(
                type_id,
                "oper",
                vec![("args", Json::List(args)), ("res", result)],
            ));
        }
        self.advance();
        if self.eat(Kind::RightParen) {
            let type_id = self.placed(self.span(start));
            return Ok(type_node(type_id, "tup", vec![("fields", empty_row())]));
        }
        let first = self.type_expr(0)?;
        if self.eat(Kind::RightParen) {
            return Ok(first);
        }
        let mut items = vec![first];
        self.expect(Kind::Comma, ",")?;
        items.push(self.type_expr(0)?);
        while self.kind() == Kind::Comma && self.kind_at(self.at + 1) != Kind::RightParen {
            self.advance();
            items.push(self.type_expr(0)?);
        }
        self.eat(Kind::Comma);
        self.expect(Kind::RightParen, ")")?;
        let type_id = self.placed(self.span(start));
        let fields = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                Json::object(vec![
                    ("fieldName", Json::Text(index.to_string())),
                    ("fieldType", item),
                ])
            })
            .collect();
        Ok(type_node(
            type_id,
            "tup",
            vec![("fields", row(fields, empty_row()))],
        ))
    }

    fn type_application(&mut self, start: usize, after: usize) -> Result<Json, Reported> {
        let wrong = self.kind_at(after) == Kind::LeftParen;
        let constructor = self.qualified_name()?;
        self.advance();
        let mut args = vec![self.type_expr(0)?];
        while self.eat(Kind::Comma) {
            args.push(self.type_expr(0)?);
        }
        let name = self.text(constructor);
        if wrong {
            self.expect(Kind::RightParen, ")")?;
            let written = self.text((constructor.1 + 2, self.at.saturating_sub(2)));
            self.report(
                "QNT009",
                format!(
                    "Use square brackets instead of parenthesis for type application: {name}[{}]",
                    written.replace(',', ", ")
                ),
            );
        } else {
            self.expect(Kind::RightBracket, "]")?;
        }
        let span = self.span(start);
        let type_id = self.placed(span);
        let constructor_id = self.placed(span);
        if name == "Map" && args.len() == 2 {
            let key = type_text(&args[0]);
            let value = type_text(&args[1]);
            self.listen(
                "QNT015",
                format!("Use '{key} -> {value}' instead of 'Map[{key}, {value}]' for map types"),
                type_id,
            );
            let mut args = args.into_iter();
            let argument = args.next().unwrap_or(Json::Null);
            let result = args.next().unwrap_or(Json::Null);
            return Ok(type_node(
                type_id,
                "fun",
                vec![("arg", argument), ("res", result)],
            ));
        }
        Ok(type_node(
            type_id,
            "app",
            vec![
                (
                    "ctor",
                    type_node(constructor_id, "const", vec![("name", Json::Text(name))]),
                ),
                ("args", Json::List(args)),
            ],
        ))
    }

    fn row(&mut self) -> Result<Json, Reported> {
        let mut fields = Vec::new();
        if self.kind() != Kind::Pipe {
            loop {
                let label = self.simple_name("record")?;
                self.expect(Kind::Colon, ":")?;
                let field_type = self.type_expr(0)?;
                fields.push(Json::object(vec![
                    ("fieldName", Json::text(&self.text(label))),
                    ("fieldType", field_type),
                ]));
                if self.kind() == Kind::Comma && self.is_identifier(self.at + 1) {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.eat(Kind::Comma) {
                return Ok(row(fields, empty_row()));
            }
        }
        let other = if self.eat(Kind::Pipe) {
            let at = self.at;
            self.identifier()?;
            Json::object(vec![
                ("kind", Json::text("var")),
                ("name", Json::text(&self.text((at, at)))),
            ])
        } else {
            empty_row()
        };
        Ok(row(fields, other))
    }
}
