#![forbid(unsafe_code)]

//! Target-neutral Ond frontend.
//!
//! This crate owns source loading, parsing, HIR construction, and MIR
//! construction. Target code generation, object emission, linking, and
//! cartridge construction belong to downstream crates.

use std::collections::BTreeMap;
use std::path::Path;

mod constant;
mod identifier;
mod numeric_literal;
mod pipeline;
mod string_literal;
use pipeline::{lower_to_hir, lower_to_mir};
pub mod diagnostic;
pub mod export;
pub mod ir;
pub mod mir;
pub mod ond;
pub mod project;
mod project_input;
pub mod semantic;
pub mod source;
/// Decode an import path using the language's string-literal rules.
pub fn decode_import_path(source: &str) -> Result<String, String> {
    string_literal::import_path(source)
}
#[cfg(any(test, feature = "conformance-fixtures"))]
pub mod test_vectors;

pub use diagnostic::{Diagnostic, FrontendError, Severity};
use ir::{ast, hir};
pub use project::{
    LoadedProject, PackageId, PackageSources, load_project, load_project_with_overrides,
    load_project_with_overrides_and_exclusions,
};
pub use project_input::{
    ExternalSource, ExternalSources, load_library_sources, load_project_with_inputs,
    load_project_with_inputs_allow_incomplete,
};

#[derive(Debug, Clone)]
pub struct Compilation {
    pub loaded: LoadedProject,
    pub ast: ast::Project,
    pub hir: hir::Project,
    pub mir: mir::Project,
}

pub fn compile_project(root: impl AsRef<Path>) -> Result<Compilation, FrontendError> {
    compile_project_with_overrides(root, &BTreeMap::new())
}

pub fn compile_project_with_overrides(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<std::path::PathBuf, String>,
) -> Result<Compilation, FrontendError> {
    let loaded = project::load_project_with_overrides(root, overrides)?;
    compile_loaded_project(loaded)
}

pub fn compile_loaded_project(loaded: LoadedProject) -> Result<Compilation, FrontendError> {
    let ast = ond::parse_project(&loaded)?;
    let hir = lower_to_hir(&loaded, &ast)?;
    let mir = lower_to_mir(&hir)?;
    mir::validate_project(&mir).map_err(|errors| {
        FrontendError::new(
            errors
                .into_iter()
                .map(|error| {
                    Diagnostic::error(
                        crate::source::Span::synthetic(),
                        format!(
                            "internal MIR validation error in {}: {}",
                            error.function, error.message
                        ),
                    )
                })
                .collect(),
        )
    })?;
    Ok(Compilation {
        loaded,
        ast,
        hir,
        mir,
    })
}

#[cfg(test)]
mod call_tests;
#[cfg(test)]
mod completion_tests;
#[cfg(test)]
mod conformance;
#[cfg(test)]
mod constant_tests;
#[cfg(test)]
mod constant_value_tests;
#[cfg(test)]
mod control_flow_tests;
#[cfg(test)]
mod defer_tests;
#[cfg(test)]
mod function_type_tests;
#[cfg(test)]
mod global_tests;
#[cfg(test)]
mod inferred_array_tests;
#[cfg(test)]
mod interface_tests;
#[cfg(test)]
mod memory_test_vm;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod method_tests;
#[cfg(test)]
mod operator_assignment_tests;
#[cfg(test)]
mod operator_overload_tests;
#[cfg(test)]
mod semantic_tests;
#[cfg(test)]
mod specification_decision_tests;
#[cfg(test)]
mod syntax_discard_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod type_identity_tests;

pub use ond::parse_source_file;

#[cfg(test)]
mod tests;
