use super::layout::{WORD_BYTES, hash_each};

#[derive(Debug)]
pub(super) struct Rows {
    bytes: Vec<u8>,
    stride: usize,
}

fn joined<P: Into<u64>>(first: P, last: P, len: usize) -> u64 {
    let apart = len.saturating_sub(size_of::<P>());
    let shift = u32::try_from(apart).unwrap_or(0).saturating_mul(u8::BITS);
    first.into() | last.into().checked_shl(shift).unwrap_or(0)
}

fn word_of(chunk: &[u8]) -> u64 {
    if let Ok(whole) = <[u8; WORD_BYTES]>::try_from(chunk) {
        return u64::from_le_bytes(whole);
    }
    if let (Some(first), Some(last)) = (chunk.first_chunk(), chunk.last_chunk()) {
        return joined(
            u32::from_le_bytes(*first),
            u32::from_le_bytes(*last),
            chunk.len(),
        );
    }
    if let (Some(first), Some(last)) = (chunk.first_chunk(), chunk.last_chunk()) {
        return joined(
            u16::from_le_bytes(*first),
            u16::from_le_bytes(*last),
            chunk.len(),
        );
    }
    chunk.first().map_or(0, |byte| u64::from(*byte))
}

fn put_ends<const PIECE: usize>(bytes: [u8; WORD_BYTES], into: &mut [u8]) -> bool {
    let len = into.len();
    let last = len
        .checked_sub(PIECE)
        .and_then(|start| bytes.get(start..len))
        .and_then(<[u8]>::first_chunk::<PIECE>);
    let (Some(first), Some(last)) = (bytes.first_chunk::<PIECE>(), last) else {
        return false;
    };
    if let Some(start) = into.first_chunk_mut::<PIECE>() {
        *start = *first;
    }
    if let Some(end) = into.last_chunk_mut::<PIECE>() {
        *end = *last;
    }
    true
}

fn put_word(word: u64, into: &mut [u8]) {
    let bytes = word.to_le_bytes();
    if let Ok(whole) = <&mut [u8; WORD_BYTES]>::try_from(&mut *into) {
        *whole = bytes;
        return;
    }
    if put_ends::<4>(bytes, into) || put_ends::<2>(bytes, into) {
        return;
    }
    if let (Some(only), Some(low)) = (into.first_mut(), bytes.first()) {
        *only = *low;
    }
}

pub(super) fn pack(row: &[u64], into: &mut [u8]) {
    for (chunk, word) in into.chunks_mut(WORD_BYTES).zip(row) {
        put_word(*word, chunk);
    }
}

impl Rows {
    pub(super) fn new(stride: usize) -> Self {
        Self {
            bytes: Vec::new(),
            stride,
        }
    }

    pub(super) fn stride(&self) -> usize {
        self.stride
    }

    fn bytes_of(&self, id: usize) -> &[u8] {
        let start = id.saturating_mul(self.stride);
        self.bytes
            .get(start..start.saturating_add(self.stride))
            .unwrap_or(&[])
    }

    pub(super) fn unpack(&self, id: usize, row: &mut Vec<u64>, words: usize) {
        row.clear();
        let bytes = self.bytes_of(id);
        if words == 1 {
            row.push(word_of(bytes));
            return;
        }
        row.extend(bytes.chunks(WORD_BYTES).map(word_of));
        row.resize(words, 0);
    }

    pub(super) fn holds(&self, id: usize, row: &[u64]) -> bool {
        let mut words = self.bytes_of(id).chunks(WORD_BYTES).map(word_of);
        row.iter().all(|word| words.next().unwrap_or(0) == *word) && words.all(|word| word == 0)
    }

    pub(super) fn hash_of(&self, id: usize) -> u64 {
        hash_each(self.bytes_of(id).chunks(WORD_BYTES).map(word_of))
    }

    pub(super) fn push(&mut self, row: &[u64]) {
        let start = self.bytes.len();
        self.bytes.resize(start.saturating_add(self.stride), 0);
        if let Some(into) = self.bytes.get_mut(start..) {
            pack(row, into);
        }
    }

    pub(super) fn grow(&mut self, rows: usize) -> &mut [u8] {
        let start = self.bytes.len();
        self.bytes
            .resize(start.saturating_add(rows.saturating_mul(self.stride)), 0);
        self.bytes.get_mut(start..).unwrap_or_default()
    }

    pub(super) fn truncate(&mut self, kept: usize) {
        self.bytes.truncate(kept.saturating_mul(self.stride));
    }
}

#[derive(Debug)]
pub(super) enum Parents {
    Narrow(Vec<u32>),
    Wide(Vec<usize>),
}

impl Parents {
    pub(super) fn for_up_to(most: usize) -> Self {
        if u32::try_from(most).is_ok_and(|narrow| narrow < u32::MAX) {
            Self::Narrow(Vec::new())
        } else {
            Self::Wide(Vec::new())
        }
    }

    pub(super) fn get(&self, id: usize) -> Option<usize> {
        match self {
            Self::Narrow(all) => all
                .get(id)
                .filter(|parent| **parent != u32::MAX)
                .and_then(|parent| usize::try_from(*parent).ok()),
            Self::Wide(all) => all.get(id).copied().filter(|parent| *parent != usize::MAX),
        }
    }

