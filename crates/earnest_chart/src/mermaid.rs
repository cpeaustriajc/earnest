use crate::markdown::code_span;
use crate::plot::{Look, Plot};
use crate::select::{ARROW_LIMIT, Found, Selection, select};
use earnest_check::{Graph, Outline, PremiseKind};
use std::collections::HashMap;
use std::fmt::Write as _;

const INDENT: &str = "    ";
const DEFAULT_BROKEN: &str = "fill:#c62828,color:#ffffff";
const KEPT_PUNCTUATION: &str = ",.'()=+/!?-_";
const DECODED_BY_MERMAID: [char; 2] = ['\u{fb02}', '\u{b6}'];
const ZERO_WIDTH_SPACE: char = '\u{200b}';
const LONGEST_COLOR_NAME: usize = 32;
const DARK_TEXT_FROM: u32 = 128_000;

struct Class {
    name: &'static str,
    color: &'static str,
    fallback: Option<&'static str>,
    applies: fn(Look) -> bool,
}

const CLASSES: [Class; 3] = [
    Class {
        name: "broken",
        color: "broken",
        fallback: Some(DEFAULT_BROKEN),
        applies: |look| look.marked,
    },
    Class {
        name: "ending",
        color: "end",
        fallback: None,
        applies: |look| look.end,
    },
    Class {
        name: "starting",
        color: "start",
        fallback: None,
        applies: |look| look.start,
    },
];

fn label(text: &str) -> String {
    let mut safe = String::new();
    for character in text.chars() {
        if character.is_whitespace() || character.is_control() {
            safe.push(' ');
        } else if DECODED_BY_MERMAID.contains(&character)
            || (character.is_ascii()
                && !character.is_ascii_alphanumeric()
                && !KEPT_PUNCTUATION.contains(character))
        {
            let _ = write!(safe, "#{};", u32::from(character));
        } else {
            safe.push(character);
        }
    }
    if safe.is_empty() {
        safe.push(' ');
    }
    safe
}

fn title(text: &str) -> String {
    let mut safe = String::new();
    for character in text.chars() {
        if character.is_whitespace() || character.is_control() {
            safe.push(' ');
        } else if DECODED_BY_MERMAID.contains(&character) {
            safe.push(character);
            safe.push(ZERO_WIDTH_SPACE);
        } else if matches!(character, '"' | '\\' | '%' | '#' | '{' | '}') {
            let _ = write!(safe, "\\x{:02X}", u32::from(character));
        } else {
            safe.push(character);
        }
    }
    safe
}

fn channel(digits: &str) -> Option<u32> {
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(if digits.len() == 1 {
        value.saturating_mul(17)
    } else {
        value
    })
}

fn text_on(hex: &str) -> Option<&'static str> {
    let width = match hex.len() {
        3 | 4 => 1,
        6 | 8 => 2,
        _ => return None,
    };
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let red = channel(hex.get(..width)?)?;
    let green = channel(hex.get(width..width.saturating_mul(2))?)?;
    let blue = channel(hex.get(width.saturating_mul(2)..width.saturating_mul(3))?)?;
    let brightness = red
        .saturating_mul(299)
        .saturating_add(green.saturating_mul(587))
        .saturating_add(blue.saturating_mul(114));
    Some(if brightness >= DARK_TEXT_FROM {
        "#000000"
    } else {
        "#ffffff"
    })
}

fn style(color: &str) -> Option<String> {
    let color = color.trim();
    if let Some(hex) = color.strip_prefix('#') {
        return text_on(hex).map(|text| format!("fill:{color},color:{text}"));
    }
    let named = !color.is_empty()
        && color.len() <= LONGEST_COLOR_NAME
        && color.bytes().all(|byte| byte.is_ascii_alphabetic());
    named.then(|| format!("fill:{color}"))
}

fn style_of(colors: &[(String, String)], class: &Class) -> Option<String> {
    colors
        .iter()
        .filter(|(name, _)| name == class.color)
        .find_map(|(_, color)| style(color))
        .or_else(|| class.fallback.map(str::to_owned))
}

struct Drawing<'a, P> {
    plot: &'a P,
    selection: Selection,
    groups: Vec<&'a str>,
    group_of: Vec<Option<usize>>,
    members: Vec<Vec<usize>>,
    homes: Vec<Vec<usize>>,
}

impl<'a, P: Plot> Drawing<'a, P> {
    fn new(plot: &'a P, selection: Selection) -> Self {
        let mut groups: Vec<&'a str> = Vec::new();
        let mut known: HashMap<&'a str, usize> = HashMap::new();
        let mut group_of = Vec::with_capacity(selection.states.len());
        for state in &selection.states {
            let place = plot.group(*state).map(|group| {
                let next = groups.len();
                let place = *known.entry(group).or_insert(next);
                if place == next {
                    groups.push(group);
                }
                place
            });
            group_of.push(place);
        }
        let mut drawing = Self {
            plot,
            selection,
            groups,
            group_of,
            members: Vec::new(),
            homes: Vec::new(),
        };
        let slots = drawing.groups.len().saturating_add(1);
        let mut members = vec![Vec::new(); slots];
        for (state, group) in drawing.selection.states.iter().zip(&drawing.group_of) {
            members[drawing.slot(*group)].push(*state);
        }
        let mut homes = vec![Vec::new(); slots];
        for position in &drawing.selection.arrows {
            let (from, to) = plot.ends(*position);
            let from_group = drawing.group_at(from);
            let home = from_group.filter(|_| from_group == drawing.group_at(to));
            homes[drawing.slot(home)].push(*position);
        }
        drawing.members = members;
        drawing.homes = homes;
        drawing
    }

