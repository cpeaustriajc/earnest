use crate::json::Json;
use crate::lexer::{Token, signed};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Point {
    pub(crate) line: usize,
    pub(crate) column: isize,
    pub(crate) index: isize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Place {
    pub(crate) source: usize,
    pub(crate) start: Point,
    pub(crate) end: Option<Point>,
}

impl Place {
    pub(crate) fn of(source: usize, start: &Token, stop: &Token) -> Self {
        let column = if stop.stop == 0 {
            signed(stop.column)
        } else {
            signed(start.column) + (stop.stop - signed(start.start))
        };
        Self {
            source,
            start: Point {
                line: start.line - 1,
                column: signed(start.column),
                index: signed(start.start),
            },
            end: Some(Point {
                line: stop.line - 1,
                column,
                index: stop.stop,
            }),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Ids {
    last: u64,
    places: Vec<(u64, Place)>,
    sources: Vec<String>,
}

impl Ids {
    pub(crate) fn source(&mut self, name: &str) -> usize {
        self.sources.push(name.to_owned());
        self.sources.len() - 1
    }

    pub(crate) fn mark(&self) -> (u64, usize) {
        (self.last, self.places.len())
    }

    pub(crate) fn rewind(&mut self, (last, places): (u64, usize)) {
        self.last = last;
        self.places.truncate(places);
    }

    pub(crate) fn unplaced(&mut self) -> u64 {
        self.last += 1;
        self.last
    }

    pub(crate) fn placed(&mut self, place: Place) -> u64 {
        let id = self.unplaced();
        self.places.push((id, place));
        id
    }

    pub(crate) fn place(&self, id: u64) -> Option<&Place> {
        self.places
            .binary_search_by_key(&id, |(known, _)| *known)
            .ok()
            .map(|index| &self.places[index].1)
    }

    pub(crate) fn source_name(&self, source: usize) -> &str {
        &self.sources[source]
    }

    pub(crate) fn compact(&self) -> Json {
        let mut first_entries: Vec<(usize, usize)> = Vec::new();
        for (entry, (_, place)) in self.places.iter().enumerate() {
            if !first_entries
                .iter()
                .any(|(source, _)| *source == place.source)
            {
                first_entries.push((place.source, entry));
            }
        }
        let index_of = |source: usize| {
            first_entries
                .iter()
                .find(|(known, _)| *known == source)
                .map_or(0, |(_, entry)| *entry)
        };
        let sources = first_entries
            .iter()
            .map(|(source, entry)| (entry.to_string(), Json::text(&self.sources[*source])))
            .collect();
        let map = self
            .places
            .iter()
            .map(|(id, place)| {
                let end = place.end.map_or(Json::Object(Vec::new()), point);
                (
                    id.to_string(),
                    Json::List(vec![
                        Json::number(u64::try_from(index_of(place.source)).unwrap_or(0)),
                        point(place.start),
                        end,
                    ]),
                )
            })
            .collect();
        Json::object(vec![
            ("sourceIndex", Json::Object(sources)),
            ("map", Json::Object(map)),
        ])
    }
}

fn point(point: Point) -> Json {
    Json::object(vec![
        ("line", Json::number(u64::try_from(point.line).unwrap_or(0))),
        ("col", Json::signed(point.column)),
        ("index", Json::signed(point.index)),
    ])
}
