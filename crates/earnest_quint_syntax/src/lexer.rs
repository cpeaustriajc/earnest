use crate::problem::Problem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Module,
    LeftBrace,
    RightBrace,
    Const,
    Colon,
    Var,
    Assume,
    Comma,
    Semicolon,
    Val,
    Pure,
    Type,
    LeftBracket,
    RightBracket,
    Pipe,
    Def,
    Action,
    Run,
    Temporal,
    Nondet,
    Dot,
    Arrow,
    FatArrow,
    IntType,
    StrType,
    BoolType,
    Caret,
    Prime,
    All,
    Any,
    If,
    Else,
    Hole,
    Spread,
    Path,
    Text,
    Truth,
    Number,
    And,
    Or,
    Iff,
    Implies,
    LeadsTo,
    Match,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Greater,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    Unequal,
    Equal,
    Assign,
    LeftParen,
    RightParen,
    Set,
    List,
    Import,
    Export,
    From,
    As,
    LowerName,
    UpperName,
    Hashbang,
    DocComment,
    End,
}

const FIXED: [(&str, Kind); 35] = [
    ("module", Kind::Module),
    ("{", Kind::LeftBrace),
    ("}", Kind::RightBrace),
    ("const", Kind::Const),
    (":", Kind::Colon),
    ("var", Kind::Var),
    ("assume", Kind::Assume),
    (",", Kind::Comma),
    (";", Kind::Semicolon),
    ("val", Kind::Val),
    ("pure", Kind::Pure),
    ("type", Kind::Type),
    ("[", Kind::LeftBracket),
    ("]", Kind::RightBracket),
    ("|", Kind::Pipe),
    ("def", Kind::Def),
    ("action", Kind::Action),
    ("run", Kind::Run),
    ("temporal", Kind::Temporal),
    ("nondet", Kind::Nondet),
    (".", Kind::Dot),
    ("->", Kind::Arrow),
    ("=>", Kind::FatArrow),
    ("int", Kind::IntType),
    ("str", Kind::StrType),
    ("bool", Kind::BoolType),
    ("^", Kind::Caret),
    ("'", Kind::Prime),
    ("all", Kind::All),
    ("any", Kind::Any),
    ("if", Kind::If),
    ("else", Kind::Else),
    ("_", Kind::Hole),
    ("...", Kind::Spread),
    ("::", Kind::Path),
];

const WORDS: [(&str, Kind); 26] = [
    ("and", Kind::And),
    ("or", Kind::Or),
    ("iff", Kind::Iff),
    ("implies", Kind::Implies),
    ("leadsTo", Kind::LeadsTo),
    ("match", Kind::Match),
    ("+", Kind::Plus),
    ("-", Kind::Minus),
    ("*", Kind::Star),
    ("/", Kind::Slash),
    ("%", Kind::Percent),
    (">", Kind::Greater),
    ("<", Kind::Less),
    (">=", Kind::GreaterOrEqual),
    ("<=", Kind::LessOrEqual),
    ("!=", Kind::Unequal),
    ("==", Kind::Equal),
    ("=", Kind::Assign),
    ("(", Kind::LeftParen),
    (")", Kind::RightParen),
    ("Set", Kind::Set),
    ("List", Kind::List),
    ("import", Kind::Import),
    ("export", Kind::Export),
    ("from", Kind::From),
    ("as", Kind::As),
];

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub(crate) kind: Kind,
    pub(crate) text: String,
    pub(crate) start: usize,
    pub(crate) stop: isize,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Copy)]
enum Rule {
    Fixed(usize),
    Text,
    Truth,
    Number,
    Word(usize),
    LowerName,
    UpperName,
    Hashbang,
    DocComment,
    LineComment,
    Comment,
    Space,
}

struct Attempt {
    accepted: Option<usize>,
    reached: usize,
}

impl Attempt {
    fn accepted(length: usize) -> Self {
        Self {
            accepted: Some(length),
            reached: length,
        }
    }

    fn refused(reached: usize) -> Self {
        Self {
            accepted: None,
            reached,
        }
    }
}

pub(crate) struct Lexed {
    pub(crate) tokens: Vec<Token>,
    pub(crate) problems: Vec<(usize, Problem)>,
}

pub(crate) fn lex(text: &[char]) -> Lexed {
    let mut tokens = Vec::new();
    let mut problems = Vec::new();
    let mut at = 0;
    let mut line = 1;
    let mut column = 0;
    while at < text.len() {
        let rest = &text[at..];
        let Some((rule, length)) = longest(rest) else {
            let reached = farthest(rest);
            let shown_end = (reached + 1).min(rest.len());
            let shown: String = rest[..shown_end].iter().collect();
            problems.push((
                at,
                Problem::syntax(
                    format!("token recognition error at: '{}'", displayed(&shown)),
                    line - 1,
                    column,
                ),
            ));
            advance(rest, shown_end, &mut line, &mut column);
            at += shown_end;
            continue;
        };
        if let Some(kind) = kind_of(rule) {
            tokens.push(Token {
                kind,
                text: rest[..length].iter().collect(),
                start: at,
                stop: signed(at + length) - 1,
                line,
                column,
            });
        }
        advance(rest, length, &mut line, &mut column);
        at += length;
    }
    tokens.push(Token {
        kind: Kind::End,
        text: "<EOF>".to_owned(),
        start: text.len(),
        stop: signed(text.len()) - 1,
        line,
        column,
    });
    Lexed { tokens, problems }
}

pub(crate) fn signed(value: usize) -> isize {
    isize::try_from(value).unwrap_or(isize::MAX)
}

