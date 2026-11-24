use super::Code;
use super::num::Bound;
use crate::eval::{arithmetic, compare};
use crate::{Expr, Frame, Math, Op, Order};
use earnest_value::Value;

/// Where a packed row keeps the code of one field, and what the code stands for, as the caller
/// that packs the rows lays them out.
///
/// The code sits in `width` bits of word `word`, starting `shift` bits from the lowest bit. A
/// row handed to a [`PackedTest`] or a [`PackedUpdate`] must hold in every slot a code that
/// stands for a value of the field, as [`Coding`] says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The word of the row that holds the code.
    pub word: usize,
    /// How many bits below the code the word holds.
    pub shift: u32,
    /// How many bits the code takes.
    pub width: u32,
    /// What each code stands for.
    pub coding: Coding,
}

/// What the codes of a [`Slot`] stand for.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Coding {
    /// The whole numbers from `lowest` to `highest`: code `c` is `lowest + c`.
    Whole {
        /// The number of code 0.
        lowest: i64,
        /// The largest number a code stands for.
        highest: i64,
    },
    /// False is code 0 and true is code 1.
    Truth,
    /// Code `c` is the value at position `c` of the list, and each value is listed once.
    Listed(Vec<Value>),
}

impl Coding {
    fn code(&self, value: &Value) -> Option<u64> {
        match (self, value) {
            (Self::Whole { lowest, highest }, Value::Int(number)) => {
                Whole::of(*lowest, *highest)?.code(*number)
            }
            (Self::Truth, Value::Bool(flag)) => Some(u64::from(*flag)),
            (Self::Listed(values), found) => {
                u64::try_from(values.iter().position(|value| value == found)?).ok()
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Bits {
    word: usize,
    shift: u32,
    mask: u64,
    placed: u64,
}

impl Bits {
    fn of(slot: &Slot) -> Option<Self> {
        if slot.width == 0 {
            return Some(Self {
                word: slot.word,
                shift: 0,
                mask: 0,
                placed: 0,
            });
        }
        if slot.width > u64::BITS || slot.shift >= u64::BITS {
            return None;
        }
        let mask = u64::MAX.checked_shr(u64::BITS - slot.width)?;
        if slot.shift.checked_add(slot.width)? > u64::BITS {
            return None;
        }
        Some(Self {
            word: slot.word,
            shift: slot.shift,
            mask,
            placed: mask.checked_shl(slot.shift)?,
        })
    }

    #[inline]
    fn read(self, row: &[u64]) -> Option<u64> {
        row.get(self.word)
            .map(|word| word.wrapping_shr(self.shift) & self.mask)
    }

    #[inline]
    fn write(self, next: &mut [u64], code: u64) -> bool {
        let Some(word) = next.get_mut(self.word) else {
            return false;
        };
        *word = (*word & !self.placed) | (code.wrapping_shl(self.shift) & self.placed);
        true
    }
}

struct Fields<'a> {
    slots: &'a [Option<Slot>],
}

const MOST_TABLED_CODES: u64 = 1 << 12;

const MOST_SWEPT_CONSTANTS: usize = 16;

#[derive(Default)]
struct Reads {
    fields: Vec<usize>,
    slots: usize,
    sweeps: usize,
}

fn small_constant(source: &Expr) -> bool {
    match source {
        Expr::Const(Value::Seq(items)) => items.len() <= MOST_SWEPT_CONSTANTS,
        Expr::Const(Value::Set(items)) => items.len() <= MOST_SWEPT_CONSTANTS,
        _ => false,
    }
}

impl Reads {
    fn of(expr: &Expr, bound: &mut Vec<usize>, reads: &mut Self) -> Option<()> {
        match expr {
            Expr::Field(index) => {
                if !reads.fields.contains(index) {
                    reads.fields.push(*index);
                }
            }
            Expr::Local(slot) => {
                bound.contains(slot).then_some(())?;
            }
            Expr::Bind { slot, value, body } => {
                Self::of(value, bound, reads)?;
                reads.slots = reads.slots.max(slot.checked_add(1)?);
                bound.push(*slot);
                let body = Self::of(body, bound, reads);
                bound.pop();
                body?;
            }
            Expr::Sweep {
                slot, source, body, ..
            } => {
                let mut swept = Self::default();
                Self::of(source, bound, &mut swept)?;
                let mut inside = Self::default();
                bound.push(*slot);
                let body = Self::of(body, bound, &mut inside);
                bound.pop();
                body?;
                let bounded =
                    !swept.fields.is_empty() || small_constant(source) && inside.sweeps == 0;
                bounded.then_some(())?;
                for field in swept.fields.into_iter().chain(inside.fields) {
                    if !reads.fields.contains(&field) {
                        reads.fields.push(field);
                    }
                }
                reads.slots = reads
                    .slots
                    .max(swept.slots)
                    .max(inside.slots)
                    .max(slot.checked_add(1)?);
                reads.sweeps = reads.sweeps + swept.sweeps + inside.sweeps + 1;
            }
            _ => {
                let mut fine = true;
                expr.each_part(&mut |part| {
                    fine = fine && Self::of(part, bound, reads).is_some();
                });
                fine.then_some(())?;
            }
        }
        Some(())
    }
}

impl Fields<'_> {
    fn slot(&self, expr: &Expr) -> Option<(Bits, &Coding)> {
        let Expr::Field(index) = expr.plain() else {
            return None;
        };
        self.slot_at(*index)
    }

    fn slot_at(&self, index: usize) -> Option<(Bits, &Coding)> {
        let slot = self.slots.get(index)?.as_ref()?;
        Some((Bits::of(slot)?, &slot.coding))
    }

    fn each_value<T>(
        &self,
        expr: &Expr,
        mut answer: impl FnMut(&Code, &mut Frame<'_>) -> Option<T>,
    ) -> Option<(Bits, Vec<Option<T>>)> {
        let mut reads = Reads::default();
        Reads::of(expr, &mut Vec::new(), &mut reads)?;
        let [field] = reads.fields.as_slice() else {
            return None;
        };
        let (bits, coding) = self.slot_at(*field)?;
        let values = match coding {
            Coding::Whole { lowest, highest } => (highest.abs_diff(*lowest) < MOST_TABLED_CODES)
                .then(|| (*lowest..=*highest).map(Value::Int).collect::<Vec<_>>())?,
            Coding::Truth => vec![Value::Bool(false), Value::Bool(true)],
            Coding::Listed(values) => values.clone(),
        };
        if u64::try_from(values.len()).ok()? > MOST_TABLED_CODES {
            return None;
        }
        let code = Code::new(expr.clone());
        let mut state = vec![Value::Null; field.checked_add(1)?];
        let answers = values
            .into_iter()
            .map(|value| {
                if let Some(place) = state.get_mut(*field) {
                    *place = value;
                }
                let mut frame = Frame::new(&state, reads.slots);
                let found = answer(&code, &mut frame);
                found.filter(|_| !frame.computed_beyond_32_bits())
            })
            .collect();
        Some((bits, answers))
    }

    fn whole(&self, expr: &Expr) -> Option<(Bits, Whole)> {
        match self.slot(expr)? {
            (bits, Coding::Whole { lowest, highest }) => {
                Some((bits, Whole::of(*lowest, *highest)?))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Whole {
    lowest: i64,
    highest: i64,
    top: u64,
}

impl Whole {
    fn of(lowest: i64, highest: i64) -> Option<Self> {
        (lowest <= highest).then(|| Self {
            lowest,
            highest,
            top: highest.abs_diff(lowest),
        })
    }

    fn code(self, number: i64) -> Option<u64> {
        (self.lowest <= number && number <= self.highest).then(|| number.abs_diff(self.lowest))
    }

    fn within_32_bits(self) -> bool {
        i32::try_from(self.lowest).is_ok() && i32::try_from(self.highest).is_ok()
    }

    fn within(self, bound: Bound) -> bool {
        bound.check(self.lowest).is_ok() && bound.check(self.highest).is_ok()
    }
}

#[derive(Clone, Debug)]
#[repr(u8)]
enum Term {
    Int(i64),
    Field { bits: Bits, lowest: i64 },
    Math(Box<Arith>),
    Bounded(Bound, Box<Self>),
}

#[derive(Clone, Debug)]
struct Arith {
    math: Math,
    left: Term,
    right: Term,
}

impl Term {
    fn of(expr: &Expr, fields: &Fields<'_>) -> Option<Self> {
        match expr.plain() {
            Expr::Const(Value::Int(number)) => Some(Self::Int(*number)),
            Expr::Field(_) => {
                let (bits, whole) = fields.whole(expr)?;
                Some(Self::Field {
                    bits,
                    lowest: whole.lowest,
                })
            }
            Expr::Apply(Op::Math(math), arguments) => match arguments.as_slice() {
                [left, right] => Some(Self::Math(Box::new(Arith {
                    math: *math,
                    left: Self::of(left, fields)?,
                    right: Self::of(right, fields)?,
                }))),
                _ => None,
            },
            Expr::Apply(Op::Safe, arguments) => Self::bounded(Bound::Safe, arguments, fields),
            Expr::Apply(Op::Fit(ty), arguments) => {
                Self::bounded(Bound::Fit(*ty), arguments, fields)
            }
            _ => None,
        }
    }

    fn bounded(bound: Bound, arguments: &[Expr], fields: &Fields<'_>) -> Option<Self> {
        let [inner] = arguments else {
            return None;
        };
        Some(Self::Bounded(bound, Box::new(Self::of(inner, fields)?)))
    }

    fn sure(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Field { .. })
    }

    #[inline]
    fn get(&self, row: &[u64]) -> Option<i64> {
        match self {
            Self::Int(number) => Some(*number),
            Self::Field { bits, lowest } => Some(lowest.wrapping_add_unsigned(bits.read(row)?)),
            Self::Math(arith) => arith.get(row),
            Self::Bounded(bound, inner) => bound.check(inner.get(row)?).ok(),
        }
    }
}

impl Arith {
    fn get(&self, row: &[u64]) -> Option<i64> {
        let left = self.left.get(row)?;
        let right = self.right.get(row)?;
        let answer = arithmetic(self.math, left, right).ok()?;
        i32::try_from(answer).ok().map(i64::from)
    }
}

#[derive(Clone, Copy, Debug)]
enum Sense {
    Kept,
    Turned,
}

impl Sense {
    #[inline]
    fn says<T: PartialEq + Copy>(self, found: T, wanted: T) -> bool {
        match self {
            Self::Kept => found == wanted,
            Self::Turned => found != wanted,
        }
    }
}

#[derive(Clone, Debug)]
#[repr(u8)]
enum Test {
    Const(bool),
    Within {
        bits: Bits,
        low: u64,
        span: u64,
        sense: Sense,
    },
    Among {
        bits: Bits,
        set: u64,
    },
    Order {
        order: Order,
        left: Term,
        right: Term,
    },
    Equal {
        left: Term,
        right: Term,
        sense: Sense,
    },
    Same {
        first: Bits,
        second: Bits,
        sense: Sense,
    },
    Table {
        bits: Bits,
        truth: Box<[u64]>,
        known: Box<[u64]>,
    },
    All(Box<[Self]>),
    Any(Box<[Self]>),
    Not(Box<Self>),
    Choose(Box<(Self, Self, Self)>),
}

fn flipped(order: Order) -> Order {
    match order {
        Order::Less => Order::Greater,
        Order::LessOrEqual => Order::GreaterOrEqual,
        Order::Greater => Order::Less,
        Order::GreaterOrEqual => Order::LessOrEqual,
    }
}

impl Test {
    fn of(expr: &Expr, fields: &Fields<'_>) -> Option<Self> {
        Self::shaped(expr, fields).or_else(|| Self::tabled(expr, fields))
    }

    fn tabled(expr: &Expr, fields: &Fields<'_>) -> Option<Self> {
        let (bits, answers) = fields.each_value(expr, |code, frame| code.truth(frame).ok())?;
        answers.iter().any(Option::is_some).then_some(())?;
        let words = answers.len().div_ceil(64);
        let mut truth = vec![0_u64; words];
        let mut known = vec![0_u64; words];
        for (code, answer) in answers.iter().enumerate() {
            let (word, bit) = (code / 64, 1_u64 << (code % 64));
            if let (Some(flag), Some(truths), Some(knowns)) =
                (answer, truth.get_mut(word), known.get_mut(word))
            {
                *knowns |= bit;
                if *flag {
                    *truths |= bit;
                }
            }
        }
        if let ([set], [all]) = (truth.as_slice(), known.as_slice())
            && answers.iter().all(Option::is_some)
        {
            let (set, all) = (*set, *all);
            return Some(match set {
                0 => Self::Const(false),
                _ if set == all => Self::Const(true),
                _ => Self::Among { bits, set },
            });
        }
        Some(Self::Table {
            bits,
            truth: truth.into_boxed_slice(),
            known: known.into_boxed_slice(),
        })
    }

    fn shaped(expr: &Expr, fields: &Fields<'_>) -> Option<Self> {
        match expr.plain() {
            Expr::Const(Value::Bool(flag)) => Some(Self::Const(*flag)),
            Expr::Field(_) => Self::among(expr, fields, |value| match value {
                Value::Bool(flag) => Some(*flag),
                _ => None,
            }),
            Expr::And(..) => Some(Self::joined(expr, Self::All, fields)?),
            Expr::Or(..) => Some(Self::joined(expr, Self::Any, fields)?),
            Expr::Choose(test, yes, no) => Some(
                match (
                    Self::of(test, fields)?,
                    Self::of(yes, fields)?,
                    Self::of(no, fields)?,
                ) {
                    (Self::Const(true), chosen, _) | (Self::Const(false), _, chosen) => chosen,
                    branches => Self::Choose(Box::new(branches)),
                },
            ),
            Expr::Apply(op, arguments) => match (op, arguments.as_slice()) {
                (Op::Not, [inner]) => Some(match Self::of(inner, fields)? {
                    Self::Const(flag) => Self::Const(!flag),
                    other => Self::Not(Box::new(other)),
                }),
                (Op::Order(order), [left, right]) => Self::order(*order, left, right, fields),
                (Op::Equal, [left, right]) => Self::equality(left, right, Sense::Kept, fields),
                (Op::Differ, [left, right]) => Self::equality(left, right, Sense::Turned, fields),
                _ => None,
            },
            _ => None,
        }
    }

    fn joined(expr: &Expr, join: fn(Box<[Self]>) -> Self, fields: &Fields<'_>) -> Option<Self> {
        let decides = matches!(expr.plain(), Expr::Or(..));
        let mut parts = Vec::new();
        let mut pending = vec![expr.plain()];
        while let Some(part) = pending.pop() {
            match (expr.plain(), part) {
                (Expr::And(..), Expr::And(left, right)) | (Expr::Or(..), Expr::Or(left, right)) => {
                    pending.push(right.plain());
                    pending.push(left.plain());
                }
                _ => match Self::of(part, fields)? {
                    Self::Const(flag) if flag == decides => {
                        if parts.iter().all(Self::sure) {
                            return Some(Self::Const(flag));
                        }
                        parts.push(Self::Const(flag));
                        break;
                    }
                    Self::Const(_) => {}
                    other => parts.push(other),
                },
            }
        }
        Some(match parts.len() {
            0 => Self::Const(!decides),
            1 => parts.pop()?,
            _ => join(parts.into_boxed_slice()),
        })
    }

    fn sure(&self) -> bool {
        match self {
            Self::Const(_) | Self::Within { .. } | Self::Same { .. } => true,
            Self::Table { .. } => false,
            Self::Among { bits, .. } => bits.mask < u64::from(u64::BITS),
            Self::Order { left, right, .. } | Self::Equal { left, right, .. } => {
                left.sure() && right.sure()
            }
            Self::All(parts) | Self::Any(parts) => parts.iter().all(Self::sure),
            Self::Not(inner) => inner.sure(),
            Self::Choose(branches) => branches.0.sure() && branches.1.sure() && branches.2.sure(),
        }
    }

    fn among(
        expr: &Expr,
        fields: &Fields<'_>,
        truth: impl Fn(&Value) -> Option<bool>,
    ) -> Option<Self> {
        let (bits, coding) = fields.slot(expr)?;
        let values: &[Value] = match coding {
            Coding::Truth => &[Value::Bool(false), Value::Bool(true)],
            Coding::Listed(values) => values,
            Coding::Whole { .. } => return None,
        };
        let mut set = 0_u64;
        for (code, value) in values.iter().enumerate() {
            if truth(value)? {
                set |= 1_u64.checked_shl(u32::try_from(code).ok()?)?;
            }
        }
        let all = match u32::try_from(values.len()).ok()? {
            count if count < u64::BITS => Some((1_u64 << count) - 1),
            u64::BITS => Some(u64::MAX),
            _ => None,
        };
        Some(match set {
            0 => Self::Const(false),
            _ if Some(set) == all => Self::Const(true),
            _ if set.is_power_of_two() => Self::Within {
                bits,
                low: u64::from(set.trailing_zeros()),
                span: 0,
                sense: Sense::Kept,
            },
            _ => Self::Among { bits, set },
        })
    }

    fn order(order: Order, left: &Expr, right: &Expr, fields: &Fields<'_>) -> Option<Self> {
        match (left.plain(), right.plain()) {
            (Expr::Field(_), Expr::Const(Value::Int(number))) => {
                Self::range(order, left, *number, fields)
            }
            (Expr::Const(Value::Int(number)), Expr::Field(_)) => {
                Self::range(flipped(order), right, *number, fields)
            }
            _ => Some(Self::Order {
                order,
                left: Term::of(left, fields)?,
                right: Term::of(right, fields)?,
            }),
        }
    }

    fn range(order: Order, field: &Expr, number: i64, fields: &Fields<'_>) -> Option<Self> {
        let wide = i128::from(number);
        let (lowest, highest) = match order {
            Order::Less => (i128::MIN, wide - 1),
            Order::LessOrEqual => (i128::MIN, wide),
            Order::Greater => (wide + 1, i128::MAX),
            Order::GreaterOrEqual => (wide, i128::MAX),
        };
        Self::within(field, lowest, highest, Sense::Kept, fields)
    }

    fn within(
        field: &Expr,
        lowest: i128,
        highest: i128,
        sense: Sense,
        fields: &Fields<'_>,
    ) -> Option<Self> {
        let (bits, whole) = fields.whole(field)?;
        let from = lowest.max(i128::from(whole.lowest));
        let to = highest.min(i128::from(whole.highest));
        if from > to {
            return Some(Self::Const(sense.says(false, true)));
        }
        if from == i128::from(whole.lowest) && to == i128::from(whole.highest) {
            return Some(Self::Const(sense.says(true, true)));
        }
        Some(Self::Within {
            bits,
            low: u64::try_from(from - i128::from(whole.lowest)).ok()?,
            span: u64::try_from(to - from).ok()?,
            sense,
        })
    }

    fn equality(left: &Expr, right: &Expr, sense: Sense, fields: &Fields<'_>) -> Option<Self> {
        match (left.plain(), right.plain()) {
            (Expr::Field(_), Expr::Const(constant)) => Self::matches(left, constant, sense, fields),
            (Expr::Const(constant), Expr::Field(_)) => {
                Self::matches(right, constant, sense, fields)
            }
            (Expr::Field(_), Expr::Field(_)) => match (fields.slot(left)?, fields.slot(right)?) {
                ((_, Coding::Whole { .. }), (_, Coding::Whole { .. })) => {
                    Self::terms(left, right, sense, fields)
                }
                ((first, one), (second, other)) if one == other => Some(Self::Same {
                    first,
                    second,
                    sense,
                }),
                _ => None,
            },
            _ => Self::terms(left, right, sense, fields),
        }
    }

    fn terms(left: &Expr, right: &Expr, sense: Sense, fields: &Fields<'_>) -> Option<Self> {
        Some(Self::Equal {
            left: Term::of(left, fields)?,
            right: Term::of(right, fields)?,
            sense,
        })
    }

    fn matches(field: &Expr, constant: &Value, sense: Sense, fields: &Fields<'_>) -> Option<Self> {
        match (fields.slot(field)?.1, constant) {
            (Coding::Whole { .. }, Value::Int(number)) => {
                let wide = i128::from(*number);
                Self::within(field, wide, wide, sense, fields)
            }
            (Coding::Whole { .. }, _) => Some(Self::Const(sense.says(false, true))),
            (Coding::Truth | Coding::Listed(_), _) => {
                Self::among(field, fields, |value| Some(sense.says(value, constant)))
            }
        }
    }

    #[inline]
    fn holds(&self, row: &[u64]) -> Option<bool> {
        match self {
            Self::Within {
                bits,
                low,
                span,
                sense,
            } => Some(sense.says(bits.read(row)?.wrapping_sub(*low) <= *span, true)),
            Self::Const(flag) => Some(*flag),
            _ => self.rest(row),
        }
    }

    #[inline(never)]
    fn rest(&self, row: &[u64]) -> Option<bool> {
        match self {
            Self::Const(_) | Self::Within { .. } => self.holds(row),
            Self::Among { bits, set } => {
                Some(set.checked_shr(u32::try_from(bits.read(row)?).ok()?)? & 1 == 1)
            }
            Self::Order { order, left, right } => {
                Some(compare(*order, left.get(row)?, right.get(row)?))
            }
            Self::Equal { left, right, sense } => Some(sense.says(left.get(row)?, right.get(row)?)),
            Self::Same {
                first,
                second,
                sense,
            } => Some(sense.says(first.read(row)?, second.read(row)?)),
            Self::Table { bits, truth, known } => {
                let code = usize::try_from(bits.read(row)?).ok()?;
                let (word, bit) = (code / 64, 1_u64 << (code % 64));
                (known.get(word)? & bit != 0)
                    .then(|| truth.get(word).is_some_and(|set| set & bit != 0))
            }
            Self::All(parts) => {
                for part in parts {
                    if !part.holds(row)? {
                        return Some(false);
                    }
                }
                Some(true)
            }
            Self::Any(parts) => {
                for part in parts {
                    if part.holds(row)? {
                        return Some(true);
                    }
                }
                Some(false)
            }
            Self::Not(inner) => Some(!inner.holds(row)?),
            Self::Choose(branches) => {
                if branches.0.holds(row)? {
                    branches.1.holds(row)
                } else {
                    branches.2.holds(row)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
#[repr(u8)]
enum Put {
    Code(u64),
    Outside,
    Moved { from: Bits, delta: u64, top: u64 },
    Number { term: Term, whole: Whole },
    Truth(Test),
    Copy(Bits),
    Table { from: Bits, codes: Box<[u64]> },
    Choose(Box<(Test, Self, Self)>),
}

const NO_CODE: u64 = u64::MAX;

const MOST_MOVED_CODE: i128 = 1 << 62;

impl Put {
    fn of(expr: &Expr, into: &Coding, fields: &Fields<'_>) -> Option<Self> {
        Self::shaped(expr, into, fields)
            .or_else(|| Self::tabled(expr, into, fields))
            .or_else(|| Self::constant(expr, into))
    }

    fn constant(expr: &Expr, into: &Coding) -> Option<Self> {
        let mut reads = Reads::default();
        Reads::of(expr, &mut Vec::new(), &mut reads)?;
        if !reads.fields.is_empty() || reads.sweeps > 0 {
            return None;
        }
        let mut frame = Frame::new(&[], reads.slots);
        let value = Code::new(expr.clone()).eval(&mut frame);
        if frame.computed_beyond_32_bits() {
            return None;
        }
        Some(into.code(&value.ok()?).map_or(Self::Outside, Self::Code))
    }

    fn tabled(expr: &Expr, into: &Coding, fields: &Fields<'_>) -> Option<Self> {
        let (from, answers) =
            fields.each_value(expr, |code, frame| into.code(&code.eval(frame).ok()?))?;
        answers.iter().any(Option::is_some).then_some(())?;
        Some(Self::Table {
            from,
            codes: answers
                .into_iter()
                .map(|answer| answer.unwrap_or(NO_CODE))
                .collect(),
        })
    }

    fn shaped(expr: &Expr, into: &Coding, fields: &Fields<'_>) -> Option<Self> {
        let expr = expr.plain();
        if let Expr::Choose(test, yes, no) = expr {
            return Some(Self::Choose(Box::new((
                Test::of(test, fields)?,
                Self::of(yes, into, fields)?,
                Self::of(no, into, fields)?,
            ))));
        }
        if let (Expr::Field(_), Some((from, coding))) = (expr, fields.slot(expr))
            && coding == into
            && !matches!(into, Coding::Whole { .. })
        {
            return Some(Self::Copy(from));
        }
        match into {
            Coding::Whole { lowest, highest } => {
                let whole = Whole::of(*lowest, *highest)?;
                match expr {
                    Expr::Const(Value::Int(number)) => {
                        Some(whole.code(*number).map_or(Self::Outside, Self::Code))
                    }
                    _ => Self::moved(expr, whole, fields).or_else(|| {
                        Some(Self::Number {
                            term: Term::of(expr, fields)?,
                            whole,
                        })
                    }),
                }
            }
            Coding::Truth => match expr {
                Expr::Const(Value::Bool(flag)) => Some(Self::Code(u64::from(*flag))),
                _ => Some(Self::Truth(Test::of(expr, fields)?)),
            },
            Coding::Listed(_) => match expr {
                Expr::Const(constant) => {
                    Some(into.code(constant).map_or(Self::Outside, Self::Code))
                }
                _ => None,
            },
        }
    }

    fn moved(expr: &Expr, into: Whole, fields: &Fields<'_>) -> Option<Self> {
        let (field, change) = match expr {
            Expr::Field(_) => (expr, 0),
            Expr::Apply(Op::Safe, arguments) if into.within(Bound::Safe) => {
                return Self::moved(single(arguments)?.plain(), into, fields);
            }
            Expr::Apply(Op::Fit(ty), arguments) if into.within(Bound::Fit(*ty)) => {
                return Self::moved(single(arguments)?.plain(), into, fields);
            }
            Expr::Apply(Op::Math(Math::Add), arguments) => match arguments.as_slice() {
                [field, Expr::Const(Value::Int(number))]
                | [Expr::Const(Value::Int(number)), field] => (field.plain(), i128::from(*number)),
                _ => return None,
            },
            Expr::Apply(Op::Math(Math::Subtract), arguments) => match arguments.as_slice() {
                [field, Expr::Const(Value::Int(number))] => (field.plain(), -i128::from(*number)),
                _ => return None,
            },
            _ => return None,
        };
        let (from, whole) = fields.whole(field)?;
        let delta = i128::from(whole.lowest) + change - i128::from(into.lowest);
        let reach = delta + i128::from(whole.top);
        let fits = into.within_32_bits()
            && delta > -MOST_MOVED_CODE
            && reach < MOST_MOVED_CODE
            && i128::from(into.top) < MOST_MOVED_CODE;
        if !fits {
            return None;
        }
        Some(Self::Moved {
            from,
            delta: i64::try_from(delta).ok()?.cast_unsigned(),
            top: into.top,
        })
    }

    #[inline]
    fn code(&self, row: &[u64]) -> Option<u64> {
        match self {
            Self::Moved { from, delta, top } => {
                let code = from.read(row)?.wrapping_add(*delta);
                (code <= *top).then_some(code)
            }
            Self::Code(code) => Some(*code),
            _ => self.rest(row),
        }
    }

    #[inline(never)]
    fn rest(&self, row: &[u64]) -> Option<u64> {
        match self {
            Self::Moved { .. } | Self::Code(_) => self.code(row),
            Self::Outside => None,
            Self::Number { term, whole } => whole.code(term.get(row)?),
            Self::Truth(test) => test.holds(row).map(u64::from),
            Self::Copy(from) => from.read(row),
            Self::Table { from, codes } => {
                let code = *codes.get(usize::try_from(from.read(row)?).ok()?)?;
                (code != NO_CODE).then_some(code)
            }
            Self::Choose(branches) => {
                if branches.0.holds(row)? {
                    branches.1.code(row)
                } else {
                    branches.2.code(row)
                }
            }
        }
    }
}

fn single(arguments: &[Expr]) -> Option<&Expr> {
    match arguments {
        [inner] => Some(inner),
        _ => None,
    }
}

/// A guard, a rule or an end prepared to be judged straight from a packed row, from
/// [`super::Code::packed_test`].
#[derive(Clone, Debug)]
pub struct PackedTest {
    test: Test,
}

fn given(expr: &Expr, inputs: &[(usize, Value)]) -> Expr {
    let each = |all: &[Expr]| all.iter().map(|part| given(part, inputs)).collect();
    let named = |all: &[(earnest_value::Sym, Expr)]| {
        all.iter()
            .map(|(name, part)| (*name, given(part, inputs)))
            .collect()
    };
    let boxed = |part: &Expr| Box::new(given(part, inputs));
    match expr {
        Expr::Local(slot) => inputs
            .iter()
            .find(|(input, _)| input == slot)
            .map_or_else(|| expr.clone(), |(_, value)| Expr::Const(value.clone())),
        Expr::Const(_) | Expr::Field(_) => expr.clone(),
        Expr::Apply(op, arguments) => Expr::Apply(*op, each(arguments)),
        Expr::And(left, right) => Expr::And(boxed(left), boxed(right)),
        Expr::Or(left, right) => Expr::Or(boxed(left), boxed(right)),
        Expr::Choose(test, yes, no) => Expr::Choose(boxed(test), boxed(yes), boxed(no)),
        Expr::Bind { slot, value, body } => Expr::Bind {
            slot: *slot,
            value: boxed(value),
            body: boxed(body),
        },
        Expr::Record(fields) => Expr::Record(named(fields)),
        Expr::Get(inner, name) => Expr::Get(boxed(inner), *name),
        Expr::With(base, changes) => Expr::With(boxed(base), named(changes)),
        Expr::SetOf(items) => Expr::SetOf(each(items)),
        Expr::SeqOf(items) => Expr::SeqOf(each(items)),
        Expr::MapOf(entries) => Expr::MapOf(
            entries
                .iter()
                .map(|(key, value)| (given(key, inputs), given(value, inputs)))
                .collect(),
        ),
        Expr::Sweep {
            sweep,
            shelf,
            slot,
            source,
            body,
        } => Expr::Sweep {
            sweep: *sweep,
            shelf: *shelf,
            slot: *slot,
            source: boxed(source),
            body: boxed(body),
        },
    }
}

fn with_inputs<'e>(expr: &'e Expr, inputs: &[(usize, Value)]) -> std::borrow::Cow<'e, Expr> {
    if inputs.is_empty() {
        std::borrow::Cow::Borrowed(expr)
    } else {
        std::borrow::Cow::Owned(given(expr, inputs))
    }
}

impl PackedTest {
    pub(super) fn of(
        expr: &Expr,
        slots: &[Option<Slot>],
        inputs: &[(usize, Value)],
    ) -> Option<Self> {
        Test::of(&with_inputs(expr, inputs), &Fields { slots }).map(|test| Self { test })
    }

    /// Whether the expression is true in the state packed in `row`, laid out as the slots the
    /// test was prepared with say.
    ///
    /// `Some` is exactly what [`super::Code::truth`] gives for that state, and that call would
    /// give no fault and note no number beyond 32 bits. `None` when the row alone cannot say so
    /// without doubt: then the caller asks [`super::Code::truth`] in a frame of the state's
    /// values.
    #[inline]
    #[must_use]
    pub fn holds(&self, row: &[u64]) -> Option<bool> {
        self.test.holds(row)
    }
}

/// An update of one field prepared to be written straight from one packed row into another,
/// from [`super::Code::packed_update`].
#[derive(Clone, Debug)]
pub struct PackedUpdate {
    into: Bits,
    put: Put,
}

impl PackedUpdate {
    pub(super) fn of(
        expr: &Expr,
        slots: &[Option<Slot>],
        field: usize,
        inputs: &[(usize, Value)],
    ) -> Option<Self> {
        let slot = slots.get(field)?.as_ref()?;
        let put = Put::of(&with_inputs(expr, inputs), &slot.coding, &Fields { slots })?;
        (!matches!(put, Put::Outside)).then_some(Self {
            into: Bits::of(slot)?,
            put,
        })
    }

    /// The word of a row of one word, `word`, with the code of the update's answer in the state
    /// packed in `row` written into the field's slot, when [`PackedUpdate::write`] would write
    /// it. `None` when it would not, and when the field's slot is not in the first word.
    #[inline]
    #[must_use]
    pub fn write_word(&self, row: &[u64], word: u64) -> Option<u64> {
        let code = self.put.code(row)?;
        (self.into.word == 0).then(|| {
            (word & !self.into.placed) | (code.wrapping_shl(self.into.shift) & self.into.placed)
        })
    }

    /// Writes the code of the update's answer in the state packed in `row` into the field's
    /// slot of `next`, and gives whether it did.
    ///
    /// `true` when [`super::Code::eval`] gives that answer for that state with no fault and no
    /// note of a number beyond 32 bits, and the answer is a value of the field. `false` when
    /// the row alone cannot say so without doubt; `next` may then have been changed, and the
    /// caller asks [`super::Code::eval`] in a frame of the state's values.
    #[inline]
    pub fn write(&self, row: &[u64], next: &mut [u64]) -> bool {
        match self.put.code(row) {
            Some(code) => self.into.write(next, code),
            None => false,
        }
    }
}
