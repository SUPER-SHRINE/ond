include!("ond/shared.rs");
include!("ond/lexer.rs");
include!("ond/parser.rs");
#[path = "ond/control_syntax.rs"]
mod control_syntax;
#[path = "ond/inline_asm.rs"]
mod inline_asm;
#[path = "ond/signatures.rs"]
mod signatures;
use control_syntax::HeaderMode;
mod roles;
pub use roles::{declaration_span, parse_context_file, parse_snapshot_file};
pub(crate) use roles::{role_identifier, role_regions};
include!("ond/tests.rs");
#[cfg(test)]
#[path = "ond/lexical_tests.rs"]
mod lexical_tests;
