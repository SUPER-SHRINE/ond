//! In-memory package validation. Each entry owns its source and type IDs.
use super::{lower_context_to_hir, validate_generic_body_names, validate_test_signatures};
use crate::diagnostic::{Diagnostic, FrontendError};
use crate::ir::{ast, hir};
use crate::project::{LoadedProject, PackageId, PackageSources, is_test_source};
use crate::source::{FileId, SourceDb, Span};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Reuses typed validation independently for ordinary and test sources of each package.
/// Entries are process-local and never expose their snapshot-local source or type IDs.
#[derive(Debug, Default)]
pub struct SemanticCache {
    entries: BTreeMap<(String, bool), Entry>,
    #[cfg(test)]
    recomputed: Vec<(String, bool)>,
}

#[derive(Debug, PartialEq, Eq)]
struct Inputs {
    root: PathBuf,
    ignored_paths: Vec<String>,
    packages: Vec<PackageInput>,
    dependencies: crate::roles::DependencyInputs,
}

#[derive(Debug, PartialEq, Eq)]
struct PackageInput {
    logical: String,
    directory: Option<PathBuf>,
    files: Vec<(
        PathBuf,
        String,
        Option<String>,
        Option<crate::roles::SourceRoles>,
    )>,
}

#[derive(Debug)]
struct Entry {
    inputs: Inputs,
    sources: SourceDb,
    // Keep typed results together with their own type universe. They are never
    // spliced into a different project's HIR or interpreted with new FileIds.
    result: Result<(hir::Project, crate::mir::Project), FrontendError>,
}

impl SemanticCache {
    /// Validate the current snapshot, rebinding all diagnostic labels by source path.
    pub fn validate(&mut self, loaded: &LoadedProject) -> Result<(), FrontendError> {
        #[cfg(test)]
        self.recomputed.clear();
        let ast = crate::ond::parse_project(loaded)?;
        validate_test_signatures(loaded, &ast)?;
        // Definition-time checks of names hidden by test visibility are cheap,
        // and must reflect added/removed test declarations even on normal hits.
        let mut diagnostics = validate_generic_body_names(loaded, &ast).into_vec();
        diagnostics.extend(super::validation::import_cycles(&ast).into_vec());
        let mut current = BTreeSet::new();
        for package in &ast.packages {
            for test in [false, true] {
                if test
                    && !package
                        .files
                        .iter()
                        .any(|file| is_test_source(loaded.sources.file(file.file_id).path()))
                {
                    continue;
                }
                let key = (package.logical_path.clone(), test);
                current.insert(key.clone());
                let inputs = collect_inputs(loaded, &ast, &package.logical_path, test);
                if self
                    .entries
                    .get(&key)
                    .is_none_or(|entry| entry.inputs != inputs)
                {
                    let entry = validate_entry(inputs, &key);
                    self.entries.insert(key.clone(), entry);
                    #[cfg(test)]
                    self.recomputed.push(key.clone());
                }
                let entry = &self.entries[&key];
                if let Err(error) = &entry.result {
                    for diagnostic in error.diagnostics() {
                        let mut diagnostic = diagnostic.clone();
                        rebind_span(
                            &mut diagnostic.primary.span,
                            &entry.sources,
                            &loaded.sources,
                        );
                        for label in &mut diagnostic.related {
                            rebind_span(&mut label.span, &entry.sources, &loaded.sources);
                        }
                        if !diagnostics.contains(&diagnostic) {
                            diagnostics.push(diagnostic);
                        }
                    }
                }
            }
        }
        self.entries.retain(|key, _| current.contains(key));
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(FrontendError::new(diagnostics))
        }
    }
}

