//! Target-neutral AST → typed HIR → MIR lowering.
mod hir;
mod mir_typed;
pub use hir::SemanticCache;
pub use hir::generation::GenerationInputs;
pub(crate) use hir::lower_to_hir;
pub(crate) use hir::lower_to_hir_with_generation;
pub(crate) use mir_typed::lower_to_mir;
