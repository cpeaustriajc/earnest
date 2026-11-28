use crate::build::{Sketch, helpers_of, int, two};
use earnest_ir::{Math, Op};
use earnest_value::{Notation, Quint};

const FAULT: &str = "Fault(message) == Assert(FALSE, message)";
const FLOORED_REMAINDER: &str = "FlooredRemainder(left, right) == IF right > 0 THEN left % right \
                                 ELSE IF right < 0 THEN -((-left) % (-right)) ELSE \
                                 Fault(\"a division or a remainder by zero\")";
const FLOORED_DIVISION: &str = "FlooredDivision(left, right) == IF right > 0 THEN left \\div \
                                right ELSE IF right < 0 THEN (-left) \\div (-right) ELSE \
                                Fault(\"a division or a remainder by zero\")";

fn quint(math: Math) -> String {
    let mut sketch = Sketch::new("Floors");
    sketch.symbols =
        std::mem::take(&mut sketch.symbols).written_in(Notation::Quint(Quint::default()));
    let n = sketch.number("n", -3, 3, 0);
    sketch.rule("part", two(Op::Math(math), &n, int(-2)));
    sketch.written().module
}

#[test]
fn a_quint_division_floors_with_a_positive_divisor_only_and_faults_on_zero() {
    let module = quint(Math::Divide);
    assert!(module.contains("FlooredDivision(n, (-2))"), "{module}");
    assert_eq!(helpers_of(&module), [FAULT, FLOORED_DIVISION]);
}

#[test]
fn a_quint_remainder_follows_the_floor_with_a_positive_divisor_only() {
    let module = quint(Math::Remainder);
    assert!(module.contains("FlooredRemainder(n, (-2))"), "{module}");
    assert_eq!(helpers_of(&module), [FAULT, FLOORED_REMAINDER]);
}