fn collect_inputs(loaded: &LoadedProject, ast: &ast::Project, owner: &str, test: bool) -> Inputs {
    let mut pending = vec![owner.to_string()];
    let mut visited = BTreeSet::new();
    let mut packages = Vec::new();
    while let Some(logical) = pending.pop() {
        if !visited.insert(logical.clone()) {
            continue;
        }
        let package = ast
            .packages
            .iter()
            .find(|package| package.logical_path == logical);
        let files = package
            .into_iter()
            .flat_map(|package| &package.files)
            .filter(|file| {
                !is_test_source(loaded.sources.file(file.file_id).path())
                    || (logical == owner && test)
            })
            .collect::<Vec<_>>();
        for import in files.iter().flat_map(|file| &file.imports) {
            if let Ok(path) = crate::string_literal::import_path(&import.path) {
                pending.push(path);
            }
        }
        let mut files = files
            .iter()
            .map(|file| {
                let source = loaded.sources.file(file.file_id);
                (
                    source.path().to_path_buf(),
                    source.text().to_string(),
                    source.display_path().map(str::to_string),
                    loaded.sources.roles(file.file_id).cloned(),
                )
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        packages.push(PackageInput {
            directory: loaded
                .packages
                .iter()
                .find(|package| package.logical_path == logical)
                .map(|package| package.directory.clone()),
            logical,
            files,
        });
    }
    packages.sort_by(|a, b| a.logical.cmp(&b.logical));
    Inputs {
        root: loaded.root.clone(),
        ignored_paths: loaded.ignored_paths.clone(),
        packages,
        dependencies: loaded.sources.dependency_inputs().clone(),
    }
}

fn validate_entry(inputs: Inputs, key: &(String, bool)) -> Entry {
    let mut loaded = LoadedProject {
        root: inputs.root.clone(),
        manifest_path: inputs.root.join("ond.toml"),
        ignored_paths: inputs.ignored_paths.clone(),
        sources: SourceDb::default(),
        packages: Vec::new(),
    };
    loaded
        .sources
        .set_dependency_inputs(inputs.dependencies.clone());
    let mut selected = BTreeSet::new();
    for package in &inputs.packages {
        let Some(directory) = &package.directory else {
            continue;
        };
        let files = package
            .files
            .iter()
            .map(|(path, text, display_path, roles)| {
                let id = if let Some(display_path) = display_path {
                    loaded.sources.add_file_with_display_path(
                        path.clone(),
                        text.clone(),
                        display_path.clone(),
                    )
                } else {
                    loaded.sources.add_file(path.clone(), text.clone())
                };
                if let Some(roles) = roles {
                    loaded.sources.set_roles(id, roles.clone());
                }
                if package.logical == key.0 && is_test_source(path) == key.1 {
                    selected.insert(id);
                }
                id
            })
            .collect();
        loaded.packages.push(PackageSources {
            id: PackageId(loaded.packages.len()),
            logical_path: package.logical.clone(),
            directory: directory.clone(),
            files,
        });
    }
    let result = crate::ond::parse_project(&loaded).and_then(|ast| {
        let hir = lower_context_to_hir(&loaded, &ast, Some(&selected))?;
        let mir = crate::pipeline::lower_to_mir(&hir)?;
        // Package validation has external ordinary function/global references;
        // whole-project linkage checks belong to compile_loaded_project.
        for function in mir.packages.iter().flat_map(|package| &package.functions) {
            crate::mir::validate_function(&mir.types, function).map_err(|errors| {
                FrontendError::new(
                    errors
                        .into_iter()
                        .map(|error| {
                            Diagnostic::error(
                                Span::synthetic(),
                                format!(
                                    "internal MIR validation error in {}: {}",
                                    error.function, error.message
                                ),
                            )
                        })
                        .collect(),
                )
            })?;
        }
        Ok((hir, mir))
    });
    Entry {
        inputs,
        sources: loaded.sources,
        result,
    }
}

fn rebind_span(span: &mut Span, old: &SourceDb, current: &SourceDb) {
    if span.is_synthetic() {
        return;
    }
    let path = old.file(span.file).path();
    // All entry sources are part of its content key and the current snapshot.
    span.file = FileId(
        current
            .files()
            .iter()
            .position(|file| file.path() == path)
            .expect("cached diagnostic source must be present"),
    );
}

#[cfg(test)]
mod tests;
