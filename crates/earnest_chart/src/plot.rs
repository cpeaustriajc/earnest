use earnest_check::{Graph, Node, Outline, Premise};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Look {
    pub(crate) start: bool,
    pub(crate) end: bool,
    pub(crate) marked: bool,
}

fn marked(node: &Node) -> bool {
    !node.broken.is_empty() || !node.outside.is_empty()
}

pub(crate) trait Plot {
    fn title(&self) -> Option<&str>;
    fn colors(&self) -> &[(String, String)];
    fn boxes(&self) -> usize;
    fn label(&self, at: usize) -> &str;
    fn group(&self, at: usize) -> Option<&str>;
    fn look(&self, at: usize) -> Look;
    fn states(&self, at: usize) -> usize;
    fn arrows(&self) -> usize;
    fn ends(&self, at: usize) -> (usize, usize);
    fn arrow_labels(&self, at: usize) -> impl Iterator<Item = &str>;
    fn premises(&self) -> &[Premise];
    fn holding(&self, premise: usize) -> impl Iterator<Item = usize>;
}

impl Plot for Graph {
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn colors(&self) -> &[(String, String)] {
        &self.colors
    }

    fn boxes(&self) -> usize {
        self.nodes.len()
    }

    fn label(&self, at: usize) -> &str {
        &self.nodes[at].label
    }

    fn group(&self, at: usize) -> Option<&str> {
        self.nodes[at].group.as_deref()
    }

    fn look(&self, at: usize) -> Look {
        let node = &self.nodes[at];
        Look {
            start: node.start,
            end: node.end,
            marked: marked(node),
        }
    }

    fn states(&self, at: usize) -> usize {
        self.nodes[at].states
    }

    fn arrows(&self) -> usize {
        self.edges.len()
    }

    fn ends(&self, at: usize) -> (usize, usize) {
        let edge = &self.edges[at];
        (edge.from, edge.to)
    }

    fn arrow_labels(&self, at: usize) -> impl Iterator<Item = &str> {
        self.edges[at].labels.iter().map(String::as_str)
    }

    fn premises(&self) -> &[Premise] {
        &self.premises
    }

    fn holding(&self, premise: usize) -> impl Iterator<Item = usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(_, node)| node.premises.binary_search(&premise).is_ok())
            .map(|(at, _)| at)
    }
}

impl Plot for Outline {
    fn title(&self) -> Option<&str> {
        self.title()
    }

    fn colors(&self) -> &[(String, String)] {
        self.colors()
    }

    fn boxes(&self) -> usize {
        self.boxes()
    }

    fn label(&self, at: usize) -> &str {
        self.label(at)
    }

    fn group(&self, at: usize) -> Option<&str> {
        self.group(at)
    }

    fn look(&self, at: usize) -> Look {
        Look {
            start: self.start(at),
            end: self.end(at),
            marked: self.marked(at),
        }
    }

    fn states(&self, at: usize) -> usize {
        self.states(at)
    }

    fn arrows(&self) -> usize {
        self.arrows()
    }

    fn ends(&self, at: usize) -> (usize, usize) {
        self.ends(at).unwrap_or_default()
    }

    fn arrow_labels(&self, at: usize) -> impl Iterator<Item = &str> {
        self.arrow_labels(at)
    }

    fn premises(&self) -> &[Premise] {
        self.premises()
    }

    fn holding(&self, premise: usize) -> impl Iterator<Item = usize> {
        self.holding(premise)
    }
}
