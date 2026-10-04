//! Target-neutral AST → typed HIR → MIR lowering.
mod hir;
mod mir_typed;
pub(crate) use hir::lower_to_hir;
pub(crate) use mir_typed::lower_to_mir;
