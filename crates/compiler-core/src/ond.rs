include!("ond/shared.rs");
include!("ond/lexer.rs");
include!("ond/parser.rs");
#[path = "ond/control_syntax.rs"]
mod control_syntax;
#[path = "ond/signatures.rs"]
mod signatures;
use control_syntax::HeaderMode;
include!("ond/tests.rs");
#[cfg(test)]
#[path = "ond/lexical_tests.rs"]
mod lexical_tests;