    fn slot(&self, group: Option<usize>) -> usize {
        group.unwrap_or(self.groups.len())
    }

    fn group_at(&self, state: usize) -> Option<usize> {
        let at = self.selection.states.binary_search(&state).ok()?;
        self.group_of[at]
    }

    fn within(&self, group: Option<usize>) -> impl Iterator<Item = usize> {
        self.members[self.slot(group)].iter().copied()
    }

    fn arrows(&self, group: Option<usize>, indent: &str) -> Vec<String> {
        let mut lines: Vec<String> = self
            .within(group)
            .filter(|state| self.plot.look(*state).start)
            .map(|state| format!("{indent}[*] --> s{state}"))
            .collect();
        for position in &self.homes[self.slot(group)] {
            let (from, to) = self.plot.ends(*position);
            let labels: Vec<String> = self
                .plot
                .arrow_labels(*position)
                .filter(|text| !text.trim().is_empty())
                .map(label)
                .collect();
            let mut line = format!("{indent}s{from} --> s{to}");
            if !labels.is_empty() {
                let _ = write!(line, " : {}", labels.join(" / "));
            }
            lines.push(line);
        }
        lines.extend(
            self.within(group)
                .filter(|state| self.plot.look(*state).end)
                .map(|state| format!("{indent}s{state} --> [*]")),
        );
        lines
    }

    fn declarations(&self) -> Vec<String> {
        let inner = INDENT.repeat(2);
        let mut opened = vec![false; self.groups.len()];
        let mut lines = Vec::new();
        for (state, group) in self.selection.states.iter().zip(&self.group_of) {
            let Some(group) = *group else {
                lines.push(format!(
                    "{INDENT}state \"{}\" as s{state}",
                    label(self.plot.label(*state))
                ));
                continue;
            };
            if opened[group] {
                continue;
            }
            opened[group] = true;
            lines.push(format!(
                "{INDENT}state \"{}\" as g{group} {{",
                label(self.groups[group])
            ));
            lines.extend(self.within(Some(group)).map(|member| {
                format!(
                    "{inner}state \"{}\" as s{member}",
                    label(self.plot.label(member))
                )
            }));
            lines.extend(self.arrows(Some(group), &inner));
            lines.push(format!("{INDENT}}}"));
        }
        lines
    }

    fn classes(&self) -> Vec<String> {
        let styles: Vec<Option<String>> = CLASSES
            .iter()
            .map(|class| style_of(self.plot.colors(), class))
            .collect();
        let mut members = vec![Vec::new(); CLASSES.len()];
        for state in &self.selection.states {
            let look = self.plot.look(*state);
            let class = CLASSES
                .iter()
                .zip(&styles)
                .position(|(class, style)| style.is_some() && (class.applies)(look));
            if let Some(class) = class {
                members[class].push(format!("s{state}"));
            }
        }
        let mut lines = Vec::new();
        for ((class, style), members) in CLASSES.iter().zip(styles).zip(members) {
            let Some(style) = style.filter(|_| !members.is_empty()) else {
                continue;
            };
            lines.push(format!("{INDENT}classDef {} {style}", class.name));
            lines.push(format!(
                "{INDENT}class {} {}",
                members.join(","),
                class.name
            ));
        }
        lines
    }

    fn left_out(&self) -> Option<String> {
        let boxes = self.plot.boxes();
        let arrows = self.plot.arrows();
        let hidden_boxes = boxes.saturating_sub(self.selection.states.len());
        let hidden_arrows = arrows.saturating_sub(self.selection.arrows.len());
        let states: usize = (0..boxes).map(|at| self.plot.states(at)).sum();
        let shown: usize = self
            .selection
            .states
            .iter()
            .map(|position| self.plot.states(*position))
            .sum();
        let hidden_states = states.saturating_sub(shown);
        let beyond = match (self.selection.beyond, self.selection.shown) {
            (0, _) => String::new(),
            (1, 0) => " A broken state lies beyond what is shown.".to_owned(),
            (many, 0) => format!(" {many} broken states lie beyond what is shown."),
            (1, _) => " Another broken state lies beyond what is shown.".to_owned(),
            (many, _) => format!(" {many} more broken states lie beyond what is shown."),
        };
        (hidden_boxes > 0 || hidden_arrows > 0).then(|| {
            format!(
                "Not shown: {hidden_states} of {states} states and {hidden_arrows} of {arrows} arrows.{beyond}"
            )
        })
    }

