use crate::explore::{Note, Search};
use hashbrown::hash_table::{Entry, HashTable};
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::hash::BuildHasher;

const NO_GROUP: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Spot {
    label_end: usize,
    states: u32,
    group: u32,
    start: bool,
    end: bool,
    marked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Link {
    from: u32,
    to: u32,
    labels: u32,
}

/// What a chart needs to know of a search's graph, held in a few flat lists instead of a
/// string for every state and arrow: the boxes with their labels, groups, marks and state
/// counts, and the arrows between them with their labels. A search fills
/// [`Outcome::outline`] when [`Limits::outline`] asks for it.
///
/// Boxes and arrows are numbered as in [`Graph`]: in the order their first state was visited
/// and their first action was taken. A box stands for every state with the same label and
/// group, and an arrow for every action from a state of one box to a state of another.
///
/// [`Limits::outline`]: crate::Limits::outline
/// [`Outcome::outline`]: crate::Outcome::outline
/// [`Graph`]: crate::Graph
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outline {
    title: Option<String>,
    colors: Vec<(String, String)>,
    text: String,
    spots: Vec<Spot>,
    groups: Vec<String>,
    links: Vec<Link>,
    steps: Vec<String>,
    sets: Vec<Vec<u32>>,
}

impl Outline {
    /// The view's title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The view's colors as pairs of `broken`, `start` or `end` and a CSS color.
    #[must_use]
    pub fn colors(&self) -> &[(String, String)] {
        &self.colors
    }

    /// How many boxes there are. A box is named by its position below this.
    #[must_use]
    pub fn boxes(&self) -> usize {
        self.spots.len()
    }

    /// The label of the box at `at`: the view's state label, or every field as
    /// `name = value` when the view has none. Empty for a position past the last box.
    #[must_use]
    pub fn label(&self, at: usize) -> &str {
        let begin = at
            .checked_sub(1)
            .and_then(|before| self.spots.get(before))
            .map_or(0, |spot| spot.label_end);
        self.spots
            .get(at)
            .and_then(|spot| self.text.get(begin..spot.label_end))
            .unwrap_or_default()
    }

    /// The view's group label of the box at `at`, when the view has one.
    #[must_use]
    pub fn group(&self, at: usize) -> Option<&str> {
        let spot = self.spots.get(at)?;
        let position = usize::try_from(spot.group).ok()?;
        self.groups.get(position).map(String::as_str)
    }

    /// Whether one of the states of the box at `at` is a start state.
    #[must_use]
    pub fn start(&self, at: usize) -> bool {
        self.spots.get(at).is_some_and(|spot| spot.start)
    }

    /// Whether one of the states of the box at `at` has no action and is marked as an end.
    #[must_use]
    pub fn end(&self, at: usize) -> bool {
        self.spots.get(at).is_some_and(|spot| spot.end)
    }

    /// Whether one of the states of the box at `at` breaks a rule or has a field out of range.
    #[must_use]
    pub fn marked(&self, at: usize) -> bool {
        self.spots.get(at).is_some_and(|spot| spot.marked)
    }

    /// How many states the box at `at` stands for.
    #[must_use]
    pub fn states(&self, at: usize) -> usize {
        self.spots
            .get(at)
            .and_then(|spot| usize::try_from(spot.states).ok())
            .unwrap_or(0)
    }

    /// How many arrows there are. An arrow is named by its position below this.
    #[must_use]
    pub fn arrows(&self) -> usize {
        self.links.len()
    }

    /// The boxes the arrow at `at` leaves and enters, which can be the same box; `None` for a
    /// position past the last arrow.
    #[must_use]
    pub fn ends(&self, at: usize) -> Option<(usize, usize)> {
        let link = self.links.get(at)?;
        Some((
            usize::try_from(link.from).ok()?,
            usize::try_from(link.to).ok()?,
        ))
    }

    /// The label of each action along the arrow at `at`, each once, in the order they were
    /// first taken: the view's step label or the action's sentence.
    pub fn arrow_labels(&self, at: usize) -> impl Iterator<Item = &str> {
        self.links
            .get(at)
            .and_then(|link| usize::try_from(link.labels).ok())
            .and_then(|set| self.sets.get(set))
            .into_iter()
            .flatten()
            .filter_map(|step| usize::try_from(*step).ok())
            .filter_map(|step| self.steps.get(step).map(String::as_str))
    }
}

fn spot_hash(group: u32, label: &str) -> u64 {
    FxBuildHasher.hash_one((group, label))
}

fn link_hash(from: u32, to: u32) -> u64 {
    FxBuildHasher.hash_one((from, to))
}

#[derive(Default)]
struct Names {
    known: FxHashMap<String, u32>,
    names: Vec<String>,
}

impl Names {
    fn id(&mut self, name: &str) -> Option<u32> {
        if let Some(id) = self.known.get(name) {
            return Some(*id);
        }
        let id = u32::try_from(self.names.len()).ok()?;
        self.known.insert(name.to_owned(), id);
        self.names.push(name.to_owned());
        Some(id)
    }
}

impl Search<'_> {
    fn mark_spot(&self, spot: &mut Spot, id: usize) {
        spot.states = spot.states.saturating_add(1);
        spot.start |= id < self.store.len() && self.store.parent(id).is_none();
        let Some(Note {
            end,
            outside,
            broken,
            ..
        }) = self.notes.get(id)
        else {
            return;
        };
        spot.end |= end;
        spot.marked |= outside.is_some_and(|position| position < self.model.fields.len())
            || broken.iter().any(|rule| *rule < self.model.rules.len());
    }

    fn outline_boxes(&self, outline: &mut Outline) -> Option<Vec<u32>> {
        let view = &self.model.view;
        let mut groups = Names::default();
        let mut places: HashTable<u32> = HashTable::new();
        let mut box_of = Vec::with_capacity(self.store.len());
        let (mut row, mut values) = (Vec::new(), Vec::new());
        let (mut label, mut group) = (String::new(), String::new());
        for id in 0..self.store.len() {
            self.store.read_state(id, &mut row, &mut values);
            let state = values.as_slice();
            label.clear();
            self.write_state_label(state, &mut label);
            let group_id = match &view.group {
                Some(found) => {
                    group.clear();
                    self.write_label(found, state, &mut group);
                    groups.id(&group)?
                }
                None => NO_GROUP,
            };
            let (spots, text) = (&outline.spots, &outline.text);
            let label_of = |at: u32| {
                let at = usize::try_from(at).ok()?;
                let begin = at
                    .checked_sub(1)
                    .and_then(|before| spots.get(before))
                    .map_or(0, |spot| spot.label_end);
                let spot = spots.get(at)?;
                Some((spot.group, text.get(begin..spot.label_end)?))
            };
            let place = match places.entry(
                spot_hash(group_id, &label),
                |at| label_of(*at) == Some((group_id, label.as_str())),
                |at| label_of(*at).map_or(0, |(group, text)| spot_hash(group, text)),
            ) {
                Entry::Occupied(found) => *found.get(),
                Entry::Vacant(free) => {
                    let place = u32::try_from(outline.spots.len()).ok()?;
                    outline.text.push_str(&label);
                    outline.spots.push(Spot {
                        label_end: outline.text.len(),
                        group: group_id,
                        ..Spot::default()
                    });
                    *free.insert(place).get()
                }
            };
            box_of.push(place);
            let spot = outline.spots.get_mut(usize::try_from(place).ok()?)?;
            self.mark_spot(spot, id);
        }
        outline.groups = groups.names;
        Some(box_of)
    }

    fn outline_arrows(&self, outline: &mut Outline, box_of: &[u32]) -> Option<()> {
        let mut steps = Names::default();
        let step_ids = (0..self.model.actions.len())
            .map(|action| steps.id(self.step_label(action)))
            .collect::<Option<Vec<u32>>>()?;
        let no_step = steps.id("")?;
        let mut more: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        let mut links_at: HashTable<u32> = HashTable::new();
        for arrow in &self.arrows {
            let (Some(&from), Some(&to)) = (box_of.get(arrow.from), box_of.get(arrow.to)) else {
                continue;
            };
            let step = step_ids.get(arrow.action).copied().unwrap_or(no_step);
            let links = &mut outline.links;
            let ends_at = |at: u32| {
                let link = links.get(usize::try_from(at).ok()?)?;
                Some((link.from, link.to))
            };
            match links_at.entry(
                link_hash(from, to),
                |at| ends_at(*at) == Some((from, to)),
                |at| ends_at(*at).map_or(0, |(from, to)| link_hash(from, to)),
            ) {
                Entry::Occupied(found) => {
                    let at = *found.get();
                    let first = links.get(usize::try_from(at).ok()?)?.labels;
                    if first != step {
                        let rest = more.entry(at).or_default();
                        if !rest.contains(&step) {
                            rest.push(step);
                        }
                    }
                }
                Entry::Vacant(free) => {
                    free.insert(u32::try_from(links.len()).ok()?);
                    links.push(Link {
                        from,
                        to,
                        labels: step,
                    });
                }
            }
        }
        outline.sets = (0..u32::try_from(steps.names.len()).ok()?)
            .map(|step| vec![step])
            .collect();
        let mut sets_at: FxHashMap<Vec<u32>, u32> = FxHashMap::default();
        let mut extended: Vec<(u32, Vec<u32>)> = more.into_iter().collect();
        extended.sort_unstable_by_key(|(at, _)| *at);
        for (at, rest) in extended {
            let link = outline.links.get_mut(usize::try_from(at).ok()?)?;
            let mut set = vec![link.labels];
            set.extend(rest);
            let next = u32::try_from(outline.sets.len()).ok()?;
            let id = *sets_at.entry(set.clone()).or_insert(next);
            if id == next {
                outline.sets.push(set);
            }
            link.labels = id;
        }
        outline.steps = steps.names;
        Some(())
    }

    pub(crate) fn outline(&self) -> Option<Outline> {
        u32::try_from(self.store.len()).ok()?;
        let view = &self.model.view;
        let mut outline = Outline {
            title: view.title.clone(),
            colors: view.colors.clone(),
            ..Outline::default()
        };
        let box_of = self.outline_boxes(&mut outline)?;
        self.outline_arrows(&mut outline, &box_of)?;
        Some(outline)
    }
}
