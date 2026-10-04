use super::{FileId, TokenKind as T, parse_source_file, tokenize};
use crate::{
    ir::ast,
    numeric_literal,
    test_support::{compile, compile_files},
};

fn lexical_error(source: &str, messages: &[&str]) {
    let errors = tokenize(FileId(7), source).unwrap_err().into_vec();
    assert_eq!(errors.len(), messages.len(), "{source}: {errors:?}");
    for (error, message) in errors.iter().zip(messages) {
        assert!(error.message.contains(message), "{source}: {error:?}");
        assert_eq!(error.severity, crate::diagnostic::Severity::Error);
        let span = error.primary.span;
        assert_eq!(span.file, FileId(7));
        assert!(span.start < span.end && span.end <= source.len());
        assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
    }
}

#[test]
fn unicode_identifiers_use_letter_and_decimal_digit_categories() {
    for name in ["日本語", "αβ", "𐐀", "a٣", "_９", "é", "ǅ", "ʰ"] {
        let tokens = tokenize(FileId(0), name).unwrap();
        assert_eq!(tokens[0].kind, T::Identifier, "{name}");
        assert_eq!(tokens[0].text, name);
        assert_eq!(tokens[0].span.end, name.len());
        assert_eq!(tokens[1].kind, T::Semi);
    }
    for name in ["٣a", "a²", "a\u{0301}", "\u{0345}", "a\u{200c}", "Ⅳ", "😀"] {
        lexical_error(name, &["unexpected character"]);
    }
    compile(
        "package main\nfunc main() { var 日本語 = 1; var α٢ = 日本語; _ = α٢ }",
        None,
    )
    .unwrap();
}

#[test]
fn unicode_does_not_change_ascii_only_export_policy() {
    compile_files(&[
        (
            "main.ond",
            "package main\nimport \"shared\"\nfunc main() { _ = shared.A値 }",
        ),
        ("shared/shared.ond", "package shared\nconst A値 = 1"),
    ])
    .unwrap();
    crate::test_support::reject(
        "package main\nimport \"shared\"\nfunc main() { _ = shared.値 }",
        Some("package shared\nconst 値 = 1"),
        "not exported",
        "shared.値",
    );
}

#[test]
fn integer_spellings_and_radices() {
    for (source, expected) in [
        ("0", 0),
        ("42", 42),
        ("4_2", 42),
        ("0600", 384),
        ("0_600", 384),
        ("0o600", 384),
        ("0O_600", 384),
        ("0b_101", 5),
        ("0B101", 5),
        ("0x_dead_BEEF", 0xdeadbeef),
        ("0X42", 66),
    ] {
        let tokens = tokenize(FileId(0), source).unwrap();
        assert_eq!(tokens[0].kind, T::IntLit);
        assert_eq!(tokens[0].text, source);
        assert_eq!(numeric_literal::integer(source), Some(expected));
    }
    // Width restrictions belong to semantic analysis, not tokenization.
    tokenize(
        FileId(0),
        "999999999999999999999999999999999999999999999999999",
    )
    .unwrap();
}

#[test]
fn recognizes_assignment_and_update_operators_as_longest_tokens() {
    let tokens = tokenize(FileId(0), "+= -= *= /= %= &= |= ^= &^= <<= >>= ++ --").unwrap();
    assert_eq!(
        tokens.iter().map(|token| token.kind).collect::<Vec<_>>(),
        [
            T::PlusAssign,
            T::MinusAssign,
            T::StarAssign,
            T::SlashAssign,
            T::PercentAssign,
            T::AmpAssign,
            T::PipeAssign,
            T::CaretAssign,
            T::AmpCaretAssign,
            T::ShiftLeftAssign,
            T::ShiftRightAssign,
            T::Increment,
            T::Decrement,
            T::Semi,
            T::Eof,
        ]
    );
}

