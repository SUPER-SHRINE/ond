//! Test-only LinkedImage builders and CPU harness; not a distributable machine profile.
use crate::diagnostic::{Diagnostic, FrontendError};
use crate::ir::object::{Object, SectionKind};
use crate::ir::object::{RelocationKind, SymbolBinding};
use crate::linker::LinkedImage;
use crate::pipeline::{Compilation, compile_project};
use std::path::Path;
const RAM_BASE: u32 = 0x0000_1000;

const ENTRY_SYMBOL: &str = "__ond.entry";
const HEAP_BASE_SYMBOL: &str = "__ond_heap_base";
const STACK_BOTTOM_SYMBOL: &str = "__ond_stack_bottom";

/// Compile Ond source through the target-neutral core and MIR backend.
pub fn compile_image(root: impl AsRef<Path>) -> Result<LinkedImage, FrontendError> {
    build_compilation(&compile_project(root)?)
}

/// Build directly from MIR (e.g. backend clients and mutation tests).
pub fn build_image(
    project: &ond_compiler_core::mir::Project,
    config: crate::ProjectConfig,
) -> Result<LinkedImage, FrontendError> {
    link_project(
        project,
        crate::mir_backend::codegen_objects(project)?,
        config,
    )
}

/// Link the MIR-generated objects, preserving the core's explicit initializer order.
pub fn build_compilation(compilation: &Compilation) -> Result<LinkedImage, FrontendError> {
    let config = test_project_config(&compilation.loaded.manifest_path)?;
    link_project(&compilation.mir, compilation.objects.clone(), config)
}

fn test_project_config(path: &Path) -> Result<crate::ProjectConfig, FrontendError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| frontend_error(format!("failed to read `{}`: {error}", path.display())))?;
    let mut config = crate::ProjectConfig::default();
    let mut section = "";
    for raw_line in text.lines() {
        let line = raw_line.split(['#', ';']).next().unwrap_or("").trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim();
            continue;
        }
        if section != "target.kagura" {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(value) = test_size(value.trim().trim_matches('"')) else {
            return Err(frontend_error(format!(
                "invalid target.kagura.{} value",
                key.trim()
            )));
        };
        match key.trim() {
            "ram" => config.ram_size = value,
            "stack" => config.stack_size = value,
            _ => {}
        }
    }
    Ok(config)
}

fn test_size(text: &str) -> Option<u32> {
    let (number, multiplier) = if let Some(number) = text.trim().strip_suffix("KiB") {
        (number.trim(), 1024u64)
    } else if let Some(number) = text.trim().strip_suffix("MiB") {
        (number.trim(), 1024u64 * 1024)
    } else {
        (text.trim(), 1)
    };
    u32::try_from(number.parse::<u64>().ok()?.checked_mul(multiplier)?).ok()
}
fn link_project(
    project: &ond_compiler_core::mir::Project,
    packages: Vec<Object>,
    config: crate::ProjectConfig,
) -> Result<LinkedImage, FrontendError> {
    let metadata = crate::program::ProgramMetadata::from_mir(project)?;
    let mut objects = vec![startup_object(
        &metadata.initializers,
        &metadata.entry_symbol,
    )];
    objects.extend(packages);
    link_objects(&objects, config.ram_size, config.stack_size)
}

// Test-only memory layout; no serialized executable format.
fn link_objects(
    objects: &[Object],
    ram_size: u32,
    stack_size: u32,
) -> Result<LinkedImage, FrontendError> {
    crate::linker::link_objects(
        objects,
        &crate::linker::LinkPlan {
            read_only: None,
            memory_base: RAM_BASE,
            memory_size: ram_size,
            stack_size,
            entry_symbol: ENTRY_SYMBOL.into(),
            heap_base_symbol: HEAP_BASE_SYMBOL.into(),
            stack_bottom_symbol: STACK_BOTTOM_SYMBOL.into(),
        },
    )
    .map_err(|error| frontend_error(error.message))
}

fn frontend_error(message: impl Into<String>) -> FrontendError {
    FrontendError::new(vec![Diagnostic::error(
        crate::source::Span::synthetic(),
        message,
    )])
}
fn startup_object(init_symbols: &[String], main_symbol: &str) -> Object {
    use crate::kagura_encoding::word;
    // Test-machine exit MMIO is cartridge policy, not part of the shared ABI.
    crate::program::program_startup(
        init_symbols,
        main_symbol,
        &[
            word(0, 1, 0, 0, -1),
            word(2, 1, 1, 0, 15),
            word(0, 1, 1, 0, 64),
            word(11, 0, 1, 0, 0),
            word(12, 0, 0, 0, -1),
        ],
    )
}
#[path = "branch_tests.rs"]
mod branch_tests;
mod machine;
mod runtime_tests;