    fn place(&self, found: Found) -> String {
        match found {
            Found::Drawn(at) => format!("in {}", code_span(self.plot.label(at))),
            Found::Beyond => "beyond what is shown".to_owned(),
        }
    }

    fn premise_note(&self) -> Option<String> {
        let premises = self.plot.premises();
        let mut sentences: Vec<String> = Vec::new();
        let mut at = 0;
        while let Some(first) = premises.get(at) {
            let rule = &first.rule;
            let mut pieces: Vec<String> = Vec::new();
            let mut implied = None;
            while let Some(premise) = premises.get(at).filter(|premise| premise.rule == *rule) {
                if let Some(Some(found)) = self.selection.premises.get(at) {
                    let place = self.place(*found);
                    match premise.kind {
                        PremiseKind::After => pieces.push(format!("starts {place}")),
                        PremiseKind::Reaches => pieces.push(format!("is met {place}")),
                        PremiseKind::Implies => implied = Some(place),
                        _ => {}
                    }
                }
                at = at.saturating_add(1);
            }
            let rule = code_span(rule);
            if let Some(place) = implied {
                sentences.push(format!("The premise of {rule} first holds {place}."));
            }
            if !pieces.is_empty() {
                sentences.push(format!("{rule} {}.", pieces.join(" and ")));
            }
        }
        (!sentences.is_empty()).then(|| sentences.join(" "))
    }

    fn write(&self) -> String {
        let mut lines = vec!["```mermaid".to_owned()];
        if let Some(text) = self.plot.title() {
            lines.push("---".to_owned());
            lines.push(format!("title: \"{}\"", title(text)));
            lines.push("---".to_owned());
        }
        lines.push("stateDiagram-v2".to_owned());
        lines.extend(self.declarations());
        lines.extend(self.arrows(None, INDENT));
        lines.extend(self.classes());
        lines.push("```".to_owned());
        if let Some(note) = self.left_out() {
            lines.push(String::new());
            lines.push(note);
        }
        if let Some(note) = self.premise_note() {
            lines.push(String::new());
            lines.push(note);
        }
        lines.join("\n")
    }
}

/// The graph as one fenced `mermaid` block holding a `stateDiagram-v2` chart, ready to paste
/// into Markdown that GitHub renders. The text has no trailing line break.
///
/// A state is declared as `s<n>`, where `n` is its position in the graph's nodes, with its label
/// attached; a group is a composite state `g<n>`. Every character of a label that Mermaid could
/// read as syntax is written as a `#<code>;` entity, and a line break in a label becomes a
/// space. A state that breaks a rule or is out of range has the class `broken`. The view's
/// `broken` color replaces the default red; its `start` and `end` colors add the classes
/// `starting` and `ending`. A color that is not `#` with 3, 4, 6 or 8 hex digits or a plain
/// color name is ignored.
///
/// At most [`ARROW_LIMIT`](crate::ARROW_LIMIT) arrows are drawn. When the graph has more, or
/// has a state no start state leads to, the block is followed by a blank line and one line of
/// the form `Not shown: 16 of 36 states and 40 of 60 arrows.` It counts states, as the boxes
/// they stand for are drawn together: a box left out takes every state it stands for with it,
/// and an arrow is one box to another, however many actions it carries. When the route to a state that
/// breaks a rule or is out of range does not fit, its first arrows are drawn as far as the
/// limit allows, the state itself is not, and the line goes on with `A broken state lies
/// beyond what is shown.` or `2 broken states lie beyond what is shown.` When another broken
/// state is drawn, it says `Another broken state lies beyond what is shown.` or `2 more broken
/// states lie beyond what is shown.`, so it never reads as if the drawn one were missing.
///
/// After the routes to broken states, the arrows left go to the route to the nearest box where
/// each of [`Graph::premises`] holds, in that order; a premise that holds in a box already
/// drawn costs none. When any premise holds in a box a start leads to, the text ends with a
/// blank line and one line naming, per rule, the box where it first holds, such as
/// ``The premise of `paid` first holds in `Paid`.`` or
/// ``` `settles` starts in `Busy` and is met in `Idle`.```, with `beyond what is shown` in place
/// of a box whose route did not fit. A graph with no premise gets the chart it got before
/// premises.
#[must_use]
pub fn chart(graph: &Graph) -> String {
    chart_with_limit(graph, ARROW_LIMIT)
}

/// [`chart`] with room for `limit` arrows instead of [`ARROW_LIMIT`](crate::ARROW_LIMIT).
/// A `limit` of 0 draws no arrow.
#[must_use]
pub fn chart_with_limit(graph: &Graph, limit: usize) -> String {
    Drawing::new(graph, select(graph, limit)).write()
}

/// The same chart as [`chart_with_limit`] of the graph `outline` was made from, to the byte,
/// with room for `limit` arrows, drawn from the outline a search left in
/// [`Limits::outline`](earnest_check::Limits::outline) in place of its graph.
#[must_use]
pub fn chart_outline(outline: &Outline, limit: usize) -> String {
    Drawing::new(outline, select(outline, limit)).write()
}