#[test]
fn float_forms_and_exponent_boundaries() {
    for source in [
        ".5",
        "1.",
        "1.e2",
        "1e-2",
        "1E+2",
        "0.15e+0_2",
        "08.5",
        "0x1p0",
        "0X_1.Fp+2",
        "0x.8p-0",
        "0x1.p0",
        "0x1_2.3_4P-1_0",
    ] {
        let tokens = tokenize(FileId(0), source).unwrap();
        assert_eq!(tokens[0].kind, T::FloatLit, "{source}");
        assert_eq!(tokens[0].text, source);
        assert!(numeric_literal::float(source).is_ok(), "{source}");
    }
    let tokens = tokenize(FileId(0), "0x1e2 + 1e2 - .5 * 1.").unwrap();
    assert_eq!(
        tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        [
            T::IntLit,
            T::Plus,
            T::FloatLit,
            T::Minus,
            T::FloatLit,
            T::Star,
            T::FloatLit,
            T::Semi,
            T::Eof
        ]
    );
}

#[test]
fn invalid_numeric_spelling_is_a_lexical_error() {
    for source in [
        "08", "0_9", "0b2", "0o8", "0x", "0x_", "42_", "4__2", "0_x1", "0b__1", "1_.0", "1._0",
        "1e_2", "1e2_", "1e", "1e+", "0x1.0", "0x1p", "0x1p-", "0x_ .1p0", "0x_.1p0", "0x1._2p0",
        "0x1p_0", "0b1.0", "1p2",
    ] {
        let messages: &[&str] = match source {
            "08" | "0b2" | "0o8" => &["invalid digit for numeric radix"],
            "0x" => &["numeric literal requires digits"],
            "1e" | "1e+" | "0x1p" | "0x1p-" => &["exponent requires decimal digits"],
            "0x1.0" => &["hexadecimal float requires a p exponent"],
            "0b1.0" => &["floating-point literal requires decimal or hexadecimal radix"],
            "1p2" => &["decimal float requires an e exponent"],
            "0x_ .1p0" => &[
                "underscore must separate digits",
                "decimal float requires an e exponent",
            ],
            _ => &["underscore must separate digits"],
        };
        lexical_error(source, messages);
    }
    crate::test_support::reject(
        "package main\nfunc main() {}\nvar x = 1i",
        None,
        "expected `;` or newline",
        "i",
    );
}

#[test]
fn strings_are_validated_even_without_hir_lowering() {
    for source in [
        r#""\q""#,
        r#""\x0""#,
        r#""\xgg""#,
        r#""\400""#,
        r#""\uD800""#,
        r#""\U00110000""#,
        "\"a\nb\"",
        "\"a\\\nb\"",
    ] {
        let messages: &[&str] = match source {
            r#""\q""# => &["invalid string escape"],
            r#""\400""# => &["string byte escape out of range"],
            r#""\uD800""# | r#""\U00110000""# => &["invalid Unicode scalar in string escape"],
            "\"a\nb\"" | "\"a\\\nb\"" => {
                &["unterminated string literal", "unterminated string literal"]
            }
            _ => &["invalid numeric string escape"],
        };
        lexical_error(source, messages);
    }
    let text = r#""日本語\x00\377\u0041\U0001F600""#;
    let tokens = tokenize(FileId(0), text).unwrap();
    assert_eq!(tokens[0].text, text);
    assert_eq!(
        crate::string_literal::decode(text).unwrap(),
        ["日本語".as_bytes(), &[0, 255, 65], "😀".as_bytes()].concat()
    );
    assert_eq!(
        crate::string_literal::decode("`a\r\nb\rc`").unwrap(),
        b"a\nbc"
    );
    let quoted_cr = "\"a\rb\"";
    tokenize(FileId(0), quoted_cr).unwrap();
    assert_eq!(crate::string_literal::decode(quoted_cr).unwrap(), b"a\rb");
}

#[test]
fn import_paths_use_the_same_string_decoding() {
    for path in [r#""\u0073hared""#, "`shared`", r#""shared""#] {
        let source = format!("package main\nimport {path}\nfunc main() {{ _ = shared.Value }}");
        compile_files(&[
            ("main.ond", &source),
            ("shared/shared.ond", "package shared\nconst Value = 1"),
        ])
        .unwrap();
    }
    crate::test_support::reject(
        "package main\nimport \"\\xff\"\nfunc main() {}",
        None,
        "import path must be valid UTF-8",
        "import \"\\xff\"",
    );
}

