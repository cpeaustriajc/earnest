use crate::{Expr, Frame, Math, Op};
use earnest_value::{IntType, MOST_SAFE_INTEGER, Value};

#[derive(Clone, Copy, Debug)]
enum Leaf {
    Int(i64),
    Field(usize),
    Local(usize),
}

impl Leaf {
    fn of(expr: &Expr) -> Option<Self> {
        match expr {
            Expr::Const(Value::Int(number)) => Some(Self::Int(*number)),
            Expr::Field(index) => Some(Self::Field(*index)),
            Expr::Local(slot) => Some(Self::Local(*slot)),
            _ => None,
        }
    }

    #[inline]
    fn get(self, frame: &Frame<'_>) -> Option<i64> {
        let found = match self {
            Self::Int(number) => return Some(number),
            Self::Field(index) => frame.state.get(index),
            Self::Local(slot) => frame.locals.get(slot),
        };
        match found {
            Some(Value::Int(number)) => Some(*number),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Divisor {
    PowerOfTwo {
        shift: u32,
    },
    Other {
        divisor: u64,
        magic: u64,
        shift: u32,
    },
}

impl Divisor {
    fn of(divisor: i64) -> Option<Self> {
        let divisor = u64::try_from(divisor)
            .ok()
            .filter(|divisor| *divisor >= 1)?;
        if divisor.is_power_of_two() {
            return Some(Self::PowerOfTwo {
                shift: divisor.trailing_zeros(),
            });
        }
        let ceiling_bits = u64::BITS.checked_sub(divisor.leading_zeros())?;
        let scale = 1_u128.checked_shl(ceiling_bits.checked_add(i64::BITS - 1)?)?;
        let wide = u128::from(divisor);
        let magic = u64::try_from(scale.checked_add(wide - 1)?.checked_div(wide)?).ok()?;
        Some(Self::Other {
            divisor,
            magic,
            shift: ceiling_bits.checked_sub(1)?,
        })
    }

    #[inline]
    fn quotient(self, number: u64) -> u64 {
        match self {
            Self::PowerOfTwo { shift } => number >> shift,
            Self::Other { magic, shift, .. } => {
                let high = (u128::from(number) * u128::from(magic)) >> u64::BITS;
                u64::try_from(high).unwrap_or(u64::MAX) >> shift
            }
        }
    }

    #[inline]
    fn remainder(self, number: u64) -> u64 {
        match self {
            Self::PowerOfTwo { shift } => number & ((1_u64 << shift) - 1),
            Self::Other { divisor, .. } => number - self.quotient(number) * divisor,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    LeafFirst,
    LeafSecond,
}

#[derive(Clone, Copy, Debug)]
enum Step {
    Add,
    Subtract,
    SubtractFrom,
    Multiply,
    Divide,
    DivideInto,
    Remainder,
    RemainderOf,
    DivideBy(Divisor),
    RemainderBy(Divisor),
    Negate,
    Keep,
}

impl Step {
    fn of(math: Math, leaf: Leaf, side: Side) -> Self {
        match (math, side, leaf) {
            (Math::Add, _, _) => Self::Add,
            (Math::Multiply, _, _) => Self::Multiply,
            (Math::Subtract, Side::LeafSecond, _) => Self::Subtract,
            (Math::Subtract, Side::LeafFirst, _) => Self::SubtractFrom,
            (Math::Divide, Side::LeafFirst, _) => Self::DivideInto,
            (Math::Remainder, Side::LeafFirst, _) => Self::RemainderOf,
            (Math::Divide, Side::LeafSecond, Leaf::Int(number)) => {
                Divisor::of(number).map_or(Self::Divide, Self::DivideBy)
            }
            (Math::Remainder, Side::LeafSecond, Leaf::Int(number)) => {
                Divisor::of(number).map_or(Self::Remainder, Self::RemainderBy)
            }
            (Math::Divide, Side::LeafSecond, _) => Self::Divide,
            (Math::Remainder, Side::LeafSecond, _) => Self::Remainder,
        }
    }

    #[inline]
    fn apply(self, number: i64, other: i64) -> Option<(i64, bool)> {
        let math = |answer: Option<i64>| answer.map(|answer| (answer, true));
        match self {
            Self::Add => math(number.checked_add(other)),
            Self::Subtract => math(number.checked_sub(other)),
            Self::SubtractFrom => math(other.checked_sub(number)),
            Self::Multiply => math(number.checked_mul(other)),
            Self::Divide => math(divided(number, other)),
            Self::DivideInto => math(divided(other, number)),
            Self::Remainder => math(remainder(number, other)),
            Self::RemainderOf => math(remainder(other, number)),
            Self::DivideBy(divisor) => math(
                u64::try_from(number)
                    .ok()
                    .and_then(|number| i64::try_from(divisor.quotient(number)).ok()),
            ),
            Self::RemainderBy(divisor) => math(
                u64::try_from(number)
                    .ok()
                    .and_then(|number| i64::try_from(divisor.remainder(number)).ok()),
            ),
            Self::Negate => number.checked_neg().map(|answer| (answer, false)),
            Self::Keep => Some((number, false)),
        }
    }
}

fn divided(left: i64, right: i64) -> Option<i64> {
    (left >= 0 && right >= 1).then(|| left / right)
}

fn remainder(left: i64, right: i64) -> Option<i64> {
    (left >= 0 && right >= 1).then(|| left % right)
}

#[derive(Clone, Copy, Debug)]
struct Link {
    step: Step,
    leaf: Leaf,
    lowest: i64,
    highest: i64,
}

#[derive(Clone, Copy, Debug)]
enum Part {
    Leaf(Leaf),
    Computed,
}

#[derive(Clone, Debug)]
pub(super) struct Chain {
    start: Leaf,
    links: Box<[Link]>,
}

struct Building {
    start: Option<Leaf>,
    links: Vec<Link>,
}

const NO_OPERAND: Leaf = Leaf::Int(0);

impl Building {
    fn begin(&mut self, part: Part) -> Option<()> {
        if let Part::Leaf(leaf) = part {
            if self.start.is_some() {
                return None;
            }
            self.start = Some(leaf);
        }
        Some(())
    }

    fn push(&mut self, step: Step, leaf: Leaf) {
        self.links.push(Link {
            step,
            leaf,
            lowest: i64::MIN,
            highest: i64::MAX,
        });
    }

    fn part(&mut self, expr: &Expr) -> Option<Part> {
        if let Some(leaf) = Leaf::of(expr) {
            return Some(Part::Leaf(leaf));
        }
        let Expr::Apply(op, arguments) = expr else {
            return None;
        };
        match (op, arguments.as_slice()) {
            (Op::Math(math), [left, right]) => self.math(*math, left, right),
            (Op::Safe, [inner]) => {
                self.bounded(inner, MOST_SAFE_INTEGER.saturating_neg(), MOST_SAFE_INTEGER)
            }
            (Op::Fit(ty), [inner]) => self.fitted(inner, *ty),
            (Op::Negate, [inner]) => {
                let inner = self.part(inner)?;
                self.begin(inner)?;
                self.push(Step::Negate, NO_OPERAND);
                Some(Part::Computed)
            }
            _ => None,
        }
    }

    fn math(&mut self, math: Math, left: &Expr, right: &Expr) -> Option<Part> {
        let first = self.part(left)?;
        let second = self.part(right)?;
        let (leaf, side) = match (first, second) {
            (Part::Leaf(first), Part::Leaf(second)) => {
                self.begin(Part::Leaf(first))?;
                (second, Side::LeafSecond)
            }
            (Part::Computed, Part::Leaf(second)) => (second, Side::LeafSecond),
            (Part::Leaf(first), Part::Computed) => (first, Side::LeafFirst),
            (Part::Computed, Part::Computed) => return None,
        };
        let step = Step::of(math, leaf, side);
        let read = match step {
            Step::DivideBy(_) | Step::RemainderBy(_) => NO_OPERAND,
            _ => leaf,
        };
        self.push(step, read);
        Some(Part::Computed)
    }

    fn fitted(&mut self, inner: &Expr, ty: IntType) -> Option<Part> {
        self.bounded(inner, ty.lowest(), ty.highest())
    }

    fn bounded(&mut self, inner: &Expr, lowest: i64, highest: i64) -> Option<Part> {
        let inner = self.part(inner)?;
        self.begin(inner)?;
        if matches!(inner, Part::Leaf(_)) {
            self.push(Step::Keep, NO_OPERAND);
        }
        let last = self.links.last_mut()?;
        last.lowest = last.lowest.max(lowest);
        last.highest = last.highest.min(highest);
        Some(Part::Computed)
    }
}

impl Chain {
    pub(super) fn of(expr: &Expr) -> Option<Self> {
        let mut building = Building {
            start: None,
            links: Vec::new(),
        };
        let Part::Computed = building.part(expr)? else {
            return None;
        };
        Some(Self {
            start: building.start?,
            links: building.links.into_boxed_slice(),
        })
    }

    #[inline]
    pub(super) fn run(&self, frame: &mut Frame<'_>) -> Option<i64> {
        let mut number = self.start.get(frame)?;
        let mut beyond_32_bits = false;
        for link in &self.links {
            let other = link.leaf.get(frame)?;
            let (answer, noted) = link.step.apply(number, other)?;
            beyond_32_bits |= noted && i32::try_from(answer).is_err();
            if answer < link.lowest || answer > link.highest {
                return None;
            }
            number = answer;
        }
        if beyond_32_bits {
            frame.note_beyond_32_bits();
        }
        Some(number)
    }
}

#[cfg(test)]
mod tests {
    use super::Divisor;

    #[test]
    fn a_divisor_gives_the_quotient_and_remainder_of_every_whole_number_tried() {
        let edges = [
            0_u64,
            1,
            2,
            3,
            7,
            99,
            100,
            101,
            u64::from(u32::MAX),
            u64::from(u32::MAX) + 1,
            1 << 53,
            (1 << 62) - 1,
            1 << 62,
            (1 << 62) + 1,
            u64::MAX >> 1,
            (u64::MAX >> 1) - 1,
        ];
        let mut divisors: Vec<u64> = (1..=300).collect();
        divisors.extend(edges.iter().copied().filter(|divisor| *divisor >= 1));
        divisors.extend((1..63).flat_map(|bits| {
            let power = 1_u64 << bits;
            [power - 1, power, power + 1]
        }));
        let mut seed = 0x2545_F491_4F6C_DD1D_u64;
        for divisor in divisors {
            let found = Divisor::of(i64::try_from(divisor).expect("a divisor below 2^63"))
                .expect("every divisor from 1 has a form");
            let mut numbers: Vec<u64> = edges.to_vec();
            for _ in 0..64 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                numbers.push(seed >> 1);
                numbers.push((seed >> 40) % 10_000);
            }
            let most = (u64::MAX >> 1) / divisor;
            for times in [1, 2, 3, most / 2, most - 1, most] {
                let multiple = divisor.saturating_mul(times);
                numbers.extend([
                    multiple.saturating_sub(1),
                    multiple,
                    multiple.saturating_add(1),
                ]);
            }
            for number in numbers
                .into_iter()
                .filter(|number| *number <= u64::MAX >> 1)
            {
                assert_eq!(
                    found.quotient(number),
                    number / divisor,
                    "{number} / {divisor}"
                );
                assert_eq!(
                    found.remainder(number),
                    number % divisor,
                    "{number} % {divisor}"
                );
            }
        }
    }

    #[test]
    fn a_divisor_below_one_has_no_form() {
        assert!(Divisor::of(0).is_none(), "zero");
        assert!(Divisor::of(-3).is_none(), "below zero");
    }
}
