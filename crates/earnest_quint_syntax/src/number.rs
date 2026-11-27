pub(crate) fn decimal(written: &str) -> String {
    let digits: String = written.chars().filter(|char| *char != '_').collect();
    match digits.strip_prefix("0x") {
        Some(hex) => from_hex(hex),
        None => digits,
    }
}

const BILLION: u64 = 1_000_000_000;

fn from_hex(hex: &str) -> String {
    let mut limbs: Vec<u64> = vec![0];
    for digit in hex.chars().filter_map(|char| char.to_digit(16)) {
        let mut carry = u64::from(digit);
        for limb in &mut limbs {
            let value = *limb * 16 + carry;
            *limb = value % BILLION;
            carry = value / BILLION;
        }
        while carry > 0 {
            limbs.push(carry % BILLION);
            carry /= BILLION;
        }
    }
    limbs
        .iter()
        .rev()
        .enumerate()
        .map(|(index, limb)| {
            if index == 0 {
                limb.to_string()
            } else {
                format!("{limb:09}")
            }
        })
        .collect()
}