    pub(super) fn push(&mut self, parent: Option<usize>) {
        match self {
            Self::Narrow(all) => all.push(narrow(parent)),
            Self::Wide(all) => all.push(parent.unwrap_or(usize::MAX)),
        }
    }

    pub(super) fn grow(&mut self, count: usize) -> ParentsTail<'_> {
        match self {
            Self::Narrow(all) => {
                let start = all.len();
                all.resize(start.saturating_add(count), u32::MAX);
                ParentsTail::Narrow(all.get_mut(start..).unwrap_or_default())
            }
            Self::Wide(all) => {
                let start = all.len();
                all.resize(start.saturating_add(count), usize::MAX);
                ParentsTail::Wide(all.get_mut(start..).unwrap_or_default())
            }
        }
    }

    pub(super) fn truncate(&mut self, kept: usize) {
        match self {
            Self::Narrow(all) => all.truncate(kept),
            Self::Wide(all) => all.truncate(kept),
        }
    }
}

fn narrow(parent: Option<usize>) -> u32 {
    parent
        .and_then(|id| u32::try_from(id).ok())
        .unwrap_or(u32::MAX)
}

pub(super) enum ParentsTail<'a> {
    Narrow(&'a mut [u32]),
    Wide(&'a mut [usize]),
}

impl ParentsTail<'_> {
    pub(super) fn split_at(self, count: usize) -> (Self, Self) {
        match self {
            Self::Narrow(all) => {
                let (head, tail) = all.split_at_mut(count.min(all.len()));
                (Self::Narrow(head), Self::Narrow(tail))
            }
            Self::Wide(all) => {
                let (head, tail) = all.split_at_mut(count.min(all.len()));
                (Self::Wide(head), Self::Wide(tail))
            }
        }
    }

    pub(super) fn set(&mut self, place: usize, parent: usize) {
        match self {
            Self::Narrow(all) => {
                if let Some(slot) = all.get_mut(place) {
                    *slot = narrow(Some(parent));
                }
            }
            Self::Wide(all) => {
                if let Some(slot) = all.get_mut(place) {
                    *slot = parent;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Rows, pack};
    use crate::store::layout::{WORD_BYTES, hash_words};

    const MOST_BYTES: usize = 3 * WORD_BYTES + 1;
    const ROWS_OF_A_STRIDE: u64 = 200;

    fn scrambled(seed: u64) -> u64 {
        let mut word = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        word = (word ^ (word >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        word ^ (word >> 31)
    }

    fn bytes_of_row(stride: usize, seed: u64) -> Vec<u8> {
        (0..stride)
            .map(|at| {
                scrambled(seed ^ u64::try_from(at).expect("a small count") << 32).to_le_bytes()[0]
            })
            .collect()
    }

    fn words_of(bytes: &[u8]) -> Vec<u64> {
        bytes
            .chunks(WORD_BYTES)
            .map(|chunk| {
                chunk
                    .iter()
                    .rev()
                    .fold(0, |word, byte| word * 256 + u64::from(*byte))
            })
            .collect()
    }

    #[test]
    fn a_row_of_any_stride_is_packed_byte_for_byte_and_read_back() {
        for stride in 1..=MOST_BYTES {
            let mut rows = Rows::new(stride);
            let mut expected = Vec::new();
            for seed in 0..ROWS_OF_A_STRIDE {
                let bytes = bytes_of_row(stride, seed);
                rows.push(&words_of(&bytes));
                expected.push(bytes);
            }
            for (id, bytes) in expected.iter().enumerate() {
                let words = words_of(bytes);
                assert_eq!(
                    rows.bytes_of(id),
                    bytes.as_slice(),
                    "stride {stride}, row {id}"
                );
                let mut read = vec![7; 5];
                rows.unpack(id, &mut read, words.len());
                assert_eq!(read, words, "stride {stride}, row {id}");
                assert!(rows.holds(id, &words), "stride {stride}, row {id}");
                assert_eq!(
                    rows.hash_of(id),
                    hash_words(&words),
                    "stride {stride}, row {id}"
                );
            }
        }
    }

    #[test]
    fn a_row_that_differs_in_one_bit_is_not_held() {
        for stride in 1..=MOST_BYTES {
            let bytes = bytes_of_row(stride, 11);
            let mut rows = Rows::new(stride);
            rows.push(&words_of(&bytes));
            for bit in 0..stride * 8 {
                let mut other = bytes.clone();
                other[bit / 8] ^= 1 << (bit % 8);
                assert!(
                    !rows.holds(0, &words_of(&other)),
                    "stride {stride}, bit {bit}"
                );
            }
        }
    }

    #[test]
    fn packing_writes_only_the_bytes_of_its_row() {
        for stride in 1..=MOST_BYTES {
            let bytes = bytes_of_row(stride, 5);
            let mut room = vec![0xAA; stride + 2];
            pack(&words_of(&bytes), &mut room[1..=stride]);
            assert_eq!(room[0], 0xAA, "stride {stride}");
            assert_eq!(room[stride + 1], 0xAA, "stride {stride}");
            assert_eq!(&room[1..=stride], bytes.as_slice(), "stride {stride}");
        }
    }
}
