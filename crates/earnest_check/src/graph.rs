use crate::explore::{Note, Search};
use earnest_ir::{Frame, Label, Piece};
use earnest_value::Value;
use hashbrown::hash_table::{Entry, HashTable};
use rustc_hash::FxBuildHasher;
use std::fmt::Write as _;
use std::hash::BuildHasher;

/// One box of a chart: every visited state with the same label and group.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Node {
    /// The view's state label, or every field as `name = value` when the view has none.
    pub label: String,
    /// The view's group label, when the view has one.
    pub group: Option<String>,
    /// Whether one of its states is a start state.
    pub start: bool,
    /// Whether one of its states has no action and is marked as an end.
    pub end: bool,
    /// Whether one of its states has no action and is not marked as an end.
    pub stuck: bool,
    /// The ids of the rules its states break, each once, in the order they were met.
    pub broken: Vec<String>,
    /// The names of the fields that are out of range in its states, each once.
    pub outside: Vec<String>,
    /// How many states it stands for.
    pub states: usize,
}

/// One arrow of a chart: every action that leads from a state of one node to a state of
/// another.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    /// The position in [`Graph::nodes`] of the node it leaves.
    pub from: usize,
    /// The position in [`Graph::nodes`] of the node it enters, which can be the same node.
    pub to: usize,
    /// The label of each action along it, each once: the view's step label or the action's
    /// sentence.
    pub labels: Vec<String>,
}

/// The visited states and the actions between them, merged by label for a chart.
///
/// When the search stopped at a limit, the graph holds what was visited so far, and a state
/// that was found but not yet expanded has no marks and no arrows out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    /// The view's title.
    pub title: Option<String>,
    /// The view's colors as pairs of `broken`, `start` or `end` and a CSS color.
    pub colors: Vec<(String, String)>,
    /// The nodes, in the order their first state was visited.
    pub nodes: Vec<Node>,
    /// The arrows, in the order their first action was taken.
    pub edges: Vec<Edge>,
}

fn add_once(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|known| known == name) {
        names.push(name.to_owned());
    }
}

fn box_hash(group: Option<&str>, label: &str) -> u64 {
    FxBuildHasher.hash_one((group, label))
}

fn arrow_hash(from: usize, to: usize) -> u64 {
    FxBuildHasher.hash_one((from, to))
}

impl Search<'_> {
    pub(crate) fn write_value(&self, value: &Value, text: &mut String) {
        match value {
            Value::Int(number) => {
                let _ = write!(text, "{number}");
            }
            Value::Str(sym) => text.push_str(self.model.symbols.name(*sym)),
            _ => text.push_str(&self.model.symbols.plain(value)),
        }
    }

    pub(crate) fn write_label(&self, label: &Label, state: &[Value], text: &mut String) {
        let mut frame = Frame::new(state, label.slots);
        for piece in &label.pieces {
            match piece {
                Piece::Text(plain) => text.push_str(plain),
                Piece::Shown(expr) => match expr.eval(&mut frame) {
                    Ok(value) => self.write_value(&value, text),
                    Err(_) => text.push('?'),
                },
                _ => text.push('?'),
            }
        }
    }

    pub(crate) fn write_default_label(&self, state: &[Value], text: &mut String) {
        for (position, (field, value)) in self.model.fields.iter().zip(state).enumerate() {
            if position > 0 {
                text.push_str(", ");
            }
            text.push_str(&field.name);
            text.push_str(" = ");
            self.write_value(value, text);
        }
    }

    pub(crate) fn write_state_label(&self, state: &[Value], text: &mut String) {
        match &self.model.view.state {
            Some(label) => self.write_label(label, state, text),
            None => self.write_default_label(state, text),
        }
    }

    pub(crate) fn step_label(&self, action: usize) -> &str {
        let Some(found) = self.model.actions.get(action) else {
            return "";
        };
        self.model
            .view
            .steps
            .iter()
            .find(|(id, _)| *id == found.id)
            .map_or(found.label.as_str(), |(_, label)| label.as_str())
    }

    fn mark_node(&self, node: &mut Node, id: usize) {
        node.states = node.states.saturating_add(1);
        node.start |= id < self.store.len() && self.store.parent(id).is_none();
        let Some(Note {
            end,
            stuck,
            outside,
            broken,
        }) = self.notes.get(id)
        else {
            return;
        };
        node.end |= end;
        node.stuck |= stuck;
        if let Some(field) = outside.and_then(|position| self.model.fields.get(position)) {
            add_once(&mut node.outside, &field.name);
        }
        for rule in broken.iter().filter_map(|rule| self.model.rules.get(*rule)) {
            add_once(&mut node.broken, &rule.id);
        }
    }

    pub(crate) fn graph(&self) -> Graph {
        let view = &self.model.view;
        let mut graph = Graph {
            title: view.title.clone(),
            colors: view.colors.clone(),
            ..Graph::default()
        };
        let mut places: HashTable<usize> = HashTable::new();
        let mut node_of = Vec::with_capacity(self.store.len());
        let (mut row, mut values) = (Vec::new(), Vec::new());
        let (mut label, mut group) = (String::new(), String::new());
        for id in 0..self.store.len() {
            self.store.read_values(id, &mut row, &mut values);
            let state = values.as_slice();
            label.clear();
            self.write_state_label(state, &mut label);
            let grouped = view.group.as_ref().map(|found| {
                group.clear();
                self.write_label(found, state, &mut group);
                group.as_str()
            });
            let hash = box_hash(grouped, &label);
            let nodes = &mut graph.nodes;
            let position = match places.entry(
                hash,
                |at| {
                    nodes
                        .get(*at)
                        .is_some_and(|node| node.label == label && node.group.as_deref() == grouped)
                },
                |at| {
                    nodes
                        .get(*at)
                        .map_or(0, |node| box_hash(node.group.as_deref(), &node.label))
                },
            ) {
                Entry::Occupied(found) => *found.get(),
                Entry::Vacant(free) => {
                    nodes.push(Node {
                        label: label.clone(),
                        group: grouped.map(str::to_owned),
                        ..Node::default()
                    });
                    *free.insert(nodes.len().saturating_sub(1)).get()
                }
            };
            node_of.push(position);
            if let Some(node) = graph.nodes.get_mut(position) {
                self.mark_node(node, id);
            }
        }
        let mut edge_at: HashTable<usize> = HashTable::new();
        for arrow in &self.arrows {
            let (Some(&source), Some(&target)) = (node_of.get(arrow.from), node_of.get(arrow.to))
            else {
                continue;
            };
            let edges = &mut graph.edges;
            let position = match edge_at.entry(
                arrow_hash(source, target),
                |at| {
                    edges
                        .get(*at)
                        .is_some_and(|edge| edge.from == source && edge.to == target)
                },
                |at| {
                    edges
                        .get(*at)
                        .map_or(0, |edge| arrow_hash(edge.from, edge.to))
                },
            ) {
                Entry::Occupied(found) => *found.get(),
                Entry::Vacant(free) => {
                    edges.push(Edge {
                        from: source,
                        to: target,
                        labels: Vec::with_capacity(1),
                    });
                    *free.insert(edges.len().saturating_sub(1)).get()
                }
            };
            if let Some(edge) = graph.edges.get_mut(position) {
                add_once(&mut edge.labels, self.step_label(arrow.action));
            }
        }
        graph
    }
}
