//! Go identifier categories, not Unicode Alphabetic/XID (which include marks).
use unicode_general_category::{GeneralCategory as C, get_general_category};

pub(crate) fn start(ch: char) -> bool {
    ch == '_'
        || matches!(
            get_general_category(ch),
            C::UppercaseLetter
                | C::LowercaseLetter
                | C::TitlecaseLetter
                | C::ModifierLetter
                | C::OtherLetter
        )
}

pub(crate) fn continuation(ch: char) -> bool {
    start(ch) || get_general_category(ch) == C::DecimalNumber
}

/// Builtin type names cannot introduce lexical bindings (not lexer keywords).
pub(crate) fn builtin_type(name: &str) -> bool {
    matches!(
        name,
        "bool" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "f32"
    )
}

/// Builtin names are always resolved by the compiler and cannot be shadowed.
/// They remain contextual identifiers so member names such as `value.len` are
/// unaffected.
pub(crate) fn reserved_builtin(name: &str) -> bool {
    builtin_type(name)
        || matches!(
            name,
            "alloc"
                | "free"
                | "len"
                | "trap"
                | "sizeof"
                | "alignof"
                | "load32"
                | "store32"
                | "thisFile"
                | "thisLine"
                | "new"
        )
}
