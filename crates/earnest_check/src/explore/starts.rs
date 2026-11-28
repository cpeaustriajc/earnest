use earnest_ir::{Field, Model};
use earnest_value::Value;

pub(crate) struct Starts<'a> {
    fields: &'a [Field],
    digits: Vec<usize>,
    done: bool,
    listed: Option<std::slice::Iter<'a, Vec<Value>>>,
}

impl<'a> Starts<'a> {
    pub(crate) fn new(fields: &'a [Field]) -> Self {
        Self {
            fields,
            digits: vec![0; fields.len()],
            done: false,
            listed: None,
        }
    }

    pub(crate) fn of(model: &'a Model) -> Self {
        let mut starts = Self::new(&model.fields);
        starts.listed = model.runs.starts.as_ref().map(|listed| listed.iter());
        starts
    }

    fn advance(&mut self) {
        for (digit, field) in self.digits.iter_mut().zip(self.fields).rev() {
            let next = digit.saturating_add(1);
            if next < field.starts.len() {
                *digit = next;
                return;
            }
            *digit = 0;
        }
        self.done = true;
    }
}

impl Iterator for Starts<'_> {
    type Item = Vec<Value>;

    fn next(&mut self) -> Option<Vec<Value>> {
        if let Some(listed) = &mut self.listed {
            return listed.next().cloned();
        }
        if self.done {
            return None;
        }
        let state: Option<Vec<Value>> = self
            .fields
            .iter()
            .zip(&self.digits)
            .map(|(field, digit)| field.starts.get(*digit).cloned())
            .collect();
        if state.is_none() {
            self.done = true;
        }
        self.advance();
        state
    }
}

#[cfg(test)]
mod tests {
    use super::Starts;
    use earnest_ir::Field;
    use earnest_value::{Domain, Symbols, Value};

    fn field(symbols: &mut Symbols, name: &str, starts: &[i64]) -> Field {
        Field {
            name: name.to_owned(),
            sym: symbols.intern(name),
            domain: Domain::Int { lo: 0, hi: 9 },
            starts: starts.iter().copied().map(Value::Int).collect(),
        }
    }

    fn listed(fields: &[Field]) -> Vec<Vec<Value>> {
        Starts::new(fields).collect()
    }

    fn numbers(all: &[i64]) -> Vec<Value> {
        all.iter().copied().map(Value::Int).collect()
    }

    #[test]
    fn start_states_are_every_combination_and_the_last_field_turns_fastest() {
        let mut symbols = Symbols::default();
        let fields = [
            field(&mut symbols, "a", &[0, 1]),
            field(&mut symbols, "b", &[5, 6, 7]),
        ];
        let expected: Vec<Vec<Value>> = [[0, 5], [0, 6], [0, 7], [1, 5], [1, 6], [1, 7]]
            .iter()
            .map(|pair| numbers(pair))
            .collect();
        assert_eq!(listed(&fields), expected);
    }

    #[test]
    fn listed_start_states_are_given_in_their_order_instead_of_the_combinations() {
        let mut symbols = Symbols::default();
        let model = earnest_ir::Model {
            name: "listed".to_owned(),
            file: "listed.json".to_owned(),
            line: 1,
            fields: vec![
                field(&mut symbols, "a", &[0, 1]),
                field(&mut symbols, "b", &[5, 6]),
            ],
            symbols,
            actions: Vec::new(),
            rules: Vec::new(),
            ends: Vec::new(),
            view: earnest_ir::View::default(),
            runs: earnest_ir::Runs {
                starts: Some(vec![numbers(&[1, 6]), numbers(&[0, 5])]),
                ..earnest_ir::Runs::default()
            },
        };
        assert_eq!(
            Starts::of(&model).collect::<Vec<_>>(),
            vec![numbers(&[1, 6]), numbers(&[0, 5])],
            "a model that lists its start states starts from those alone, in the listed order"
        );
    }

    #[test]
    fn a_model_without_fields_has_one_empty_start_state() {
        assert_eq!(listed(&[]), vec![Vec::new()]);
    }

    #[test]
    fn a_field_with_no_start_value_leaves_no_start_state() {
        let mut symbols = Symbols::default();
        let fields = [
            field(&mut symbols, "a", &[0, 1]),
            field(&mut symbols, "b", &[]),
        ];
        assert_eq!(listed(&fields), Vec::<Vec<Value>>::new());
    }
}
