//! Target-independent decoding of Ond/Go string literal byte sequences.
pub(crate) fn decode(source: &str) -> Result<Vec<u8>, String> {
    if let Some(raw) = source.strip_prefix('`').and_then(|s| s.strip_suffix('`')) {
        return Ok(raw.replace('\r', "").into_bytes());
    }
    let body = source
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or("invalid string literal")?;
    let mut chars = body.chars();
    let mut bytes = Vec::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            if ch == '\n' {
                return Err("newline in quoted string literal".into());
            }
            bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
            continue;
        }
        let escape = chars.next().ok_or("incomplete string escape")?;
        match escape {
            'a' => bytes.push(7),
            'b' => bytes.push(8),
            'f' => bytes.push(12),
            'n' => bytes.push(10),
            'r' => bytes.push(13),
            't' => bytes.push(9),
            'v' => bytes.push(11),
            '\\' => bytes.push(b'\\'),
            '"' => bytes.push(b'"'),
            'x' | 'u' | 'U' | '0'..='7' => {
                let (radix, digits, mut value) = match escape {
                    'x' => (16, 2, 0),
                    'u' => (16, 4, 0),
                    'U' => (16, 8, 0),
                    ch => (8, 2, ch.to_digit(8).unwrap()),
                };
                for _ in 0..digits {
                    let digit = chars
                        .next()
                        .and_then(|ch| ch.to_digit(radix))
                        .ok_or("invalid numeric string escape")?;
                    value = value * radix + digit;
                }
                if matches!(escape, 'u' | 'U') {
                    let ch =
                        char::from_u32(value).ok_or("invalid Unicode scalar in string escape")?;
                    bytes.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                } else {
                    bytes.push(u8::try_from(value).map_err(|_| "string byte escape out of range")?);
                }
            }
            _ => return Err(format!("invalid string escape: \\{escape}")),
        }
    }
    Ok(bytes)
}

pub(crate) fn import_path(source: &str) -> Result<String, String> {
    String::from_utf8(decode(source)?).map_err(|_| "import path must be valid UTF-8".into())
}
