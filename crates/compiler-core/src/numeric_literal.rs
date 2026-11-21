//! Number token boundaries, spelling validation and target-neutral conversion.
mod hex_float;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Integer,
    Float,
}

/// Scan ASCII numeric syntax. Invalid digits/separators are diagnosed before HIR.
pub(crate) fn scan(source: &str) -> (usize, Result<Kind, &'static str>) {
    let bytes = source.as_bytes();
    let radix = match bytes.get(..2) {
        Some(b"0x" | b"0X") => 16,
        Some(b"0b" | b"0B") => 2,
        Some(b"0o" | b"0O") => 8,
        _ => 10,
    };
    let prefix = if radix == 10 { 0 } else { 2 };
    let mut i = prefix;
    let digit = |b: u8| b.is_ascii_digit() || (radix == 16 && b.is_ascii_hexdigit());
    while bytes.get(i).is_some_and(|b| digit(*b) || *b == b'_') {
        i += 1;
    }
    let integer_end = i;
    let dot = bytes.get(i) == Some(&b'.');
    let mut fraction_start = i;
    if dot {
        i += 1;
        fraction_start = i;
        while bytes.get(i).is_some_and(|b| digit(*b) || *b == b'_') {
            i += 1;
        }
    }
    let fraction_end = i;
    let exponent = bytes.get(i).copied().filter(|b| {
        if radix == 16 {
            matches!(b, b'p' | b'P')
        } else {
            matches!(b, b'e' | b'E' | b'p' | b'P')
        }
    });
    let mut exponent_start = i;
    if exponent.is_some() {
        i += 1;
        if bytes.get(i).is_some_and(|b| matches!(b, b'+' | b'-')) {
            i += 1;
        }
        exponent_start = i;
        while bytes
            .get(i)
            .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
        {
            i += 1;
        }
    }
    let result = (|| {
        let float = dot || exponent.is_some();
        if float && (radix == 2 || radix == 8) {
            return Err("floating-point literal requires decimal or hexadecimal radix");
        }
        if radix == 16 && dot && exponent.is_none() {
            return Err("hexadecimal float requires a p exponent");
        }
        if radix != 16 && matches!(exponent, Some(b'p' | b'P')) {
            return Err("decimal float requires an e exponent");
        }
        let actual_radix =
            if !float && radix == 10 && bytes.first() == Some(&b'0') && integer_end > 1 {
                8
            } else {
                radix
            };
        let whole = digits(&source[prefix..integer_end], actual_radix, prefix > 0)?;
        let fraction = if dot {
            digits(&source[fraction_start..fraction_end], radix, false)?
        } else {
            0
        };
        if whole + fraction == 0 {
            return Err("numeric literal requires digits");
        }
        if exponent.is_some() && digits(&source[exponent_start..i], 10, false)? == 0 {
            return Err("exponent requires decimal digits");
        }
        Ok(if float { Kind::Float } else { Kind::Integer })
    })();
    let mut candidate_end = if bytes.first() == Some(&b'0') {
        let mut end = 1;
        while bytes.get(end) == Some(&b'_') {
            end += 1;
        }
        if bytes
            .get(end)
            .is_some_and(|byte| matches!(byte, b'b' | b'B' | b'o' | b'O' | b'x' | b'X'))
        {
            end + 1
        } else {
            0
        }
    } else {
        0
    };
    let hex_candidate = candidate_end > 0 && matches!(bytes[candidate_end - 1], b'x' | b'X');
    while let Some(&byte) = bytes.get(candidate_end) {
        let exponent = if hex_candidate {
            matches!(byte, b'p' | b'P')
        } else {
            matches!(byte, b'e' | b'E' | b'p' | b'P')
        };
        let allowed = byte.is_ascii_digit()
            || matches!(byte, b'_' | b'.')
            || exponent
            || (hex_candidate && byte.is_ascii_hexdigit())
            || (matches!(byte, b'+' | b'-')
                && candidate_end > prefix
                && if hex_candidate {
                    matches!(bytes[candidate_end - 1], b'p' | b'P')
                } else {
                    matches!(bytes[candidate_end - 1], b'e' | b'E' | b'p' | b'P')
                });
        if !allowed {
            break;
        }
        candidate_end += 1;
    }
    if candidate_end != i {
        (candidate_end, result.and(Err("invalid numeric literal")))
    } else {
        (i, result)
    }
}