fn advance(chars: &[char], length: usize, line: &mut usize, column: &mut usize) {
    for &char in &chars[..length] {
        if char == '\n' {
            *line += 1;
            *column = 0;
        } else {
            *column += 1;
        }
    }
}

fn displayed(text: &str) -> String {
    text.replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn kind_of(rule: Rule) -> Option<Kind> {
    match rule {
        Rule::Fixed(index) => Some(FIXED[index].1),
        Rule::Word(index) => Some(WORDS[index].1),
        Rule::Text => Some(Kind::Text),
        Rule::Truth => Some(Kind::Truth),
        Rule::Number => Some(Kind::Number),
        Rule::LowerName => Some(Kind::LowerName),
        Rule::UpperName => Some(Kind::UpperName),
        Rule::Hashbang => Some(Kind::Hashbang),
        Rule::DocComment => Some(Kind::DocComment),
        Rule::LineComment | Rule::Comment | Rule::Space => None,
    }
}

fn rules() -> impl Iterator<Item = Rule> {
    (0..FIXED.len())
        .map(Rule::Fixed)
        .chain([Rule::Text, Rule::Truth, Rule::Number])
        .chain((0..WORDS.len()).map(Rule::Word))
        .chain([
            Rule::LowerName,
            Rule::UpperName,
            Rule::Hashbang,
            Rule::DocComment,
            Rule::LineComment,
            Rule::Comment,
            Rule::Space,
        ])
}

fn longest(rest: &[char]) -> Option<(Rule, usize)> {
    let mut best: Option<(Rule, usize)> = None;
    for rule in rules() {
        if let Some(length) = attempt(rule, rest).accepted
            && length > 0
            && best.is_none_or(|(_, longest)| length > longest)
        {
            best = Some((rule, length));
        }
    }
    best
}

fn farthest(rest: &[char]) -> usize {
    rules()
        .map(|rule| attempt(rule, rest).reached)
        .max()
        .unwrap_or(0)
}

fn attempt(rule: Rule, rest: &[char]) -> Attempt {
    match rule {
        Rule::Fixed(index) => literal(FIXED[index].0, rest),
        Rule::Word(index) => literal(WORDS[index].0, rest),
        Rule::Text => delimited(rest, &['"'], &['"']),
        Rule::Truth => {
            let true_word = literal("true", rest);
            let false_word = literal("false", rest);
            if true_word.accepted.is_some() {
                true_word
            } else if false_word.accepted.is_some() {
                false_word
            } else {
                Attempt::refused(true_word.reached.max(false_word.reached))
            }
        }
        Rule::Number => number(rest),
        Rule::LowerName => name(rest, char::is_ascii_lowercase),
        Rule::UpperName => name(rest, char::is_ascii_uppercase),
        Rule::Hashbang => delimited(rest, &['#', '!'], &['\n']),
        Rule::DocComment => delimited(rest, &['/', '/', '/'], &['\n']),
        Rule::LineComment => delimited(rest, &['/', '/'], &['\n']),
        Rule::Comment => delimited(rest, &['/', '*'], &['*', '/']),
        Rule::Space => {
            let length = rest
                .iter()
                .take_while(|char| matches!(char, ' ' | '\t' | '\r' | '\n'))
                .count();
            if length > 0 {
                Attempt::accepted(length)
            } else {
                Attempt::refused(0)
            }
        }
    }
}

fn literal(word: &str, rest: &[char]) -> Attempt {
    let mut matched = 0;
    for expected in word.chars() {
        if rest.get(matched) == Some(&expected) {
            matched += 1;
        } else {
            return Attempt::refused(matched);
        }
    }
    Attempt::accepted(matched)
}

fn delimited(rest: &[char], open: &[char], close: &[char]) -> Attempt {
    let opened = rest.iter().zip(open).take_while(|(a, b)| a == b).count();
    if opened < open.len() {
        return Attempt::refused(opened);
    }
    let mut at = open.len();
    while at < rest.len() {
        if rest[at..].starts_with(close) {
            return Attempt::accepted(at + close.len());
        }
        at += 1;
    }
    Attempt::refused(rest.len())
}

fn number(rest: &[char]) -> Attempt {
    match rest.first() {
        Some('0') => {
            if rest.get(1) == Some(&'x') && rest.get(2).is_some_and(char::is_ascii_hexdigit) {
                Attempt::accepted(3 + separated(&rest[3..], char::is_ascii_hexdigit))
            } else {
                Attempt::accepted(1)
            }
        }
        Some(first) if first.is_ascii_digit() => {
            Attempt::accepted(1 + separated(&rest[1..], char::is_ascii_digit))
        }
        _ => Attempt::refused(0),
    }
}

fn separated(rest: &[char], digit: fn(&char) -> bool) -> usize {
    let mut at = 0;
    loop {
        if rest.get(at).is_some_and(digit) {
            at += 1;
        } else if rest.get(at) == Some(&'_') && rest.get(at + 1).is_some_and(digit) {
            at += 2;
        } else {
            return at;
        }
    }
}

fn name(rest: &[char], first: fn(&char) -> bool) -> Attempt {
    let inner = |char: &char| char.is_ascii_alphanumeric() || *char == '_';
    match rest.first() {
        Some(char) if first(char) => {
            Attempt::accepted(1 + rest[1..].iter().take_while(|c| inner(c)).count())
        }
        Some('_') => {
            let length = rest[1..].iter().take_while(|c| inner(c)).count();
            if length > 0 {
                Attempt::accepted(1 + length)
            } else {
                Attempt::refused(1)
            }
        }
        _ => Attempt::refused(0),
    }
}
