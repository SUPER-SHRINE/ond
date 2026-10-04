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
    (i, result)
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
    u64::from_str_radix(digits, radix).ok()
}

pub(crate) fn float(text: &str) -> Result<f32, &'static str> {
    if scan(text) != (text.len(), Ok(Kind::Float)) {
        return Err("invalid f32 literal");
    }
    let clean = text.replace('_', "");
    let value = if clean.starts_with("0x") || clean.starts_with("0X") {
        hex_float::parse(&clean)?
    } else {
        clean.parse::<f32>().map_err(|_| "invalid f32 literal")?
    };
    if !value.is_finite() {
        return Err("f32 literal out of range");
    }
    Ok(value)
}