fn digits(text: &str, radix: u32, prefix: bool) -> Result<usize, &'static str> {
    let bytes = text.as_bytes();
    let valid = |b: u8| b.is_ascii_hexdigit() && (b as char).to_digit(radix).is_some();
    let mut count = 0;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'_' {
            if !(i == 0 && prefix || i > 0 && valid(bytes[i - 1]))
                || !bytes.get(i + 1).is_some_and(|b| valid(*b))
            {
                return Err("underscore must separate digits or immediately follow a radix prefix");
            }
        } else if valid(b) {
            count += 1;
        } else {
            return Err("invalid digit for numeric radix");
        }
    }
    Ok(count)
}

pub(crate) fn integer(text: &str) -> Option<u64> {
    use num_traits::ToPrimitive;
    integer_exact(text)?.to_u64()
}

pub(crate) fn integer_exact(text: &str) -> Option<num_bigint::BigInt> {
    if scan(text) != (text.len(), Ok(Kind::Integer)) {
        return None;
    }
    let clean = text.replace('_', "");
    let (radix, digits) = match clean.get(..2) {
        Some("0x" | "0X") => (16, &clean[2..]),
        Some("0b" | "0B") => (2, &clean[2..]),
        Some("0o" | "0O") => (8, &clean[2..]),
        _ if clean.starts_with('0') && clean.len() > 1 => (8, clean.as_str()),
        _ => (10, clean.as_str()),
    };
    num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix)
}

pub(crate) fn float(text: &str) -> Result<f32, &'static str> {
    if scan(text) != (text.len(), Ok(Kind::Float)) {
        return Err("invalid f32 literal");
    }
    let clean = text.replace('_', "");
    let value = if clean.starts_with("0x") || clean.starts_with("0X") {
        f32::from_bits(
            hex_float::parse(&clean, 23, 127).map_err(|_| "f32 literal out of range")? as u32,
        )
    } else {
        clean.parse::<f32>().map_err(|_| "invalid f32 literal")?
    };
    if !value.is_finite() {
        return Err("f32 literal out of range");
    }
    Ok(value)
}

pub(crate) fn double(text: &str) -> Result<f64, &'static str> {
    let clean = text.replace('_', "");
    let value = if clean.starts_with("0x") || clean.starts_with("0X") {
        f64::from_bits(hex_float::parse(&clean, 52, 1023)?)
    } else {
        clean.parse().map_err(|_| "invalid f64 literal")?
    };
    value
        .is_finite()
        .then_some(value)
        .ok_or("f64 literal out of range")
}

pub(crate) fn fixed(text: &str, fraction: u32, negative: bool) -> Result<u64, &'static str> {
    use num_bigint::BigInt;
    use num_traits::{ToPrimitive, Zero};

    const OVERFLOW: &str = "fixed-point literal out of range";
    let raw = if let Some(integer) = integer_exact(text) {
        integer << fraction
    } else {
        let clean = text.replace('_', "");
        let hex = clean.starts_with("0x") || clean.starts_with("0X");
        let (mantissa, exponent) = if hex {
            clean[2..]
                .split_once(['p', 'P'])
                .ok_or("hexadecimal float requires a p exponent")?
        } else {
            clean.split_once(['e', 'E']).unwrap_or((&clean, "0"))
        };
        let mut scale = exponent.parse::<i64>().unwrap_or_else(|_| {
            if exponent.starts_with('-') {
                i64::MIN
            } else {
                i64::MAX
            }
        });
        let fractional_digits = mantissa.split_once('.').map_or(0, |(_, frac)| frac.len()) as i64;
        scale = scale.saturating_sub(fractional_digits * if hex { 4 } else { 1 });
        let digits = mantissa.replace('.', "");
        let value = BigInt::parse_bytes(digits.as_bytes(), if hex { 16 } else { 10 })
            .ok_or("invalid fixed-point literal")?;
        if value.is_zero() {
            return Ok(0);
        }
        if scale > 64 {
            return Err(OVERFLOW);
        }
        // Any remaining exponent corresponds to a bounded integer or a
        // denominator with size proportional to the source significand.
        if scale < -(4 * digits.len() as i64 + 64) {
            return Ok(0);
        }
        let base = BigInt::from(if hex { 2 } else { 10 });
        let scaled = value << fraction;
        if scale >= 0 {
            scaled * base.pow(scale as u32)
        } else {
            let denominator = base.pow((-scale) as u32);
            let q = &scaled / &denominator;
            let twice_remainder = (&scaled % &denominator) * 2u32;
            if twice_remainder > denominator
                || (twice_remainder == denominator && (&q % 2u32) != BigInt::ZERO)
            {
                q + 1
            } else {
                q
            }
        }
    };
    let raw = if negative { -raw } else { raw };
    crate::constant::fixed_bits(raw.to_i128().ok_or(OVERFLOW)?, fraction)
}
