use crate::build::{Sketch, body_of, int, two};
use earnest_ir::{Math, Op};
use earnest_value::{Domain, Value};

fn listed() -> Sketch {
    let mut sketch = Sketch::new("Listed");
    let count = sketch.field("count", Domain::Any, vec![Value::Int(0), Value::Int(2)]);
    let on = sketch.field("on", Domain::Bool, vec![Value::Bool(false)]);
    sketch.action(
        "inc",
        two(Op::Equal, &on, on.clone()),
        &[(&count, two(Op::Math(Math::Add), &count, int(1)))],
    );
    sketch
}

#[test]
fn listed_start_states_are_written_as_one_disjunction_of_whole_states() {
    let mut model = listed().model();
    model.runs.starts = Some(vec![
        vec![Value::Int(0), Value::Bool(false)],
        vec![Value::Int(2), Value::Bool(false)],
    ]);
    let files = earnest_tla::write(&model);
    assert_eq!(
        body_of(&files.module, "Init"),
        ["/\\ (((count = 0) /\\ (on = FALSE)) \\/ ((count = 2) /\\ (on = FALSE)))"],
        "each listed start state is one disjunct, not the product of the fields' starts"
    );
}

#[test]
fn a_field_that_holds_every_value_is_always_in_range() {
    let files = earnest_tla::write(&listed().model());
    assert_eq!(
        body_of(&files.module, "InRange"),
        ["/\\ TRUE", "/\\ (on \\in BOOLEAN)"],
        "a field of every value has no range to check"
    );
}
