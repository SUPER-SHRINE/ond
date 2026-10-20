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
pub use pipeline::GenerationInputs;
pub use pipeline::SemanticCache;
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
    ExternalSource, ExternalSources, load_project_with_inputs,
    load_project_with_inputs_allow_incomplete, load_project_with_test_inputs,
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

/// Compile source and retain the normalized Generation inputs used by artifact hashes.
/// Logical root names follow the project manifest; physical main symbols are preserved.
pub fn compile_loaded_project_with_generation(
    mut loaded: LoadedProject,
) -> Result<(Compilation, GenerationInputs), FrontendError> {
    let mut ast = ond::parse_project(&loaded)?;
    let (mut hir, generation) = pipeline::lower_to_hir_with_generation(&loaded, &ast)?;
    let root_name = project::project_name(&loaded.manifest_path)?;
    if let Some(root) = loaded
        .packages
        .iter()
        .find(|package| package.logical_path == ".")
        .cloned()
    {
        if root.directory == loaded.root {
            let files = root.files.iter().copied().collect();
            hir.types.name_logical_root(&files, &root_name);
            for package in &mut loaded.packages {
                if package.id == root.id {
                    package.logical_path = root_name.clone();
                }
            }
            for package in &mut ast.packages {
                if package.logical_path == "." {
                    package.logical_path = root_name.clone();
                }
            }
            for package in &mut hir.packages {
                if package.id == root.id {
                    package.logical_path = root_name.clone();
                }
            }
        } else {
            loaded.packages.retain(|package| package.id != root.id);
            ast.packages.retain(|package| package.logical_path != ".");
            hir.packages.retain(|package| package.id != root.id);
            hir.symbols
                .retain(|symbol| !root.files.contains(&symbol.span.file));
            if let Some(package) = hir.packages.first() {
                hir.entry_package = package.id;
            }
        }
    }
    let generation = generation.normalize(&hir).map_err(|message| {
        FrontendError::new(vec![Diagnostic::error(source::Span::synthetic(), message)])
    })?;
    let mir = lower_to_mir(&hir)?;
    mir::validate_project(&mir).map_err(|errors| {
        FrontendError::new(
            errors
                .into_iter()
                .map(|error| {
                    Diagnostic::error(
                        source::Span::synthetic(),
                        format!(
                            "internal MIR validation error in {}: {}",
                            error.function, error.message
                        ),
                    )
                })
                .collect(),
        )
    })?;
    Ok((
        Compilation {
            loaded,
            ast,
            hir,
            mir,
        },
        generation,
    ))
}

#[cfg(test)]
mod array_layout_tests;
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
mod generic_diagnostic_tests;
#[cfg(test)]
mod generic_identity_tests;
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
mod operator_len_tests;
#[cfg(test)]
mod operator_overload_tests;
#[cfg(test)]
mod project_contract_tests;
#[cfg(test)]
mod review_regression_tests;
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