#[test]
fn hexadecimal_conversion_matches_exact_power_of_two_reference() {
    for seed in 0..256u32 {
        let n = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        for exponent in [-180, -160, -150, -126, -32, -1, 0, 64, 96, 127] {
            // A 32-bit integer scaled by these powers is exactly representable in f64.
            let reference = (f64::from(n) * 2f64.powi(exponent)) as f32;
            let source = format!("0x{n:x}p{exponent}");
            let actual = numeric_literal::float(&source);
            if reference.is_finite() {
                assert_eq!(actual.unwrap().to_bits(), reference.to_bits(), "{source}");
            } else {
                assert_eq!(actual.unwrap_err(), "f32 literal out of range", "{source}");
            }
        }
    }
}

#[test]
fn eof_comments_bom_and_source_offsets() {
    for source in [
        "package main\nfunc main() {}",
        "package main\nfunc main() {} // trailing comment",
        "\u{feff}package main\r\nfunc main() {}",
    ] {
        parse_source_file(FileId(0), source).unwrap();
    }
    for source in [
        "package\0main",
        "//\0",
        "`\0`",
        "//\u{feff}",
        "package main\n\u{feff}func main() {}",
    ] {
        lexical_error(source, &["NUL or non-leading byte order mark in source"]);
    }
    let tokens = tokenize(FileId(0), "値/* comment\nnext */次").unwrap();
    assert_eq!(
        tokens.iter().map(|t| t.kind).collect::<Vec<_>>(),
        [T::Identifier, T::Semi, T::Identifier, T::Semi, T::Eof]
    );
    for t in tokens {
        assert!("値/* comment\nnext */次".is_char_boundary(t.span.start));
        assert!("値/* comment\nnext */次".is_char_boundary(t.span.end));
    }
    lexical_error("/* not closed", &["unterminated block comment"]);
}

#[test]
fn octal_array_lengths_and_float_literals_reach_mir() {
    let c = compile(
        "package main\nfunc main() { var a: [010]u8; _ = a; _ = 0x1.8p1; _ = .5; _ = 1. }",
        None,
    )
    .unwrap();
    assert!(
        c.mir
            .types
            .definitions()
            .iter()
            .any(|d| matches!(d.kind, crate::semantic::TypeKind::Array { length: 8, .. }))
    );
    let bits = c.mir.packages[0].functions[0]
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .filter_map(|i| match i.kind {
            crate::mir::InstructionKind::Constant(crate::mir::Constant::Float32 {
                bits, ..
            }) => Some(bits),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(bits, [3f32.to_bits(), 0.5f32.to_bits(), 1f32.to_bits()]);
    let file = parse_source_file(FileId(0), "package main\nconst X = 0x1p2").unwrap();
    assert!(
        matches!(&file.decls[0], ast::TopLevelDecl::Const(c) if matches!(&c.specs[0].values[0], ast::Expr::Literal(ast::Literal::Float(s, _)) if s == "0x1p2"))
    );
}

#[test]
fn hexadecimal_float_rounds_once_with_guard_and_sticky_bits() {
    for (source, expected) in [
        ("0x1p0", 0x3f800000),
        ("0x1.000001p0", 0x3f800000),
        ("0x1.00000100000000000001p0", 0x3f800001),
        ("0x1.000003p0", 0x3f800002),
        ("0x1.fffffep127", 0x7f7fffff),
        ("0x1p-126", 0x00800000),
        ("0x1p-149", 1),
        ("0x1p-150", 0),
        ("0x1.00000000001p-150", 1),
        ("0x3p-150", 2),
        ("0x1.fffffcp-127", 0x007fffff),
        ("0x1.fffffep-127", 0x00800000),
        ("0x0p9999999999999999999999", 0),
        ("0x1p-9999999999999999999999", 0),
    ] {
        assert_eq!(
            numeric_literal::float(source).unwrap().to_bits(),
            expected,
            "{source}"
        );
    }
    for source in [
        "0x1p128",
        "0x1.ffffffp127",
        "0x1p9999999999999999999999",
        "1e100",
    ] {
        assert_eq!(
            numeric_literal::float(source).unwrap_err(),
            "f32 literal out of range",
            "{source}"
        );
    }
    assert_eq!(numeric_literal::float(".5").unwrap(), 0.5);
}
