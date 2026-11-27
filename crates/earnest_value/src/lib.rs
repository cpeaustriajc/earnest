//! Values, interned names and finite domains.
//!
//! A [`Value`] is what a model's field holds in one state. A [`Domain`] is the finite set of
//! values a field or an input declares. [`Symbols`] holds the text behind every [`Sym`] and
//! prints values as the report shows them. An [`IntType`] is the integer type a number has in
//! a Rust model file, with the numbers it reaches. A [`Big`] is a whole number outside 64 bits,
//! which only a Quint model's arithmetic and literals give.

mod domain;
mod int_type;
mod value;

pub use domain::{Domain, MOST_VALUES_LISTED};
pub use int_type::{IntType, MOST_SAFE_INTEGER};
pub use num_bigint::{BigInt, Sign};
pub use value::{Big, Notation, Quint, Sym, Symbols, Value};
