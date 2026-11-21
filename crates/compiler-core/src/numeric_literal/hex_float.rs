//! Direct binary32 and binary64 rounding with guard and sticky bits.
pub(super) fn parse(text: &str, fraction_bits: u32, bias: i64) -> Result<u64, &'static str> {
    let (mantissa, exponent) = text[2..]
        .split_once(['p', 'P'])
        .ok_or("hexadecimal float requires a p exponent")?;
    let negative = exponent.starts_with('-');
    let mut scale = 0i64;
    for digit in exponent.trim_start_matches(['+', '-']).bytes() {
        scale = scale
            .saturating_mul(10)
            .saturating_add(i64::from(digit - b'0'));
    }
    if negative {
        scale = scale.saturating_neg();
    }
    let fraction = mantissa.split_once('.').map_or(0, |(_, s)| s.len());
    scale = scale.saturating_sub((fraction as i64).saturating_mul(4));
    let mut bits = Vec::new();
    let mut started = false;
    for digit in mantissa.chars().filter(|ch| *ch != '.') {
        let nibble = digit.to_digit(16).ok_or("invalid hexadecimal mantissa")?;
        for shift in (0..4).rev() {
            let bit = (nibble >> shift) & 1 != 0;
            started |= bit;
            if started {
                bits.push(bit);
            }
        }
    }
    if bits.is_empty() {
        return Ok(0);
    }
    let mut exponent = scale.saturating_add(bits.len() as i64 - 1);
    if exponent > bias {
        return Err("floating-point literal out of range");
    }
    let normal_min = 1 - bias;
    let halfway_min = normal_min - i64::from(fraction_bits) - 1;
    if exponent < halfway_min {
        return Ok(0);
    }
    let count = if exponent >= normal_min {
        fraction_bits as usize + 1
    } else {
        (exponent - halfway_min) as usize
    };
    let mut significand = 0u64;
    for i in 0..count {
        significand = (significand << 1) | u64::from(bits.get(i).copied().unwrap_or(false));
    }
    let guard = bits.get(count).copied().unwrap_or(false);
    let sticky = bits
        .get(count + 1..)
        .is_some_and(|tail| tail.iter().any(|bit| *bit));
    if guard && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    if exponent < normal_min {
        return Ok(significand);
    }
    if significand == 1 << (fraction_bits + 1) {
        significand >>= 1;
        exponent += 1;
    }
    if exponent > bias {
        return Err("floating-point literal out of range");
    }
    Ok(((exponent + bias) as u64) << fraction_bits | (significand & ((1 << fraction_bits) - 1)))
}
