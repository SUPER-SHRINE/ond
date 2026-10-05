//! O0 MIR-only Kagura backend, with demand-driven MIR runtime helpers. No AST fallback.
#[cfg(test)]
mod branch_tests;
mod calls;
#[cfg(test)]
mod conformance;
mod emit;
mod frame;
#[cfg(test)]
mod frame_tests;
mod function;
#[cfg(test)]
mod global_tests;
mod globals;
#[cfg(test)]
mod heap_tests;
#[cfg(test)]
mod host_probe_tests;
pub mod layout;
mod memory;
#[cfg(test)]
mod memory_tests;
mod numeric;
#[cfg(test)]
mod numeric_tests;
mod ops;
#[cfg(test)]
mod ordered_tests;
mod readonly;
pub mod runtime;
#[cfg(test)]
mod tests;
use crate::{
    diagnostic::{Diagnostic, FrontendError},
    ir::object::*,
    source::Span,
};
use ond_compiler_core::mir;
use ond_protocol::debug::{
    DebugObject, ObjectDebugRange, ObjectFunctionDebug, ObjectUnwindRow, SourceLocation,
};

#[derive(Debug, Clone)]
pub struct CodegenOutput {
    pub objects: Vec<Object>,
    pub debug: Vec<DebugObject>,
}

fn error(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(span, format!("Kagura MIR backend: {}", message.into()))
}

pub fn codegen_objects(project: &mir::Project) -> Result<Vec<Object>, FrontendError> {
    runtime::supply(codegen_raw(project)?.objects)
}

pub fn codegen_with_debug(
    project: &mir::Project,
    sources: &crate::source::SourceDb,
    paths: &std::collections::BTreeMap<crate::source::FileId, String>,
) -> Result<CodegenOutput, FrontendError> {
    let output = codegen_selected_with_debug(
        project,
        &project
            .packages
            .iter()
            .map(|package| package.logical_path.clone())
            .collect(),
        Some((sources, paths)),
    )?;
    runtime::supply_with_debug(output.objects, output.debug)
}

/// Runtime compilation uses this non-recursive entry after resolving its own closure.
fn codegen_raw(project: &mir::Project) -> Result<CodegenOutput, FrontendError> {
    codegen_selected_with_debug(
        project,
        &project
            .packages
            .iter()
            .map(|p| p.logical_path.clone())
            .collect(),
        None,
    )
}

/// Emit only selected packages, leaving runtime closure resolution to the final link.
/// The complete MIR remains available for cross-package layout and validation.
pub fn codegen_selected(
    project: &mir::Project,
    selected: &std::collections::BTreeSet<String>,
) -> Result<Vec<Object>, FrontendError> {
    Ok(codegen_selected_with_debug(project, selected, None)?.objects)
}

pub fn codegen_selected_with_debug(
    project: &mir::Project,
    selected: &std::collections::BTreeSet<String>,
    debug_sources: Option<(
        &crate::source::SourceDb,
        &std::collections::BTreeMap<crate::source::FileId, String>,
    )>,
) -> Result<CodegenOutput, FrontendError> {
    mir::validate_project(project).map_err(|e| {
        FrontendError::new(vec![error(
            Span::synthetic(),
            format!("invalid MIR: {e:?}"),
        )])
    })?;
    let build = || -> Result<CodegenOutput, Diagnostic> {
        let mut objects = Vec::new();
        let mut debug_objects = Vec::new();
        for package in &project.packages {
            if !selected.contains(&package.logical_path) {
                continue;
            }
            let mut emitter = emit::Emitter::default();
            let mut debug = DebugObject {
                package: package.logical_path.clone(),
                ..DebugObject::default()
            };
            let (global_data, bss, mut symbols, mut global_relocations) =
                globals::storage(&project.types, package)?;
            let (rodata, rodata_symbols, rodata_relocations) =
                readonly::storage(&project.types, package)?;
            symbols.extend(rodata_symbols);
            for f in package.functions.iter().chain(package.initializer.iter()) {
                let start = emitter.offset(f.span)?;
                let pending = function::lower(project, f, &mut emitter)?;
                let size = emitter.offset(f.span)? - start;
                let source = debug_sources
                    .and_then(|(sources, paths)| source_location(sources, paths, f.span));
                debug.functions.push(ObjectFunctionDebug {
                    symbol: f.name.clone(),
                    section: SectionKind::Text,
                    offset: start,
                    size,
                    frame_size: pending.frame_size,
                    return_address_offset: pending.return_address_offset,
                    source,
                    unwind: pending
                        .unwind
                        .into_iter()
                        .map(|row| ObjectUnwindRow {
                            offset: start + row.offset,
                            cfa_sp_offset: row.cfa_sp_offset,
                            return_address: row.return_address,
                        })
                        .collect(),
                });
                debug
                    .ranges
                    .extend(pending.ranges.into_iter().map(|range| ObjectDebugRange {
                        section: SectionKind::Text,
                        start: start + range.start,
                        end: start + range.end,
                        operation: range.operation,
                        message: range.message,
                        source: debug_sources.and_then(|(sources, paths)| {
                            source_location(sources, paths, range.span)
                        }),
                        function: f.name.clone(),
                    }));
                symbols.push(Symbol {
                    name: f.name.clone(),
                    binding: SymbolBinding::Exported,
                    kind: SymbolKind::Function,
                    section: Some(SectionKind::Text),
                    offset: start,
                    size,
                });
            }
            let (data, mut relocations) = emitter.finish()?;
            relocations.append(&mut global_relocations);
            relocations.extend(rodata_relocations);
            objects.push(Object {
                name: package.logical_path.clone(),
                sections: vec![
                    Section {
                        kind: SectionKind::Text,
                        alignment: 4,
                        memory_size: data.len() as u32,
                        data,
                    },
                    rodata,
                    global_data,
                    bss,
                ],
                symbols,
                relocations,
            });
            debug_objects.push(debug);
        }
        Ok(CodegenOutput {
            objects,
            debug: debug_objects,
        })
    };
    build().map_err(|e| FrontendError::new(vec![e]))
}

fn source_location(
    sources: &crate::source::SourceDb,
    paths: &std::collections::BTreeMap<crate::source::FileId, String>,
    span: Span,
) -> Option<SourceLocation> {
    if span.is_synthetic() {
        return None;
    }
    let file = sources.files().get(span.file.0)?;
    let (line, column) = file.line_col(span.start);
    Some(SourceLocation {
        path: paths
            .get(&span.file)
            .cloned()
            .unwrap_or_else(|| file.path().display().to_string()),
        line: u32::try_from(line).ok()?,
        column: u32::try_from(column).ok()?,
    })
}
