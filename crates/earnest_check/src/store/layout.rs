use earnest_value::Domain;

pub(super) const WORD_BITS: u32 = u64::BITS;
pub(super) const WORD_BYTES: usize = 8;
const SEAT_SPREAD: u64 = 0x9E37_79B9_7F4A_7C15;
const MIX: u64 = 0xD6E8_FEB8_6659_FD93;
const HALF_WORD: u32 = 32;

#[derive(Debug)]
pub(crate) struct RowLayout {
    places: Vec<Place>,
    numbers: Vec<Option<RangeSlot>>,
    truths: Vec<Option<RangeSlot>>,
    words: usize,
    stride: usize,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Place {
    pub(super) word: usize,
    pub(super) shift: u32,
    pub(super) mask: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RangeSlot {
    pub(crate) word: usize,
    pub(crate) shift: u32,
    pub(crate) width: u32,
    pub(crate) offset: i64,
    highest: i64,
}

impl RangeSlot {
    #[must_use]
    pub(crate) fn code(&self, value: i64) -> Option<u64> {
        (self.offset <= value && value <= self.highest).then(|| value.abs_diff(self.offset))
    }

    pub(crate) fn write(&self, row: &mut [u64], value: i64) -> bool {
        let (Some(code), Some(word)) = (self.code(value), row.get_mut(self.word)) else {
            return false;
        };
        let placed = mask_of(self.width).checked_shl(self.shift).unwrap_or(0);
        *word = (*word & !placed) | (code.checked_shl(self.shift).unwrap_or(0) & placed);
        true
    }
}

pub(super) fn mask_of(bits: u32) -> u64 {
    if bits >= WORD_BITS {
        u64::MAX
    } else {
        (1_u64 << bits) - 1
    }
}

fn mix(word: u64, at: usize) -> u64 {
    let seat = u64::try_from(at).unwrap_or(0).wrapping_mul(SEAT_SPREAD);
    let mut mixed = word ^ seat;
    mixed ^= mixed >> HALF_WORD;
    mixed = mixed.wrapping_mul(MIX);
    mixed ^= mixed >> HALF_WORD;
    mixed = mixed.wrapping_mul(MIX);
    mixed ^ (mixed >> HALF_WORD)
}

pub(crate) fn hash_words(row: &[u64]) -> u64 {
    hash_each(row.iter().copied())
}

pub(super) fn hash_each(words: impl Iterator<Item = u64>) -> u64 {
    words
        .enumerate()
        .fold(0, |hash, (at, word)| hash ^ mix(word, at))
}

impl RowLayout {
    pub(super) fn of(domains: &[&Domain], widths: &[u32]) -> Self {
        let mut places = Vec::with_capacity(widths.len());
        let mut numbers = Vec::with_capacity(widths.len());
        let mut truths = Vec::with_capacity(widths.len());
        let mut word = 0;
        let mut used = 0;
        for (domain, width) in domains.iter().zip(widths) {
            if used > 0 && used + width > WORD_BITS {
                word += 1;
                used = 0;
            }
            places.push(Place {
                word,
                shift: used,
                mask: mask_of(*width),
            });
            numbers.push(match domain {
                Domain::Int { lo, hi } => Some(RangeSlot {
                    word,
                    shift: used,
                    width: *width,
                    offset: *lo,
                    highest: *hi,
                }),
                _ => None,
            });
            truths.push(matches!(domain, Domain::Bool).then_some(RangeSlot {
                word,
                shift: used,
                width: *width,
                offset: 0,
                highest: 1,
            }));
            used += width;
        }
        let words = if used > 0 { word + 1 } else { word };
        let stride =
            word * WORD_BYTES + usize::try_from(used.div_ceil(u8::BITS)).unwrap_or(WORD_BYTES);
        Self {
            places,
            numbers,
            truths,
            words,
            stride,
        }
    }

    #[must_use]
    pub(crate) fn words(&self) -> usize {
        self.words
    }

    #[must_use]
    pub(crate) fn number(&self, position: usize) -> Option<RangeSlot> {
        self.numbers.get(position).copied().flatten()
    }

    #[must_use]
    pub(crate) fn truth(&self, position: usize) -> Option<RangeSlot> {
        self.truths.get(position).copied().flatten()
    }

    pub(super) fn place(&self, position: usize) -> Option<Place> {
        self.places.get(position).copied()
    }

    pub(super) fn stride(&self) -> usize {
        self.stride
    }
}

impl Place {
    pub(super) fn read(self, row: &[u64]) -> u64 {
        row.get(self.word).map_or(0, |word| {
            word.checked_shr(self.shift).unwrap_or(0) & self.mask
        })
    }

    pub(super) fn write(self, row: &mut [u64], code: u64) {
        if let Some(word) = row.get_mut(self.word) {
            let placed = self.mask.checked_shl(self.shift).unwrap_or(0);
            *word = (*word & !placed) | (code.checked_shl(self.shift).unwrap_or(0) & placed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::codec::Codec;
    use super::RowLayout;
    use earnest_value::Domain;

    fn layout(domains: &[Domain]) -> RowLayout {
        let widths: Vec<u32> = domains
            .iter()
            .map(|domain| Codec::kind_of(domain).1)
            .collect();
        RowLayout::of(&domains.iter().collect::<Vec<_>>(), &widths)
    }

    #[test]
    fn a_number_field_is_written_as_its_distance_from_the_lowest_value() {
        let digits = layout(&[Domain::Int { lo: 0, hi: 9 }, Domain::Int { lo: -3, hi: 3 }]);
        let second = digits.number(1).expect("a number field");
        assert_eq!(
            (second.word, second.shift, second.width),
            (0, 4, 3),
            "after the first"
        );
        let mut row = vec![0; digits.words()];
        assert!(second.write(&mut row, -1), "a value of the domain");
        assert_eq!(row, [2 << 4], "two above the lowest value");
        assert!(!second.write(&mut row, 4), "a value outside the domain");
        assert_eq!(row, [2 << 4], "is not written");
    }

    #[test]
    fn a_field_that_does_not_fit_in_the_word_starts_the_next_one() {
        let wide = layout(&[
            Domain::Int { lo: 0, hi: 1 << 40 },
            Domain::Int { lo: 0, hi: 1 << 40 },
        ]);
        let second = wide.number(1).expect("a number field");
        assert_eq!(
            (second.word, second.shift),
            (1, 0),
            "the start of the second word"
        );
        assert_eq!(wide.words(), 2, "two words");
    }

    #[test]
    fn a_boolean_field_is_a_slot_of_one_bit_that_holds_one_for_true() {
        let mixed = layout(&[Domain::Int { lo: 0, hi: 9 }, Domain::Bool]);
        assert!(mixed.truth(0).is_none(), "a number is no boolean");
        assert!(mixed.number(1).is_none(), "a boolean is no number");
        let flag = mixed.truth(1).expect("a boolean field");
        assert_eq!((flag.word, flag.shift, flag.width), (0, 4, 1));
        let mut row = vec![3; mixed.words()];
        assert!(flag.write(&mut row, 1), "true fits");
        assert_eq!(row, [3 | 1 << 4], "the bit after the number");
        assert!(!flag.write(&mut row, 2), "only 0 and 1");
    }

    #[test]
    fn rows_that_differ_in_one_field_hash_apart() {
        let hashes: std::collections::HashSet<u64> =
            (0..100_u64).map(|row| super::hash_words(&[row])).collect();
        assert_eq!(hashes.len(), 100, "every row its own hash");
    }
}
