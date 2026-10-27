use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use compiler::ir::ast;
use compiler::source::{SourceDb, Span};
use serde_json::Value;

use crate::completion::{
    CompletionScope, append_builtin_function_completion_items,
    append_builtin_type_completion_items, append_import_completion_items,
    append_indexed_symbol_completion_items, append_local_completion_items,
    append_top_level_completion_items, append_top_level_variable_completion_items,
    completion_qualifier, completion_text_with_placeholder, completion_text_with_value_placeholder,
    dedupe_completion_items, find_completion_selector, import_path_completion_context,
    import_path_completion_items, keyword_completion_items, package_member_completion_items,
    package_member_completion_items_for_alias, project_completion_base_has_known_type,
    resolve_completion_methods, resolve_completion_struct_fields,
    resolve_project_completion_methods, resolve_project_completion_struct_fields,
    struct_field_completion_items,
};
use crate::hover::{hover_for_file, hover_for_project};
use crate::navigation::{definition_for_file, definition_for_project};
use crate::protocol::{
    TextDocumentContentChangeEvent, cursor_is_in_comment, lsp_offset, publish_diagnostics,
};
#[path = "project_loading.rs"]
mod project_loading;
use crate::semantic_tokens::{
    indexed_semantic_tokens, package_semantic_tokens, project_semantic_tokens,
};
use crate::symbol_index::ProjectSymbolIndex;
use crate::symbols::{document_symbols_for_file, to_lsp_diagnostic};
use crate::util::{
    find_project_root, log_stderr, normalize_path, path_from_uri, same_path, uri_from_path,
};

#[derive(Debug, Default)]
pub struct ServerState {
    documents: BTreeMap<PathBuf, DocumentState>,
    published_project_uris: BTreeMap<PathBuf, BTreeSet<String>>,
    symbol_indexes: BTreeMap<PathBuf, ProjectSymbolIndex>,
    semantic_caches: BTreeMap<PathBuf, compiler::SemanticCache>,
}

#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
}

impl ServerState {
    pub fn open_document(
        &mut self,
        uri: String,
        text: String,
        writer: &mut impl Write,
    ) -> Result<(), String> {
        let path = normalize_path(path_from_uri(&uri)?);
        self.documents.insert(path, DocumentState { text });
        self.validate_uri(&uri, writer)
    }

    pub fn change_document(
        &mut self,
        uri: String,
        changes: Vec<TextDocumentContentChangeEvent>,
        writer: &mut impl Write,
    ) -> Result<(), String> {
        let path = normalize_path(path_from_uri(&uri)?);
        self.apply_content_changes(path, changes)?;
        self.validate_uri(&uri, writer)
    }

    pub fn close_document(&mut self, uri: &str, writer: &mut impl Write) -> Result<(), String> {
        let path = normalize_path(path_from_uri(uri)?);
        self.documents.remove(&path);
        self.validate_uri(uri, writer)
    }

    pub fn validate_uri(&mut self, uri: &str, writer: &mut impl Write) -> Result<(), String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(project_root) = find_project_root(&path) {
            match self.validate_project(&project_root, writer) {
                Ok(()) => Ok(()),
                Err(message) => {
                    log_stderr(&format!("project validation failed: {message}"));
                    publish_diagnostics(writer, uri, vec![project_error_diagnostic(&message)])
                }
            }
        } else {
            self.validate_single_file(uri, &path, writer)
        }
    }

    fn validate_project(&mut self, root: &Path, writer: &mut impl Write) -> Result<(), String> {
        let overrides = self.collect_overrides();
        log_stderr(&format!(
            "validate_project: root={} open_docs={}",
            root.display(),
            overrides.len()
        ));
        let project = self.load_project(root, &overrides).inspect_err(|_| {
            self.symbol_indexes
                .remove(&normalize_path(root.to_path_buf()));
        })?;
        self.refresh_symbol_index(root, project.clone());
        let mut diagnostics_by_path = BTreeMap::<PathBuf, Vec<Value>>::new();

        let cache = self
            .semantic_caches
            .entry(normalize_path(root.to_path_buf()))
            .or_default();
        let validation_error = match catch_unwind(AssertUnwindSafe(|| cache.validate(&project))) {
            Ok(result) => result.err(),
            Err(_) => Some(compiler::FrontendError::new(vec![
                compiler::Diagnostic::error(
                    Span::synthetic(),
                    "compiler panicked while validating the document",
                ),
            ])),
        };
        if let Some(error) = validation_error {
            for diagnostic in error.diagnostics() {
                let path = if diagnostic.primary.span.is_synthetic() {
                    project_loading::synthetic_diagnostic_path(root, &project)
                } else {
                    project
                        .sources
                        .file(diagnostic.primary.span.file)
                        .path()
                        .to_path_buf()
                };
                diagnostics_by_path
                    .entry(normalize_path(path))
                    .or_default()
                    .push(to_lsp_diagnostic(diagnostic, &project.sources));
            }
        }

        let root = normalize_path(root.to_path_buf());
        let current_uris = self.project_source_uris(&root, &project)?;
        let mut uris = current_uris.clone();
        if let Some(previous_uris) = self.published_project_uris.get(&root) {
            uris.extend(previous_uris.iter().cloned());
        }
        for uri in uris {
            let path = normalize_path(path_from_uri(&uri)?);
            let diagnostics = diagnostics_by_path.remove(&path).unwrap_or_default();
            log_stderr(&format!(
                "publishDiagnostics: uri={} count={}",
                uri,
                diagnostics.len()
            ));
            publish_diagnostics(writer, &uri, diagnostics)?;
        }
        self.published_project_uris.insert(root, current_uris);

        Ok(())
    }

    fn validate_single_file(
        &self,
        uri: &str,
        path: &Path,
        writer: &mut impl Write,
    ) -> Result<(), String> {
        let Some(document) = self.documents.get(path) else {
            publish_diagnostics(writer, uri, Vec::new())?;
            return Ok(());
        };

        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path.to_path_buf(), document.text.clone());
        let diagnostics = match compiler::parse_source_file(file_id, &document.text) {
            Ok(_) => Vec::new(),
            Err(diagnostics) => diagnostics
                .into_iter()
                .map(|diagnostic| to_lsp_diagnostic(&diagnostic, &sources))
                .collect(),
        };
        log_stderr(&format!(
            "publishDiagnostics(single): uri={} count={}",
            uri,
            diagnostics.len()
        ));
        publish_diagnostics(writer, uri, diagnostics)
    }

    fn collect_overrides(&self) -> BTreeMap<PathBuf, String> {
        self.documents
            .iter()
            .map(|(path, document)| (path.clone(), document.text.clone()))
            .collect()
    }

    fn load_project(
        &self,
        root: &Path,
        overrides: &BTreeMap<PathBuf, String>,
    ) -> Result<compiler::LoadedProject, String> {
        project_loading::load_project(root, overrides)
    }

    pub(crate) fn load_current_project(
        &self,
        root: &Path,
    ) -> Result<compiler::LoadedProject, String> {
        self.load_project(root, &self.collect_overrides())
    }

    pub(crate) fn document_text(&self, path: &Path) -> Option<&str> {
        self.documents
            .get(path)
            .map(|document| document.text.as_str())
    }

    pub fn watched_files_changed(&mut self, writer: &mut impl Write) -> Result<(), String> {
        let roots = self
            .documents
            .keys()
            .filter_map(|path| find_project_root(path))
            .map(normalize_path)
            .collect::<BTreeSet<_>>();
        for root in roots {
            if let Err(message) = self.validate_project(&root, writer) {
                log_stderr(&format!("project validation failed: {message}"));
                for uri in self.project_document_uris(&root)? {
                    publish_diagnostics(writer, &uri, vec![project_error_diagnostic(&message)])?;
                }
            }
        }
        Ok(())
    }

    fn symbol_index(
        &mut self,
        root: &Path,
        overrides: &BTreeMap<PathBuf, String>,
    ) -> Result<ProjectSymbolIndex, String> {
        let project = self.load_project(root, overrides).inspect_err(|_| {
            self.symbol_indexes
                .remove(&normalize_path(root.to_path_buf()));
        })?;
        Ok(self.refresh_symbol_index(root, project))
    }

    fn refresh_symbol_index(
        &mut self,
        root: &Path,
        project: compiler::LoadedProject,
    ) -> ProjectSymbolIndex {
        let key = normalize_path(root.to_path_buf());
        let index = ProjectSymbolIndex::refresh(project, self.symbol_indexes.get(&key));
        self.symbol_indexes.insert(key, index.clone());
        index
    }

    fn apply_content_changes(
        &mut self,
        path: PathBuf,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), String> {
        let mut text = self
            .documents
            .get(&path)
            .map(|document| document.text.clone())
            .unwrap_or_default();

        for change in changes {
            if let Some(range) = change.range {
                let start = lsp_offset(&text, &range.start)?;
                let end = lsp_offset(&text, &range.end)?;
                if start > end {
                    return Err("content change range is reversed".to_string());
                }
                text.replace_range(start..end, &change.text);
            } else {
                text = change.text;
            }
        }

        self.documents.insert(path, DocumentState { text });
        Ok(())
    }

    pub fn completions(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(document) = self.documents.get(&path) {
            if cursor_is_in_comment(&document.text, line, character) {
                log_stderr("completion: cursor is inside a comment");
                return Ok(Vec::new());
            }
        }
        let import_context = self.documents.get(&path).and_then(|document| {
            import_path_completion_context(&document.text, line, character)
                .map(|context| (document.text.clone(), context))
        });
        if let Some((text, context)) = import_context {
            let Some(project_root) = find_project_root(&path) else {
                return Ok(Vec::new());
            };
            return self
                .project_import_path_completions(&path, &project_root, &text, &context)
                .inspect_err(|_| {
                    self.symbol_indexes.remove(&normalize_path(project_root));
                });
        }
        if let Some(project_root) = find_project_root(&path) {
            self.project_completions(&path, &project_root, line, character)
        } else {
            self.single_file_completions(&path, line, character)
        }
    }

    fn project_import_path_completions(
        &mut self,
        path: &Path,
        root: &Path,
        text: &str,
        context: &crate::completion::ImportPathCompletion,
    ) -> Result<Vec<Value>, String> {
        let overrides = self.collect_overrides();
        let index = self.symbol_index(root, &overrides)?;
        let project = &index.loaded;
        let current_package = project.packages.iter().find_map(|package| {
            package
                .files
                .iter()
                .any(|file| same_path(project.sources.file(*file).path(), path))
                .then_some(package.logical_path.as_str())
        });
        let mut paths = project
            .packages
            .iter()
            .map(|package| package.logical_path.clone())
            .collect::<BTreeSet<_>>();
        let dependencies =
            ond_cli::resolve_dependencies(root).map_err(|diagnostics| diagnostics.join("\n"))?;
        for source_name in dependencies.sources.keys() {
            if let Some((package, _)) = source_name.rsplit_once('/') {
                paths.insert(package.to_string());
            }
        }
        paths.remove(".");
        if let Some(current_package) = current_package {
            paths.remove(current_package);
        }
        Ok(import_path_completion_items(
            text,
            context,
            paths.iter().map(String::as_str),
        ))
    }

    pub fn semantic_tokens(&self, uri: &str) -> Result<Vec<u32>, String> {
        let path = normalize_path(path_from_uri(uri)?);
        let Some(document) = self.documents.get(&path) else {
            return Ok(Vec::new());
        };
        if let Some(root) = find_project_root(&path)
            && let Some(index) = self.symbol_indexes.get(&normalize_path(root))
        {
            let mut indexed_file_found = false;
            for package in &index.ast.packages {
                if let Some(file) = package
                    .files
                    .iter()
                    .find(|file| same_path(index.loaded.sources.file(file.file_id).path(), &path))
                {
                    indexed_file_found = true;
                    if index.loaded.sources.file(file.file_id).text() == document.text {
                        return Ok(crate::semantic_tokens::snapshot_semantic_tokens(
                            &document.text,
                            &index.ast,
                            package,
                            file,
                            &index.loaded.sources,
                        ));
                    }
                }
            }
            if indexed_file_found {
                return Ok(indexed_semantic_tokens(&document.text, &index.ast));
            }
        }

        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path, document.text.clone());
        if let Ok(file) = compiler::parse_source_file(file_id, &document.text) {
            let package = ast::Package {
                logical_path: ".".to_string(),
                files: vec![file.clone()],
            };
            let project = ast::Project {
                packages: vec![package.clone()],
            };
            return Ok(project_semantic_tokens(
                &document.text,
                &project,
                &package,
                &file,
            ));
        }
        Ok(package_semantic_tokens(&document.text))
    }

    fn project_completions(
        &mut self,
        path: &Path,
        root: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>, String> {
        let base_overrides = self.collect_overrides();
        let cached_index = self
            .symbol_index(root, &base_overrides)?
            .for_source(path)
            .for_position(path, line, character);
        let mut overrides = base_overrides;
        let completion_marker = self.documents.get(path).and_then(|document| {
            completion_text_with_placeholder(&document.text, line, character).map(
                |(text, marker)| {
                    overrides.insert(path.to_path_buf(), text);
                    marker
                },
            )
        });
        let value_placeholder = if completion_marker.is_none() {
            self.documents.get(path).and_then(|document| {
                completion_text_with_value_placeholder(&document.text, line, character)
            })
        } else {
            None
        };
        if let Some(text) = &value_placeholder {
            overrides.insert(path.to_path_buf(), text.clone());
        }
        let repaired_index = if completion_marker.is_some() || value_placeholder.is_some() {
            self.load_project(root, &overrides)
                .ok()
                .and_then(|project| {
                    let repaired = ProjectSymbolIndex::refresh(project, Some(&cached_index))
                        .for_source(path)
                        .for_position(path, line, character);
                    let current_file_parsed = !repaired.file_failed_to_parse(path)
                        && repaired
                            .ast
                            .packages
                            .iter()
                            .flat_map(|package| &package.files)
                            .any(|file| {
                                same_path(repaired.loaded.sources.file(file.file_id).path(), path)
                            });
                    if current_file_parsed {
                        Some(repaired)
                    } else {
                        None
                    }
                })
        } else {
            None
        };
        let index = if cached_index
            .ast
            .packages
            .iter()
            .flat_map(|package| &package.files)
            .any(|file| same_path(cached_index.loaded.sources.file(file.file_id).path(), path))
        {
            cached_index
        } else {
            repaired_index.clone().unwrap_or(cached_index)
        };
        let completion_qualifier = self
            .documents
            .get(path)
            .and_then(|document| completion_qualifier(&document.text, line, character));
        let cursor_offset = self.documents.get(path).and_then(|document| {
            lsp_offset(
                &document.text,
                &crate::protocol::Position { line, character },
            )
            .ok()
        });
        let mut items = keyword_completion_items();
        append_builtin_type_completion_items(&mut items);

        let contains_current_file = index
            .ast
            .packages
            .iter()
            .flat_map(|package| &package.files)
            .any(|file| same_path(index.loaded.sources.file(file.file_id).path(), path));
        if !contains_current_file {
            if completion_marker.is_some() {
                return Ok(Vec::new());
            }
            append_builtin_function_completion_items(&mut items);
            return Ok(items);
        }
        let project = &index.loaded;
        let ast = &index.ast;

        for package in &ast.packages {
            let contains_file = package
                .files
                .iter()
                .any(|file| same_path(project.sources.file(file.file_id).path(), path));
            if !contains_file {
                continue;
            }

            for file in &package.files {
                append_import_completion_items(&mut items, file);
                append_top_level_variable_completion_items(&mut items, file);
            }
            append_indexed_symbol_completion_items(&mut items, &index, &package.logical_path);

            if let Some(current_file) = package
                .files
                .iter()
                .find(|file| same_path(project.sources.file(file.file_id).path(), path))
            {
                if let Some(marker) = completion_marker.as_deref() {
                    let selector_index = repaired_index.as_ref();
                    if let Some(selector_index) = selector_index {
                        let selector_package = selector_index.ast.packages.iter().find(|package| {
                            package.files.iter().any(|file| {
                                same_path(
                                    selector_index.loaded.sources.file(file.file_id).path(),
                                    path,
                                )
                            })
                        });
                        let selector_file = selector_package.and_then(|package| {
                            package.files.iter().find(|file| {
                                same_path(
                                    selector_index.loaded.sources.file(file.file_id).path(),
                                    path,
                                )
                            })
                        });
                        if let Some((selector_package, selector_file)) =
                            selector_package.zip(selector_file)
                            && let Some((base, function)) =
                                find_completion_selector(selector_file, marker)
                        {
                            let methods = resolve_project_completion_methods(
                                &selector_index.ast,
                                selector_package,
                                selector_file,
                                function,
                                &base,
                            );
                            if let Some(fields) = resolve_project_completion_struct_fields(
                                &selector_index.ast,
                                selector_package,
                                selector_file,
                                function,
                                &base,
                            ) {
                                let mut items = struct_field_completion_items(&fields);
                                items.extend(methods.unwrap_or_default());
                                return Ok(items);
                            }
                            if let Some(methods) = methods
                                && !methods.is_empty()
                            {
                                return Ok(methods);
                            }
                            if project_completion_base_has_known_type(
                                &selector_index.ast,
                                selector_package,
                                selector_file,
                                function,
                                &base,
                            ) {
                                return Ok(Vec::new());
                            }
                            if let Some(items) = package_member_completion_items(
                                selector_index,
                                selector_file,
                                &base,
                            ) {
                                return Ok(items);
                            }
                        }
                    }
                    if let Some(alias) = completion_qualifier.as_deref()
                        && let Some(items) =
                            package_member_completion_items_for_alias(&index, current_file, alias)
                    {
                        return Ok(items);
                    }
                    if let (Some(name), Some(offset), Some(document)) = (
                        completion_qualifier.as_deref(),
                        cursor_offset,
                        self.documents.get(path),
                    ) && let Some((selector_index, selector_package, selector_file)) =
                        repaired_index
                            .as_ref()
                            .and_then(|repaired| {
                                repaired.ast.packages.iter().find_map(|package| {
                                    package
                                        .files
                                        .iter()
                                        .find(|file| {
                                            same_path(
                                                repaired.loaded.sources.file(file.file_id).path(),
                                                path,
                                            )
                                        })
                                        .map(|file| (repaired, package, file))
                                })
                            })
                            .or_else(|| {
                                (!index.file_failed_to_parse(path)).then_some((
                                    &index,
                                    package,
                                    current_file,
                                ))
                            })
                    {
                        let span = Span::new(selector_file.file_id, offset, offset);
                        let base = ast::Expr::Name(ast::Path {
                            segments: vec![ast::Ident {
                                name: name.to_string(),
                                span,
                            }],
                            span,
                        });
                        let function = completion_function_for_incomplete_member(
                            selector_file,
                            &document.text,
                            offset,
                        );
                        let methods = resolve_project_completion_methods(
                            &selector_index.ast,
                            selector_package,
                            selector_file,
                            function,
                            &base,
                        );
                        if let Some(fields) = resolve_project_completion_struct_fields(
                            &selector_index.ast,
                            selector_package,
                            selector_file,
                            function,
                            &base,
                        ) {
                            let mut items = struct_field_completion_items(&fields);
                            items.extend(methods.unwrap_or_default());
                            return Ok(items);
                        }
                        if let Some(methods) = methods
                            && !methods.is_empty()
                        {
                            return Ok(methods);
                        }
                    }
                    return Ok(Vec::new());
                }

                let completion_file = repaired_index
                    .as_ref()
                    .and_then(|repaired| {
                        repaired.ast.packages.iter().find_map(|package| {
                            package.files.iter().find(|file| {
                                same_path(repaired.loaded.sources.file(file.file_id).path(), path)
                            })
                        })
                    })
                    .or_else(|| (!index.file_failed_to_parse(path)).then_some(current_file));
                let offset = cursor_offset.unwrap_or_else(|| {
                    project
                        .sources
                        .file(current_file.file_id)
                        .offset_at_line_col(line as usize + 1, character as usize + 1)
                });
                if let Some(completion_file) = completion_file {
                    append_local_completion_items(&mut items, completion_file, offset);
                }
            }

            break;
        }

        if completion_marker.is_some() {
            return Ok(Vec::new());
        }
        append_builtin_function_completion_items(&mut items);
        dedupe_completion_items(items)
    }

    fn single_file_completions(
        &self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>, String> {
        let Some(document) = self.documents.get(path) else {
            let mut items = keyword_completion_items();
            append_builtin_type_completion_items(&mut items);
            append_builtin_function_completion_items(&mut items);
            return Ok(items);
        };

        let completion = completion_text_with_placeholder(&document.text, line, character);
        let value_completion = if completion.is_none() {
            completion_text_with_value_placeholder(&document.text, line, character)
        } else {
            None
        };
        let text = completion
            .as_ref()
            .map(|(text, _)| text.as_str())
            .or(value_completion.as_deref())
            .unwrap_or(&document.text);
        let marker = completion.as_ref().map(|(_, marker)| *marker);

        let mut items = keyword_completion_items();
        append_builtin_type_completion_items(&mut items);
        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path.to_path_buf(), text.to_string());
        let Ok(file) = compiler::parse_source_file(file_id, text) else {
            if marker.is_some() {
                return Ok(Vec::new());
            }
            append_builtin_function_completion_items(&mut items);
            return Ok(items);
        };

        if let Some(marker) = marker {
            if let Some((base, function)) = find_completion_selector(&file, marker) {
                let package = ast::Package {
                    logical_path: ".".to_string(),
                    files: vec![file.clone()],
                };
                if let Some(fields) = resolve_completion_struct_fields(&package, function, &base) {
                    let mut items = struct_field_completion_items(&fields);
                    items.extend(
                        resolve_completion_methods(&package, function, &base).unwrap_or_default(),
                    );
                    return Ok(items);
                }
                if let Some(methods) = resolve_completion_methods(&package, function, &base)
                    && !methods.is_empty()
                {
                    return Ok(methods);
                }
            }
            return Ok(Vec::new());
        }

        append_import_completion_items(&mut items, &file);
        append_top_level_completion_items(&mut items, &file);
        let source = sources.file(file_id);
        let offset = source.offset_at_line_col(line as usize + 1, character as usize + 1);
        append_local_completion_items(&mut items, &file, offset);
        append_builtin_function_completion_items(&mut items);
        dedupe_completion_items(items)
    }

    pub fn document_symbols(&mut self, uri: &str) -> Result<Value, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(project_root) = find_project_root(&path) {
            self.project_document_symbols(&path, &project_root)
        } else {
            self.single_file_document_symbols(&path)
        }
    }

    pub fn hover(&mut self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(project_root) = find_project_root(&path) {
            self.project_hover(&path, &project_root, line, character)
        } else {
            self.single_file_hover(&path, line, character)
        }
    }

    pub fn definition(&mut self, uri: &str, line: u32, character: u32) -> Result<Value, String> {
        let path = normalize_path(path_from_uri(uri)?);
        if let Some(project_root) = find_project_root(&path) {
            self.project_definition(&path, &project_root, line, character)
        } else {
            self.single_file_definition(&path, line, character)
        }
    }

    fn project_definition(
        &mut self,
        path: &Path,
        root: &Path,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        let overrides = self.collect_overrides();
        let index = self
            .symbol_index(root, &overrides)?
            .for_source(path)
            .for_position(path, line, character);
        let project = &index.loaded;
        let ast = &index.ast;
        for package in &ast.packages {
            if let Some(file) = package
                .files
                .iter()
                .find(|file| same_path(project.sources.file(file.file_id).path(), path))
            {
                return Ok(definition_for_project(
                    ast,
                    package,
                    file,
                    &project.sources,
                    &crate::protocol::Position { line, character },
                )
                .unwrap_or(Value::Null));
            }
        }
        Ok(Value::Null)
    }

    fn single_file_definition(
        &mut self,
        path: &Path,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        let Some(document) = self.documents.get(path) else {
            return Ok(Value::Null);
        };
        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path.to_path_buf(), document.text.clone());
        let file = match compiler::parse_source_file(file_id, &document.text) {
            Ok(file) => file,
            Err(_) => return Ok(Value::Null),
        };
        Ok(definition_for_file(
            &file,
            &sources,
            &crate::protocol::Position { line, character },
        )
        .unwrap_or(Value::Null))
    }

    fn project_hover(
        &mut self,
        path: &Path,
        root: &Path,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        let overrides = self.collect_overrides();
        let index = self
            .symbol_index(root, &overrides)?
            .for_source(path)
            .for_position(path, line, character);
        if index.file_failed_to_parse(path) {
            return Ok(Value::Null);
        }
        let project = &index.loaded;
        let ast = &index.ast;

        for package in &ast.packages {
            for file in &package.files {
                let source = project.sources.file(file.file_id);
                if same_path(source.path(), path) {
                    return Ok(hover_for_project(
                        ast,
                        package,
                        file,
                        &project.sources,
                        &crate::protocol::Position { line, character },
                    )
                    .unwrap_or(Value::Null));
                }
            }
        }

        Ok(Value::Null)
    }

    fn single_file_hover(&self, path: &Path, line: u32, character: u32) -> Result<Value, String> {
        let Some(document) = self.documents.get(path) else {
            return Ok(Value::Null);
        };

        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path.to_path_buf(), document.text.clone());
        let file = match compiler::parse_source_file(file_id, &document.text) {
            Ok(file) => file,
            Err(_) => return Ok(Value::Null),
        };
        Ok(hover_for_file(
            &file,
            sources.file(file_id),
            &crate::protocol::Position { line, character },
        )
        .unwrap_or(Value::Null))
    }

    fn project_document_symbols(&mut self, path: &Path, root: &Path) -> Result<Value, String> {
        let overrides = self.collect_overrides();
        let index = self.symbol_index(root, &overrides)?.for_source(path);
        let project = &index.loaded;

        for package in &index.ast.packages {
            for file in &package.files {
                let source_path = project.sources.file(file.file_id).path();
                if same_path(source_path, path) {
                    return Ok(document_symbols_for_file(file, &project.sources));
                }
            }
        }

        Ok(Value::Array(Vec::new()))
    }

    fn single_file_document_symbols(&self, path: &Path) -> Result<Value, String> {
        let Some(document) = self.documents.get(path) else {
            return Ok(Value::Array(Vec::new()));
        };

        let mut sources = SourceDb::default();
        let file_id = sources.add_file(path.to_path_buf(), document.text.clone());
        let file = match compiler::parse_source_file(file_id, &document.text) {
            Ok(file) => file,
            Err(_) => return Ok(Value::Array(Vec::new())),
        };
        Ok(document_symbols_for_file(&file, &sources))
    }

    fn project_document_uris(&self, root: &Path) -> Result<Vec<String>, String> {
        let mut uris = BTreeSet::new();
        for path in self.documents.keys().filter(|path| {
            find_project_root(path).is_some_and(|document_root| same_path(&document_root, root))
        }) {
            uris.insert(uri_from_path(path)?);
        }
        Ok(uris.into_iter().collect())
    }

    fn project_source_uris(
        &self,
        root: &Path,
        project: &compiler::LoadedProject,
    ) -> Result<BTreeSet<String>, String> {
        let mut uris = BTreeSet::new();
        for source in project.sources.files() {
            let path = normalize_path(source.path().to_path_buf());
            if path.is_file()
                && find_project_root(&path).is_some_and(|source_root| same_path(&source_root, root))
            {
                uris.insert(uri_from_path(&path)?);
            }
        }
        for path in self.documents.keys().filter(|path| {
            find_project_root(path).is_some_and(|document_root| same_path(&document_root, root))
        }) {
            uris.insert(uri_from_path(path)?);
        }
        Ok(uris)
    }
}

fn completion_function_for_incomplete_member<'a>(
    file: &'a ast::File,
    text: &str,
    offset: usize,
) -> Option<CompletionScope<'a>> {
    let function_name = incomplete_completion_function_name(text, offset);
    let functions = || {
        file.decls.iter().filter_map(|decl| match decl {
            ast::TopLevelDecl::Func(function) => Some(function),
            _ => None,
        })
    };
    function_name
        .and_then(|name| {
            functions()
                .filter(|function| function.name.name == name)
                .max_by_key(|function| function.span.start)
                .map(CompletionScope::Function)
        })
        .or_else(|| {
            file.decls
                .iter()
                .filter_map(|decl| match decl {
                    ast::TopLevelDecl::Func(function) if function.span.start <= offset => {
                        Some((function.span.start, CompletionScope::Function(function)))
                    }
                    ast::TopLevelDecl::Operator(operator) if operator.span.start <= offset => {
                        Some((operator.span.start, CompletionScope::Operator(operator)))
                    }
                    _ => None,
                })
                .max_by_key(|(start, _)| *start)
                .map(|(_, scope)| scope)
        })
}

fn incomplete_completion_function_name(text: &str, offset: usize) -> Option<&str> {
    let prefix = text.get(..offset)?;
    let bytes = prefix.as_bytes();
    let mut search_end = bytes.len();
    while let Some(relative) = prefix[..search_end].rfind("func") {
        let before_is_identifier = relative > 0 && is_identifier_byte(bytes[relative - 1]);
        let after = relative + "func".len();
        let after_is_identifier = after < bytes.len() && is_identifier_byte(bytes[after]);
        if !before_is_identifier && !after_is_identifier {
            let mut cursor = after;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'(' {
                let mut depth = 0usize;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                cursor += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    cursor += 1;
                }
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
            }
            let start = cursor;
            while cursor < bytes.len() && is_identifier_byte(bytes[cursor]) {
                cursor += 1;
            }
            if start < cursor {
                return prefix.get(start..cursor);
            }
        }
        search_end = relative;
    }
    None
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn project_error_diagnostic(message: &str) -> Value {
    serde_json::json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 0, "character": 0 }
        },
        "severity": 1,
        "source": "ond-lsp",
        "message": message
    })
}

#[cfg(test)]
mod project_contract_tests;
#[cfg(test)]
mod role_tests;
#[cfg(test)]
impl ServerState {
    fn with_dependencies(root: &Path, roots: Vec<PathBuf>) -> Self {
        let manifest_path = root.join("ond.toml");
        let mut text = std::fs::read_to_string(&manifest_path).unwrap();
        if !text.contains("[dependencies]") {
            text.push_str("\n[dependencies]\n");
        }
        for directory in roots {
            if same_path(root, &directory) {
                continue;
            }
            for entry in std::fs::read_dir(directory).unwrap().map(Result::unwrap) {
                if !entry.path().is_dir() {
                    continue;
                }
                let name = entry.file_name().into_string().unwrap();
                std::fs::write(
                    entry.path().join("ond.toml"),
                    format!("[project]\nname={name:?}\n"),
                )
                .unwrap();
                let path = entry
                    .path()
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                text.push_str(&format!("{name}={{path={path:?}}}\n"));
            }
        }
        std::fs::write(manifest_path, text).unwrap();
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use serde_json::Value;

    use super::{DocumentState, ServerState};
    use crate::protocol::TextDocumentContentChangeEvent;
    use crate::util::{normalize_path, same_path, uri_from_path};

    pub(super) fn published_diagnostics(output: &[u8], uri: &str) -> Vec<Value> {
        let mut messages = Vec::new();
        let mut offset = 0;
        while offset < output.len() {
            let header_end = output[offset..]
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| offset + index)
                .unwrap();
            let header = std::str::from_utf8(&output[offset..header_end]).unwrap();
            let body_length = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let body_start = header_end + 4;
            let body_end = body_start + body_length;
            let message: Value = serde_json::from_slice(&output[body_start..body_end]).unwrap();
            if message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
            {
                messages.push(message["params"]["diagnostics"].clone());
            }
            offset = body_end;
        }
        messages
    }

    fn test_project_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ond-lsp-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::canonicalize(path).unwrap()
    }

    #[test]
    fn project_diagnostics_include_closed_files_but_exclude_path_dependencies() {
        let base = test_project_root("closed-file-diagnostics");
        let root = base.join("project");
        let dependency = base.join("dependency");
        let nested_dependency = root.join(".ond/dependencies/nested");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&dependency).unwrap();
        std::fs::create_dir_all(&nested_dependency).unwrap();
        let dependency_path = dependency.to_string_lossy().replace('\\', "/");
        let nested_dependency_path = nested_dependency.to_string_lossy().replace('\\', "/");
        std::fs::write(
            root.join("ond.toml"),
            format!(
                "[project]\nname = \"lsp\"\n[dependencies]\nstandard = \"{dependency_path}\"\nnested = \"{nested_dependency_path}\"\n"
            ),
        )
        .unwrap();
        let main = root.join("main.ond");
        let closed = root.join("closed.ond");
        let external = dependency.join("broken.ond");
        let nested_external = nested_dependency.join("broken.ond");
        let source = "package main\nfunc main() {}\n";
        std::fs::write(&main, source).unwrap();
        std::fs::write(&closed, "").unwrap();
        std::fs::write(
            dependency.join("ond.toml"),
            "[project]\nname = \"standard\"\n",
        )
        .unwrap();
        std::fs::write(&external, "").unwrap();
        std::fs::write(
            nested_dependency.join("ond.toml"),
            "[project]\nname = \"nested\"\n",
        )
        .unwrap();
        std::fs::write(&nested_external, "").unwrap();

        let uri = uri_from_path(&main).unwrap();
        let closed_uri = uri_from_path(&closed).unwrap();
        let external_uri = uri_from_path(&external).unwrap();
        let nested_external_uri = uri_from_path(&nested_external).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(nested_external_uri.clone(), String::new(), &mut output)
            .unwrap();
        output.clear();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();

        let closed_diagnostics = published_diagnostics(&output, &closed_uri);
        assert_eq!(closed_diagnostics.len(), 1, "{output:?}");
        assert!(!closed_diagnostics[0].as_array().unwrap().is_empty());
        assert!(published_diagnostics(&output, &external_uri).is_empty());
        assert!(published_diagnostics(&output, &nested_external_uri).is_empty());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn closed_project_diagnostics_are_cleared_after_fix_rename_and_removal() {
        let root = test_project_root("clear-closed-diagnostics");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname = \"lsp\"\n").unwrap();
        let main = root.join("main.ond");
        let closed = root.join("closed.ond");
        let renamed = root.join("renamed.ond");
        let source = "package main\nfunc main() {}\n";
        std::fs::write(&main, source).unwrap();
        std::fs::write(&closed, "").unwrap();
        let uri = uri_from_path(&main).unwrap();
        let closed_uri = uri_from_path(&closed).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();

        let initial_diagnostics = published_diagnostics(&output, &closed_uri);
        assert_eq!(initial_diagnostics.len(), 1, "{output:?}");
        assert!(!initial_diagnostics[0].as_array().unwrap().is_empty());

        std::fs::write(&closed, "package main\n").unwrap();
        output.clear();
        state.watched_files_changed(&mut output).unwrap();
        assert_eq!(
            published_diagnostics(&output, &closed_uri),
            vec![serde_json::json!([])]
        );

        std::fs::rename(&closed, &renamed).unwrap();
        output.clear();
        state.watched_files_changed(&mut output).unwrap();
        assert_eq!(
            published_diagnostics(&output, &closed_uri),
            vec![serde_json::json!([])]
        );
        let renamed_uri = uri_from_path(&renamed).unwrap();
        assert_eq!(
            published_diagnostics(&output, &renamed_uri),
            vec![serde_json::json!([])]
        );

        std::fs::remove_file(&renamed).unwrap();
        output.clear();
        state.watched_files_changed(&mut output).unwrap();
        assert_eq!(
            published_diagnostics(&output, &renamed_uri),
            vec![serde_json::json!([])]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_document_revalidates_project_disk_and_clears_standalone_diagnostics() {
        let assert_project_close =
            |name: &str, disk_text: &str, buffer_text: &str, has_errors: bool| {
                let root = test_project_root(name);
                std::fs::write(root.join("ond.toml"), "[project]\nname = \"lsp\"\n").unwrap();
                let file = root.join("main.ond");
                std::fs::write(&file, disk_text).unwrap();
                let uri = uri_from_path(&file).unwrap();
                let mut state = ServerState::default();
                let mut output = Vec::new();
                state
                    .open_document(uri.clone(), buffer_text.to_string(), &mut output)
                    .unwrap();

                output.clear();
                state.close_document(&uri, &mut output).unwrap();
                let published = published_diagnostics(&output, &uri);
                assert_eq!(
                    published.last().unwrap().as_array().unwrap().is_empty(),
                    !has_errors
                );
                std::fs::remove_dir_all(root).unwrap();
            };

        let valid = "package main\nfunc main() {}\n";
        let invalid = "package main\nfunc main( {\n";
        // The closed document is the only open file when didClose is handled.
        assert_project_close("close-disk-invalid", invalid, invalid, true);
        assert_project_close("close-disk-valid", valid, invalid, false);
        assert_project_close("close-buffer-valid", invalid, valid, true);

        let root = test_project_root("close-standalone");
        let file = root.join("standalone.ond");
        std::fs::write(&file, "").unwrap();
        let uri = uri_from_path(&file).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), String::new(), &mut output)
            .unwrap();
        output.clear();
        state.close_document(&uri, &mut output).unwrap();
        assert_eq!(
            published_diagnostics(&output, &uri).last().unwrap(),
            &serde_json::json!([])
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_dependencies_are_loaded_for_language_features() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-manifest-dependency-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let project = base.join("project");
        let library = base.join("library");
        std::fs::create_dir_all(library.join("json")).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(
            project.join("ond.toml"),
            "[project]\nname = \"app\"\n[dependencies]\nlocal_alias = { path = \"../library\" }\n",
        )
        .unwrap();
        std::fs::write(
            project.join("main.ond"),
            "package main\nimport std_json \"standard/json\"\nfunc main(){std_json.Use()}\n",
        )
        .unwrap();
        std::fs::write(library.join("ond.toml"), "[project]\nname = \"standard\"\n").unwrap();
        std::fs::write(
            library.join("json/json.ond"),
            "package json\nfunc Use(){}\n",
        )
        .unwrap();

        let mut state = ServerState::default();
        let loaded = state.load_project(&project, &BTreeMap::new()).unwrap();
        assert!(
            loaded
                .packages
                .iter()
                .any(|package| package.logical_path == "standard/json")
        );
        let path = project.join("main.ond");
        let uri = uri_from_path(&path).unwrap();
        let source = "package main\nimport \"\"\n";
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        let completions = state.completions(&uri, 1, 8).unwrap();
        assert!(
            completions
                .iter()
                .any(|item| item["label"] == "standard/json"),
            "{completions:?}"
        );
        assert!(
            !completions
                .iter()
                .any(|item| item["label"] == "local_alias/json")
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn manifest_path_dependency_keeps_local_hover_and_document_symbols() {
        let base = normalize_path(std::env::temp_dir().join(format!(
                "ond-lsp-manifest-dependency-symbols-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
        let project = base.join("project");
        let library = base.join("library");
        std::fs::create_dir_all(library.join("io")).unwrap();
        std::fs::create_dir_all(library.join("ansi/escape_code")).unwrap();
        std::fs::create_dir_all(project.join("api")).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        let dependency_path = library.to_string_lossy().replace('\\', "/");
        std::fs::write(
            project.join("ond.toml"),
            format!(
                "[project]\nname = \"app\"\n[dependencies]\nstandard = \"{dependency_path}\"\n"
            ),
        )
        .unwrap();
        std::fs::write(project.join("main.ond"), "package main\nfunc main() {}\n").unwrap();
        std::fs::write(library.join("ond.toml"), "[project]\nname = \"standard\"\n").unwrap();
        std::fs::write(library.join("io/io.ond"), "package io\nfunc Read(){}\n").unwrap();
        std::fs::write(library.join("ansi/escape_code/api.ond"), "").unwrap();
        let path = project.join("api/driver.ond");
        std::fs::write(&path, "package api\nfunc Local() {}\n").unwrap();

        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(
                uri.clone(),
                std::fs::read_to_string(&path).unwrap(),
                &mut output,
            )
            .unwrap();
        let hover = state.hover(&uri, 1, 5).unwrap();
        let symbols = state.document_symbols(&uri).unwrap();
        let has_local_symbol = symbols
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["name"] == "Local"));
        assert!(
            !hover.is_null() && has_local_symbol,
            "hover={hover}; document symbols={symbols}"
        );

        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn same_package_completion_survives_project_root_in_ond_path() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-self-library-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let package = root.join("graphic");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"library\"\n").unwrap();
        std::fs::write(
            package.join("symbols.ond"),
            "package graphic\nconst Limit: u32 = 1\ntype Item u32\nfunc Helper() {}\n",
        )
        .unwrap();
        let path = package.join("main.ond");
        let source = "package graphic\nfunc main() {\n    \n}\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![root.clone()]);
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        let completions = state.completions(&uri, 2, 4).unwrap();
        for expected in ["Helper", "Limit", "Item"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing {expected}: {completions:?}"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_member_completion_uses_last_good_index_when_library_is_incomplete() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-incomplete-library-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let libraries = base.join("libraries");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(libraries.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"app\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\nimport \"shared\"\nfunc main() { shared.Run() }\n";
        std::fs::write(&path, source).unwrap();
        let library_path = libraries.join("shared/lib.ond");
        std::fs::write(
            &library_path,
            "package shared\nfunc Run() {}\nconst Limit: u32 = 1\ntype Item u32\n",
        )
        .unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![libraries]);
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        state.documents.insert(
            normalize_path(library_path),
            DocumentState {
                text: "package shared\nfunc Run(".to_string(),
            },
        );

        let character = source.lines().nth(2).unwrap().find("Run").unwrap() as u32;
        let completions = state.completions(&uri, 2, character).unwrap();
        for expected in ["Run", "Limit", "Item"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing {expected}: {completions:?}"
            );
        }
        assert!(!completions.iter().any(|item| item["label"] == "package"));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn package_member_completion_uses_updated_parseable_library_index() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-updated-library-index-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let libraries = base.join("libraries");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(libraries.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"app\"\n").unwrap();
        let main_path = root.join("main.ond");
        let source = "package main\nimport \"shared\"\nfunc main() { shared.Old() }\n";
        std::fs::write(&main_path, source).unwrap();
        let library_path = libraries.join("shared/lib.ond");
        std::fs::write(&library_path, "package shared\nfunc Old() {}\n").unwrap();

        let main_uri = uri_from_path(&main_path).unwrap();
        let library_uri = uri_from_path(&library_path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![libraries]);
        let mut output = Vec::new();
        state
            .open_document(main_uri.clone(), source.to_string(), &mut output)
            .unwrap();
        state
            .change_document(
                library_uri,
                vec![TextDocumentContentChangeEvent {
                    text: "package shared\nfunc New() {}\n".to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let incomplete = "package main\nimport \"shared\"\nfunc main() { shared. }\n";
        state
            .change_document(
                main_uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: incomplete.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let line = incomplete.lines().nth(2).unwrap();
        let completions = state
            .completions(&main_uri, 2, line.find(". }").unwrap() as u32 + 1)
            .unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "New"),
            "missing updated symbol: {completions:?}"
        );
        assert!(
            completions.iter().all(|item| item["label"] != "Old"),
            "stale symbol remained: {completions:?}"
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn package_member_completion_uses_cached_index_when_new_file_is_unindexed() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-unindexed-file-library-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let libraries = base.join("libraries");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(libraries.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"app\"\n").unwrap();
        let main1_path = root.join("main1.ond");
        let main1 = "package main\nimport \"shared\"\nfunc main() { shared.Run() }\n";
        std::fs::write(&main1_path, main1).unwrap();
        let main2_path = root.join("main2.ond");
        let main2 = "package main\nimport \"shared\"\nfunc main() { shared. }\n";
        std::fs::write(
            &main2_path,
            "package main\nimport \"shared\"\nfunc main() { shared.Run() }\n",
        )
        .unwrap();
        let library_path = libraries.join("shared/lib.ond");
        std::fs::write(&library_path, "package shared\nfunc Run() {}\n").unwrap();

        let main1_uri = uri_from_path(&main1_path).unwrap();
        let main2_uri = uri_from_path(&main2_path).unwrap();
        let library_uri = uri_from_path(&library_path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![libraries]);
        let mut output = Vec::new();
        state
            .open_document(main1_uri, main1.to_string(), &mut output)
            .unwrap();
        state
            .open_document(main2_uri.clone(), main2.to_string(), &mut output)
            .unwrap();
        let cached = state
            .symbol_indexes
            .get_mut(&normalize_path(root.clone()))
            .unwrap();
        for package in &mut cached.ast.packages {
            package.files.retain(|file| {
                !same_path(cached.loaded.sources.file(file.file_id).path(), &main2_path)
            });
        }
        state
            .change_document(
                library_uri,
                vec![TextDocumentContentChangeEvent {
                    text: "package shared\nfunc Run(".to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();

        let character = main2.lines().nth(2).unwrap().find(". }").unwrap() as u32 + 1;
        let completions = state.completions(&main2_uri, 2, character).unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "Run"),
            "missing last-good symbol: {completions:?}"
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn member_completion_returns_only_fields_for_struct_values_and_pointers() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-struct-members-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\ntype Pair struct { Left: u32; Right: u16; left: u32; right: u16 }\nfunc explicit(value: Pair) { value.Left = 1 }\nfunc pointer(value: *Pair) { value.Right = 1 }\nfunc inferred() { value := Pair{}; value.Left = 1 }\nfunc allocated() { value := new(Pair); value.Right = 1 }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        for line in 2..=5 {
            let text = source.lines().nth(line).unwrap();
            let character = text.rfind('.').unwrap() as u32 + 1;
            let completions = state.completions(&uri, line as u32, character).unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                labels,
                vec!["Left", "Right", "left", "right"],
                "line {line}: {completions:?}"
            );
            assert!(completions.iter().all(|item| item["kind"] == 5));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_includes_implicitly_convertible_receiver_methods() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-method-members-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"methods\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\ntype S struct {}\ntype Reader struct {}; func (value: Reader) Read() -> i32 { return 1 }\nfunc (value: S) Value() -> i32 { return 1; }\nfunc (value: S) privateValue() -> i32 { return 1; }\nfunc (value: *S) Pointer() -> i32 { return 2; }\nfunc (value: *S) privatePointer() -> i32 { return 2; }\nfunc use(value: S) { _ = value.Value(); }\nfunc usePointer(value: *S) { _ = value.Pointer(); }\nfunc useReader(value: Reader) { _ = value.Read(); }\nfunc main() {}\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        for (line, expected) in [
            (
                7,
                &["Value", "privateValue", "Pointer", "privatePointer"][..],
            ),
            (
                8,
                &["Value", "privateValue", "Pointer", "privatePointer"][..],
            ),
            (9, &["Read"][..]),
        ] {
            let text = source.lines().nth(line).unwrap();
            let character = text.rfind('.').unwrap() as u32 + 1;
            let completions = state.completions(&uri, line as u32, character).unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(labels, expected, "line {line}: {completions:?}");
            assert_eq!(completions[0]["kind"], 2);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn address_of_local_inference_completes_pointer_methods_in_both_modes() {
        for project_mode in [true, false] {
            for (selector, has_field, has_pointer_method, has_value_method) in [
                ("s.", true, true, true),
                ("ptr.", true, true, true),
                ("pp.", false, true, false),
                ("make().", true, false, true),
            ] {
                let root = normalize_path(std::env::temp_dir().join(format!(
                    "ond-lsp-address-of-members-{}-{}-{project_mode}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )));
                std::fs::create_dir_all(&root).unwrap();
                if project_mode {
                    std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
                }
                let source = format!(
                    "package main\ntype S struct {{ A: u32 }}\nfunc (s: *S) PointerMethod() {{}}\nfunc (s: S) ValueMethod() {{}}\nfunc make() -> S {{ return S{{}} }}\nfunc main() {{\n    var s = S{{ A: 14 }}\n    ptr := &s\n    pp := &ptr\n    {selector}\n}}\n"
                );
                let path = root.join("main.ond");
                std::fs::write(&path, &source).unwrap();
                let uri = uri_from_path(&path).unwrap();
                let mut state = ServerState::default();
                let mut output = Vec::new();
                state
                    .open_document(uri.clone(), source.clone(), &mut output)
                    .unwrap();

                let text = source.lines().nth(9).unwrap();
                let character = text.rfind('.').unwrap() as u32 + 1;
                let completions = state.completions(&uri, 9, character).unwrap();
                let labels = completions
                    .iter()
                    .filter_map(|item| item["label"].as_str())
                    .collect::<Vec<_>>();
                assert_eq!(
                    labels.contains(&"A"),
                    has_field,
                    "project={project_mode}, selector={selector}: {completions:?}"
                );
                assert_eq!(
                    labels.contains(&"PointerMethod"),
                    has_pointer_method,
                    "project={project_mode}, selector={selector}: {completions:?}"
                );
                assert_eq!(
                    labels.contains(&"ValueMethod"),
                    has_value_method,
                    "project={project_mode}, selector={selector}: {completions:?}"
                );
                std::fs::remove_dir_all(root).unwrap();
            }
        }
    }

    #[test]
    fn named_pointer_alias_completes_one_level_methods_in_both_modes() {
        for project_mode in [true, false] {
            let root = normalize_path(std::env::temp_dir().join(format!(
                "ond-lsp-named-pointer-methods-{}-{}-{project_mode}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
            std::fs::create_dir_all(&root).unwrap();
            if project_mode {
                std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
            }
            let source = "package main\ntype Child struct {}\ntype S struct { child: Child }\ntype P *S\ntype R struct { A: u32 }\nfunc (s: S) Value() -> R { return R{} }\nfunc (s: *S) Pointer() {}\nfunc (p: *P) Address() {}\nfunc (child: *Child) Run() {}\nfunc make() -> P { return nil }\nfunc main() { var p: P; p. }\n";
            let path = root.join("main.ond");
            std::fs::write(&path, source).unwrap();
            let uri = uri_from_path(&path).unwrap();
            let mut state = ServerState::default();
            let mut output = Vec::new();
            state
                .open_document(uri.clone(), source.to_string(), &mut output)
                .unwrap();

            let main_line = source
                .lines()
                .position(|line| line.starts_with("func main"))
                .unwrap();
            let line = source.lines().nth(main_line).unwrap();
            let character = line.rfind('.').unwrap() as u32 + 1;
            let completions = state
                .completions(&uri, main_line as u32, character)
                .unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert!(labels.contains(&"Value"), "{completions:?}");
            assert!(labels.contains(&"Address"), "{completions:?}");
            assert!(!labels.contains(&"Pointer"), "{completions:?}");

            if project_mode {
                let source = source.replace("p. }", "p.Value(). }");
                state
                    .change_document(
                        uri.clone(),
                        vec![TextDocumentContentChangeEvent {
                            text: source.clone(),
                            range: None,
                        }],
                        &mut output,
                    )
                    .unwrap();
                let line = source.lines().nth(main_line).unwrap();
                let character = line.rfind('.').unwrap() as u32 + 1;
                let completions = state
                    .completions(&uri, main_line as u32, character)
                    .unwrap();
                assert!(
                    completions.iter().any(|item| item["label"] == "A"),
                    "{completions:?}"
                );
            }

            let source = source.replace("p. }", "make().child. }");
            state
                .change_document(
                    uri.clone(),
                    vec![TextDocumentContentChangeEvent {
                        text: source.clone(),
                        range: None,
                    }],
                    &mut output,
                )
                .unwrap();
            let line = source.lines().nth(main_line).unwrap();
            let character = line.rfind('.').unwrap() as u32 + 1;
            let completions = state
                .completions(&uri, main_line as u32, character)
                .unwrap();
            assert!(
                completions.iter().any(|item| item["label"] == "Run"),
                "{completions:?}"
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn imported_generic_named_pointer_keeps_type_argument_package() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-imported-generic-pointer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(root.join("runtime")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        std::fs::write(
            root.join("runtime/api.ond"),
            "package runtime\ntype P[T] *T\ntype S struct {}\nfunc (s: S) Foreign() {}\ntype Cell[T] struct { value: T }\ntype CellPointer[T] *Cell[T]\nfunc (cell: Cell[T]) Get() -> T { return cell.value }\n",
        )
        .unwrap();
        let path = root.join("main.ond");
        let source = "package main\nimport \"members/runtime\"\ntype S struct {}\nfunc (s: S) Value() {}\nfunc main() { var p: runtime.P[S]; p. }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        let line = source.lines().nth(4).unwrap();
        let character = line.rfind('.').unwrap() as u32 + 1;
        let completions = state.completions(&uri, 4, character).unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "Value"),
            "{completions:?}"
        );
        assert!(
            completions.iter().all(|item| item["label"] != "Foreign"),
            "{completions:?}"
        );

        let source = "package main\nimport \"members/runtime\"\ntype S struct { A: u32 }\nfunc main() { var p: runtime.CellPointer[S]; p.Get(). }\n";
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: source.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let line = source.lines().nth(3).unwrap();
        let character = line.rfind('.').unwrap() as u32 + 1;
        let completions = state.completions(&uri, 3, character).unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "A"),
            "{completions:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn named_pointer_alias_does_not_complete_through_two_pointer_levels() {
        for project_mode in [true, false] {
            let root = normalize_path(std::env::temp_dir().join(format!(
                "ond-lsp-pointer-alias-members-{}-{}-{project_mode}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
            std::fs::create_dir_all(&root).unwrap();
            if project_mode {
                std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
            }
            let source = "package main\ntype S struct { A: u32 }\ntype P *S\nfunc (s: *S) Method() {}\nvar p: P\nfunc main() { pp := &p; pp. }\n";
            let path = root.join("main.ond");
            std::fs::write(&path, source).unwrap();
            let uri = uri_from_path(&path).unwrap();
            let mut state = ServerState::default();
            let mut output = Vec::new();
            state
                .open_document(uri.clone(), source.to_string(), &mut output)
                .unwrap();

            let line = source.lines().nth(5).unwrap();
            let character = line.rfind('.').unwrap() as u32 + 1;
            let completions = state.completions(&uri, 5, character).unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert!(!labels.contains(&"A"), "{completions:?}");
            assert!(!labels.contains(&"Method"), "{completions:?}");
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn deep_address_of_local_inference_returns_completion_in_both_modes() {
        for project_mode in [true, false] {
            let root = normalize_path(std::env::temp_dir().join(format!(
                "ond-lsp-deep-address-of-{}-{}-{project_mode}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
            std::fs::create_dir_all(&root).unwrap();
            if project_mode {
                std::fs::write(root.join("ond.toml"), "[project]\nname=\"deep\"\n").unwrap();
            }
            let mut source = String::from(
                "package main\ntype S struct { A: u32 }\nfunc main() {\n    var s = S{}\n",
            );
            for index in 0..1024 {
                let previous = if index == 0 {
                    "s".to_string()
                } else {
                    format!("v{}", index - 1)
                };
                source.push_str(&format!("    v{index} := &{previous}\n"));
            }
            source.push_str("    s.\n}\n");
            let path = root.join("main.ond");
            std::fs::write(&path, &source).unwrap();
            let uri = uri_from_path(&path).unwrap();
            let mut state = ServerState::default();
            let mut output = Vec::new();
            state
                .open_document(uri.clone(), source.clone(), &mut output)
                .unwrap();

            let line = source.lines().nth(1028).unwrap();
            let completions = state.completions(&uri, 1028, line.len() as u32).unwrap();
            assert!(
                completions.iter().any(|item| item["label"] == "A"),
                "project={project_mode}: {completions:?}"
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn generic_struct_completion_uses_applied_type_arguments() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-generic-struct-members-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"methods\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\ntype List[T] struct { item:T }\nfunc (value: List[T]) Get(index: u32) -> T { var result:T; return result }\nfunc (value: List[T]) Set(index: u32, item:T) {}\nfunc use(values: List[u16]) { values. }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        let character = source.lines().nth(4).unwrap().find(" }").unwrap() as u32;
        let completions = state.completions(&uri, 4, character).unwrap();
        assert_eq!(
            completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>(),
            vec!["item", "Get", "Set"]
        );
        assert_eq!(completions[0]["detail"], "item: u16");
        assert_eq!(completions[1]["detail"], "Get(index: u32) -> T");
        assert_eq!(completions[2]["detail"], "Set(index: u32, item: T)");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_includes_private_generic_fields_and_methods_in_same_package() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-private-generic-members-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        let path = root.join("main.ond");
        let valid = "package main\ntype Inner struct { hidden: u32 }\ntype Box[T] struct { Value: T; value: T }\nfunc (box: *Box[T]) Read() -> T { return box.value }\nfunc (box: *Box[T]) read() -> T { return box.value }\nfunc use(box: *Box[Inner]) { _ = box.Read(); _ = box.value.hidden }\n";
        std::fs::write(&path, valid).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();

        let source = valid.replace("_ = box.Read()", "_ = box.");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: source.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();

        let character =
            source.lines().nth(5).unwrap().find("box.").unwrap() as u32 + "box.".len() as u32;
        let completions = state.completions(&uri, 5, character).unwrap();
        let labels = completions
            .iter()
            .filter_map(|item| item["label"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["Value", "value", "Read", "read"]);

        let nested_source = valid.replace("_ = box.value.hidden", "_ = box.value.");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: nested_source.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let nested_character = nested_source
            .lines()
            .nth(5)
            .unwrap()
            .find("box.value.")
            .unwrap() as u32
            + "box.value.".len() as u32;
        let nested = state.completions(&uri, 5, nested_character).unwrap();
        assert_eq!(
            nested.iter().map(|item| &item["label"]).collect::<Vec<_>>(),
            vec!["hidden"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_infers_explicit_generic_function_result() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-generic-call-result-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        let path = root.join("main.ond");
        let valid = "package main\ntype List[T] struct { length: u32; first: T }\nfunc (list: *List[T]) push(value: T) {}\nfunc NewList[T](length: u32, capacity: u32) -> *List[T] { return new(List[T]) }\nfunc f() { l := NewList[u32](1, 1); _ = l.length }\n";
        std::fs::write(&path, valid).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();

        let source = valid.replace("_ = l.length", "l.");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: source.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let character = source.lines().nth(4).unwrap().find("l. }").unwrap() as u32 + 2;
        let completions = state.completions(&uri, 4, character).unwrap();
        assert_eq!(
            completions
                .iter()
                .map(|item| &item["label"])
                .collect::<Vec<_>>(),
            vec!["length", "first", "push"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn single_file_member_completion_infers_explicit_generic_function_result() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-single-file-generic-call-result-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("main.ond");
        let source = "package main\ntype List[T] struct { length: u32; first: T }\nfunc (list: *List[T]) push(value: T) {}\nfunc NewList[T](length: u32, capacity: u32) -> *List[T] { return new(List[T]) }\nfunc f() { l := NewList[u32](1, 1); l. }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        let character = source.lines().nth(4).unwrap().find("l. }").unwrap() as u32 + 2;
        let completions = state.completions(&uri, 4, character).unwrap();
        assert_eq!(
            completions
                .iter()
                .map(|item| &item["label"])
                .collect::<Vec<_>>(),
            vec!["length", "first", "push"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn method_receiver_and_operator_rhs_keep_local_and_member_completion() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-receiver-completion-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"receiver\"\n").unwrap();
        let path = root.join("main.ond");
        let valid = "package main\ntype Counter struct { Value: u32; value: u32 }\nfunc (counter: *Counter) Reset() {}\nfunc (counter: *Counter) reset() {}\nfunc (counter: *Counter) Tick(delta: u32) { var total = delta; counter.Reset() }\nfunc main() {}\n";
        std::fs::write(&path, valid).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();

        let member_source = "package main\ntype Counter struct { Value: u32; value: u32 }\nfunc (counter: *Counter) Reset() {}\nfunc (counter: *Counter) reset() {}\nfunc (counter: *Counter) Tick(delta: u32) { _ = counter. }\nfunc main() {}\n";
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: member_source.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let member_character = member_source.lines().nth(4).unwrap().find(" }").unwrap() as u32;
        let member_items = state.completions(&uri, 4, member_character).unwrap();
        for expected in ["Value", "value", "Reset", "reset", "Tick"] {
            assert!(
                member_items.iter().any(|item| item["label"] == expected),
                "missing {expected}: {member_items:?}"
            );
        }

        for incomplete_body in [
            "if counter. {}",
            "if delta > 0 && counter. {}",
            "var total = delta + counter.",
            "_ = delta + counter.",
        ] {
            let source = format!(
                "package main\ntype Counter struct {{ Value: u32; value: u32 }}\nfunc (counter: *Counter) Reset() {{}}\nfunc (counter: *Counter) reset() {{}}\nfunc (counter: *Counter) Tick(delta: u32) {{ {incomplete_body} }}\nfunc main() {{}}\n"
            );
            state
                .change_document(
                    uri.clone(),
                    vec![TextDocumentContentChangeEvent {
                        text: source.clone(),
                        range: None,
                    }],
                    &mut output,
                )
                .unwrap();
            let line = source.lines().nth(4).unwrap();
            let character = line.find("counter.").unwrap() as u32 + "counter.".len() as u32;
            let items = state.completions(&uri, 4, character).unwrap();
            for expected in ["Value", "value", "Reset", "reset", "Tick"] {
                assert!(
                    items.iter().any(|item| item["label"] == expected),
                    "missing {expected} after `{incomplete_body}`: {items:?}"
                );
            }
        }

        let multiline_source = "package main\ntype Counter struct { Value: u32; value: u32 }\nfunc (counter: *Counter) Reset() {}\nfunc (counter: *Counter) reset() {}\nfunc (counter: *Counter) Tick(delta: u32) {\n    if delta > 0 &&\n        counter. {}\n}\nfunc main() {}\n";
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: multiline_source.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let multiline_line = multiline_source.lines().nth(6).unwrap();
        let multiline_character =
            multiline_line.find("counter.").unwrap() as u32 + "counter.".len() as u32;
        let multiline_items = state.completions(&uri, 6, multiline_character).unwrap();
        for expected in ["Value", "value", "Reset", "reset", "Tick"] {
            assert!(
                multiline_items.iter().any(|item| item["label"] == expected),
                "missing {expected} in a multiline incomplete condition: {multiline_items:?}"
            );
        }

        let operator_source = "package main\ntype Counter struct { Value: u32; value: u32 }\nfunc (counter: *Counter) Reset() {}\nfunc (counter: *Counter) reset() {}\nfunc (counter: *Counter) Tick(delta: u32) { var total = delta +  }\nfunc main() {}\n";
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: operator_source.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let operator_character =
            operator_source.lines().nth(4).unwrap().find("  }").unwrap() as u32;
        let operator_items = state.completions(&uri, 4, operator_character).unwrap();
        for expected in ["counter", "delta"] {
            assert!(
                operator_items.iter().any(|item| item["label"] == expected),
                "missing {expected}: {operator_items:?}"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn operator_parameters_support_local_and_member_completion() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-operator-parameter-completion-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"operators\"\n").unwrap();
        let path = root.join("main.ond");
        let valid = "package main\ntype Box[T] struct { Value: T; value: T }\nfunc (box: Box[T]) Read() -> T { return box.value }\noperator (left: Box[T]) + (right: Box[T]) -> Box[T] { return left }\n";
        std::fs::write(&path, valid).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();

        let member_source = valid.replace("return left", "left.");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: member_source.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let member_character = member_source.lines().nth(3).unwrap().find("left.").unwrap() as u32
            + "left.".len() as u32;
        let members = state.completions(&uri, 3, member_character).unwrap();
        assert_eq!(
            members
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>(),
            vec!["Value", "value", "Read"]
        );

        let value_source = valid.replace("return left", "return left + ");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: value_source.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let value_character = value_source.lines().nth(3).unwrap().find(" }").unwrap() as u32;
        let values = state.completions(&uri, 3, value_character).unwrap();
        for expected in ["left", "right"] {
            assert!(
                values.iter().any(|item| item["label"] == expected),
                "missing {expected}: {values:?}"
            );
        }

        std::fs::write(&path, &member_source).unwrap();
        let mut fresh_state = ServerState::default();
        let mut fresh_output = Vec::new();
        fresh_state
            .open_document(uri.clone(), member_source.clone(), &mut fresh_output)
            .unwrap();
        let members = fresh_state.completions(&uri, 3, member_character).unwrap();
        assert_eq!(
            members
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>(),
            vec!["Value", "value", "Read"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_chains_from_generic_function_results() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-generic-result-chain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"chains\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\ntype Box[T] struct { Value: T; value: T }\nfunc (box: Box[T]) Read() -> T { return box.value }\nfunc NewBox[T]() -> Box[T] { return Box[T]{} }\nfunc f() { NewBox[u32](). }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        let character = source.lines().nth(4).unwrap().find(" }").unwrap() as u32;
        let completions = state.completions(&uri, 4, character).unwrap();
        assert_eq!(
            completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>(),
            vec!["Value", "value", "Read"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_chains_from_implicitly_dereferenced_generic_method_results() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-generic-method-result-chain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"chains\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\ntype List[T] struct {}\nfunc (list: List[T]) Pop() -> T { trap \"empty\" }\ntype Num u32\nfunc (value: Num) toStr() -> *u8 { return nil }\nfunc NewList[T]() -> *List[T] { return new(List[T]) }\nfunc f() { l := NewList[Num](); l.Pop(). }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        let character = source.lines().nth(6).unwrap().find(" }").unwrap() as u32;
        let completions = state.completions(&uri, 6, character).unwrap();
        assert_eq!(
            completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>(),
            vec!["toStr"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_recovers_while_assignment_target_is_incomplete() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-incomplete-assignment-target-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        let path = root.join("main.ond");
        let mut state = ServerState::default();
        let mut output = Vec::new();
        let valid_source = "package main\ntype Pair struct { Left: u32; Right: u16 }\nfunc update(value: Pair) {}\n";
        std::fs::write(&path, valid_source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        state
            .open_document(uri.clone(), valid_source.to_string(), &mut output)
            .unwrap();

        for target in ["value.", "value.L", "value. =", "value.L ="] {
            let source = format!(
                "package main\ntype Pair struct {{ Left: u32; Right: u16 }}\nfunc update(value: Pair) {{\n    {target}\n}}\n"
            );
            state
                .change_document(
                    uri.clone(),
                    vec![TextDocumentContentChangeEvent {
                        text: source.clone(),
                        range: None,
                    }],
                    &mut output,
                )
                .unwrap();

            let character = source.lines().nth(3).unwrap().find(target).unwrap() as u32
                + target.find([' ', '=']).unwrap_or(target.len()) as u32;
            let completed =
                crate::completion::completion_text_with_placeholder(&source, 3, character)
                    .unwrap()
                    .0;
            let mut sources = compiler::source::SourceDb::default();
            let file_id = sources.add_file(path.clone(), completed.clone());
            compiler::parse_source_file(file_id, &completed)
                .unwrap_or_else(|error| panic!("target {target:?}: {error:?}\n{completed}"));
            let completions = state.completions(&uri, 3, character).unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(labels, vec!["Left", "Right"], "target {target:?}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_resolves_external_struct_types_and_results() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-external-struct-members-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(root.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        std::fs::write(
            root.join("shared/lib.ond"),
            "package shared\ntype Inner struct { Visible: u16; hidden: u16 }\ntype Pair struct { Left: u32; Child: Inner; hidden: u32 }\ntype Box[T] struct { Visible: T; hidden: T }\nfunc NewPair() -> Pair { return Pair{} }\n",
        )
        .unwrap();
        let path = root.join("main.ond");
        let source = "package main\nimport \"members/shared\"\nfunc explicit(value: shared.Pair) { value.Left = 1 }\nfunc composite() { value := shared.Pair{}; value.Left = 1 }\nfunc allocated() { value := new(shared.Pair); value.Left = 1 }\nfunc factory() { value := shared.NewPair(); value.Left = 1 }\nfunc nested(value: shared.Pair) { value.Child.Visible = 1 }\nfunc generic(value: shared.Box[u32]) { value.Visible = 1 }\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        for line in 2..=5 {
            let text = source.lines().nth(line).unwrap();
            let character = text.rfind('.').unwrap() as u32 + 1;
            let completions = state.completions(&uri, line as u32, character).unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                labels,
                vec!["Left", "Child"],
                "line {line}: {completions:?}"
            );
        }

        let nested = source.lines().nth(6).unwrap();
        let character = nested.rfind('.').unwrap() as u32 + 1;
        let completions = state.completions(&uri, 6, character).unwrap();
        let labels = completions
            .iter()
            .filter_map(|item| item["label"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["Visible"], "{completions:?}");

        let generic = source.lines().nth(7).unwrap();
        let character = generic.rfind('.').unwrap() as u32 + 1;
        let completions = state.completions(&uri, 7, character).unwrap();
        let labels = completions
            .iter()
            .filter_map(|item| item["label"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["Visible"], "{completions:?}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_resolves_methods_from_imported_function_parameter_types() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-imported-parameter-methods-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(root.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"members\"\n").unwrap();
        std::fs::write(
            root.join("shared/lib.ond"),
            "package shared\ntype Item struct { Visible: u32; hidden: u32 }\nfunc (item: Item) Read() -> u32 { return item.Visible }\nfunc (item: Item) read() -> u32 { return item.hidden }\ntype Box[T] struct { Value: T; hidden: T }\nfunc (box: *Box[T]) Get() -> T { return box.Value }\nfunc (box: *Box[T]) get() -> T { return box.hidden }\n",
        )
        .unwrap();
        let path = root.join("main.ond");
        let valid = "package main\nimport \"members/shared\"\nfunc plain(value: shared.Item) { _ = value.Read() }\nfunc generic(value: *shared.Box[u16]) { _ = value.Get() }\n";
        std::fs::write(&path, valid).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();

        for (line, call, expected) in [
            (2, "_ = value.Read()", &["Visible", "Read"][..]),
            (3, "_ = value.Get()", &["Value", "Get"][..]),
        ] {
            let source = valid.replace(call, "value.");
            state
                .change_document(
                    uri.clone(),
                    vec![TextDocumentContentChangeEvent {
                        text: source.clone(),
                        range: None,
                    }],
                    &mut output,
                )
                .unwrap();
            let character = source.lines().nth(line).unwrap().find("value.").unwrap() as u32
                + "value.".len() as u32;
            let completions = state.completions(&uri, line as u32, character).unwrap();
            let labels = completions
                .iter()
                .filter_map(|item| item["label"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(labels, expected, "line {line}: {completions:?}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn member_completion_does_not_fall_back_to_unrelated_candidates() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-filtered-members-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(root.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"filtered\"\n").unwrap();
        std::fs::write(
            root.join("shared/lib.ond"),
            "package shared\nfunc Run() {}\n",
        )
        .unwrap();
        let path = root.join("main.ond");
        let source = "package main\nimport \"shared\"\nfunc main(shared: u32) {\n    shared.Missing = 1\n}\n";
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();

        let line = source.lines().nth(3).unwrap();
        let character = line.find('.').unwrap() as u32 + 1;
        let completions = state.completions(&uri, 3, character).unwrap();
        assert!(
            completions.is_empty(),
            "unexpected candidates: {completions:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_path_completion_includes_project_and_library_packages() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-import-paths-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let libraries = base.join("libraries");
        std::fs::create_dir_all(root.join("tools/diag")).unwrap();
        std::fs::create_dir_all(root.join("unrelated")).unwrap();
        std::fs::create_dir_all(libraries.join("tools/log")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"lsp\"\n").unwrap();
        let path = root.join("main.ond");
        let source = "package main\nimport \"to\n";
        std::fs::write(&path, "package main\n").unwrap();
        std::fs::write(root.join("tools/diag/diag.ond"), "package diag\n").unwrap();
        std::fs::write(root.join("unrelated/value.ond"), "package unrelated\n").unwrap();
        std::fs::write(libraries.join("tools/log/log.ond"), "package log\n").unwrap();
        let path = normalize_path(path);
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![libraries]);
        state.documents.insert(
            path.clone(),
            DocumentState {
                text: source.to_string(),
            },
        );

        let completions = state.completions(&uri, 1, 10).unwrap();
        let labels = completions
            .iter()
            .filter_map(|item| item["label"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["tools/log"]);
        assert_eq!(completions[0]["kind"], 9);
        assert_eq!(completions[0]["textEdit"]["newText"], "tools/log");
        assert_eq!(completions[0]["textEdit"]["range"]["start"]["character"], 8);
        state.documents.insert(
            path,
            DocumentState {
                text: "package main\nimport \"lsp/to\n".to_string(),
            },
        );
        let own = state.completions(&uri, 1, 14).unwrap();
        assert!(own.iter().any(|item| item["label"] == "lsp/tools/diag"));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn symbol_index_survives_an_incomplete_edit() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-symbol-index-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"index\"\n").unwrap();
        let path = root.join("main.ond");
        let valid = "package main\nconst Limit: u32 = 1\ntype Item u32\nfunc Helper() {}\nfunc main() {\n    Helper()\n}\n";
        let incomplete = "package main\nconst Limit: u32 = 1\ntype Item u32\nfunc Helper() {}\nfunc main() {\n    Helper(\n}\n";
        std::fs::write(&path, valid).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: incomplete.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();

        let completions = state.completions(&uri, 5, 4).unwrap();
        for expected in ["Helper", "Limit", "Item"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing {expected}: {completions:?}"
            );
        }
        let definition = state.definition(&uri, 5, 5).unwrap();
        assert_eq!(definition[0]["targetSelectionRange"]["start"]["line"], 3);
        let symbols = state.document_symbols(&uri).unwrap();
        assert!(
            symbols
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["name"] == "Helper")),
            "{symbols}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_loader_includes_imported_ond_path_packages() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-libraries-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let libraries = base.join("libraries");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(libraries.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"lsp\"\n").unwrap();
        std::fs::write(
            root.join("main.ond"),
            "package main\nimport \"shared\"\nfunc main() { shared.Run() }\n",
        )
        .unwrap();
        let library_file = libraries.join("shared/lib.ond");
        std::fs::write(&library_file, "package shared\nfunc Run() {}\n").unwrap();

        let state = ServerState::with_dependencies(&root, vec![libraries]);
        let loaded = state.load_project(&root, &BTreeMap::new()).unwrap();
        let library_file = library_file.canonicalize().unwrap();
        assert!(
            loaded
                .packages
                .iter()
                .any(|package| package.logical_path == "shared")
        );
        assert!(
            loaded
                .sources
                .files()
                .iter()
                .any(|source| source.path() == library_file)
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn indexed_import_members_include_functions_constants_and_types() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-indexed-imports-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let libraries = base.join("libraries");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(libraries.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"lsp\"\n").unwrap();
        let path = root.join("main.ond");
        let valid =
            "package main\nimport \"shared\"\nfunc main() { var value: u32 = shared.Limit }\n";
        let incomplete =
            "package main\nimport \"shared\"\nfunc main() { var value: u32 = shared. }\n";
        std::fs::write(&path, valid).unwrap();
        std::fs::write(
            libraries.join("shared/lib.ond"),
            "package shared\nfunc Run() {}\nconst Limit: u32 = 1\nconst hidden: u32 = 2\ntype Item u32\n",
        )
        .unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![libraries]);
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), valid.to_string(), &mut output)
            .unwrap();
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: incomplete.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let character = incomplete.lines().nth(2).unwrap().find(" }").unwrap() as u32;
        let completions = state.completions(&uri, 2, character).unwrap();

        for expected in ["Run", "Limit", "Item"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing {expected}: {completions:?}"
            );
        }
        assert!(!completions.iter().any(|item| item["label"] == "hidden"));
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn imports_remain_completable_when_an_unrelated_package_fails_to_parse() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-imports-with-parse-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(root.join("collection/span")).unwrap();
        std::fs::create_dir_all(root.join("managed/string")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"standard\"\n").unwrap();
        let path = root.join("main.ond");
        let baseline =
            "package main\nimport \"standard/collection/span\"\nfunc main() { span.Init(1) }\n";
        let source = "package main\nimport \"standard/collection/span\"\nfunc main() { s }\n";
        std::fs::write(&path, baseline).unwrap();
        std::fs::write(
            root.join("collection/span/api.ond"),
            "package span\ntype Span[T] struct { value: T }\nfunc Init[T](value: T) -> Span[T] { return Span[T]{value: value} }\nfunc New[T](value: T) -> *Span[T] { return nil }\nfunc hidden() {}\n",
        )
        .unwrap();
        std::fs::write(
            root.join("managed/string/string.ond"),
            "package string\ntype String struct {}\nfunc (value: String) Length() -> u32 { return 0 }\noperator (value: String) [] (index: u32) -> u32 { return value.Length() }\nfunc Broken(\n",
        )
        .unwrap();

        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), baseline.to_string(), &mut output)
            .unwrap();
        assert!(compiler::ond::parse_project(&state.load_current_project(&root).unwrap()).is_err());
        assert!(String::from_utf8_lossy(&output).contains("publishDiagnostics"));
        let baseline_line = baseline.lines().nth(2).unwrap();
        let init_character = baseline_line.find("Init").unwrap() as u32 + 1;
        assert!(
            !state.hover(&uri, 2, init_character).unwrap().is_null(),
            "valid file hover should survive an unrelated package parse error"
        );
        assert!(
            !state.definition(&uri, 2, init_character).unwrap().is_null(),
            "valid file definition should survive an unrelated package parse error"
        );
        assert!(
            !state
                .document_symbols(&uri)
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty(),
            "valid file symbols should survive an unrelated package parse error"
        );
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: source.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let source_line = source.lines().nth(2).unwrap();
        let completions = state
            .completions(&uri, 2, source_line.find("s }").unwrap() as u32 + 1)
            .unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "span"),
            "missing import alias: {completions:?}"
        );

        let incomplete =
            "package main\nimport \"standard/collection/span\"\nfunc main() { span. }\n";
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: incomplete.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let incomplete_line = incomplete.lines().nth(2).unwrap();
        let completions = state
            .completions(&uri, 2, incomplete_line.find(". }").unwrap() as u32 + 1)
            .unwrap();
        for expected in ["Span", "Init", "New"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing public import member {expected}: {completions:?}"
            );
        }
        assert!(
            completions.iter().all(|item| item["label"] != "hidden"),
            "private member was exposed: {completions:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_file_index_refreshes_files_independently_and_drops_stale_declarations() {
        let root = test_project_root("per-file-index");
        std::fs::create_dir_all(root.join("model")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"app\"\n").unwrap();
        let main = root.join("main.ond");
        let same = root.join("same.ond");
        let model = root.join("model/item.ond");
        let initial = "package main\nimport \"app/model\"\nfunc Use(stale: model.Item) { var staleLocal: model.Item; _ = staleLocal.Field }\nfunc main() { Shared() }\n";
        let updated = "package main\nimport \"app/model\"\nconst Fresh: u32 = 1\nfunc Use(current: model.Item) { var local: model.Item; _ = current; _ = local }\nfunc main() { Shared() }\n";
        std::fs::write(&main, initial).unwrap();
        std::fs::write(&same, "package main\nfunc Shared() {}\n").unwrap();
        std::fs::write(&model, "package model\ntype Item struct { Field: u32 }\n").unwrap();

        let uri = uri_from_path(&main).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), initial.to_string(), &mut output)
            .unwrap();

        // A new file shifts source slots while the other package fails to parse.
        std::fs::write(root.join("aaa.ond"), "package main\nfunc Earlier() {}\n").unwrap();
        std::fs::write(&same, "package main\nfunc Shared() {}\nfunc Broken(\n").unwrap();
        std::fs::write(
            &model,
            "package model\ntype Item struct { Field: u32 }\nfunc Broken(\n",
        )
        .unwrap();
        std::fs::write(
            root.join("model/extra.ond"),
            "package model\nfunc NewModel() {}\n",
        )
        .unwrap();
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: updated.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();

        let fresh_line = updated.lines().nth(2).unwrap();
        let fresh_hover = state
            .hover(&uri, 2, fresh_line.find("Fresh").unwrap() as u32 + 1)
            .unwrap();
        assert!(
            fresh_hover["contents"]["value"]
                .as_str()
                .is_some_and(|value| value.contains("Fresh")),
            "updated valid file was not indexed: {fresh_hover}"
        );
        let shared_line = updated.lines().nth(4).unwrap();
        let shared_character = shared_line.find("Shared").unwrap() as u32 + 1;
        assert!(
            !state.hover(&uri, 4, shared_character).unwrap().is_null(),
            "last-good declaration from the failed file was not available"
        );
        let definition = state.definition(&uri, 4, shared_character).unwrap();
        assert!(
            definition[0]["targetUri"]
                .as_str()
                .is_some_and(|target| target.ends_with("same.ond")),
            "failed-file declaration mapped to the wrong source: {definition}"
        );

        let use_line = updated.lines().nth(3).unwrap();
        let completions = state
            .completions(
                &uri,
                3,
                use_line.rfind("local").unwrap() as u32 + "local".len() as u32,
            )
            .unwrap();
        for expected in ["current", "local"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing current local/parameter {expected}: {completions:?}"
            );
        }
        assert!(
            completions
                .iter()
                .all(|item| { item["label"] != "stale" && item["label"] != "staleLocal" }),
            "locals from the previous file text leaked: {completions:?}"
        );

        let member_completion =
            updated.replace("_ = current; _ = local", "_ = current.Field; _ = local.");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: member_completion.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let use_line = member_completion.lines().nth(3).unwrap();
        let completions = state
            .completions(
                &uri,
                3,
                use_line.find("local.").unwrap() as u32 + "local.".len() as u32,
            )
            .unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "Field"),
            "imported type information was not available for the current local: {completions:?}"
        );
        let package_completion = updated.replace("_ = current; _ = local", "model.");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: package_completion.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let use_line = package_completion.lines().nth(3).unwrap();
        let completions = state
            .completions(
                &uri,
                3,
                use_line.find("model.").unwrap() as u32 + "model.".len() as u32,
            )
            .unwrap();
        for expected in ["Item", "NewModel"] {
            assert!(
                completions.iter().any(|item| item["label"] == expected),
                "missing retained/new imported member {expected}: {completions:?}"
            );
        }

        let changed_type = format!(
            "{}func Broken(\n",
            updated
                .replace("current: model.Item", "current: model.Other")
                .replace("_ = current; _ = local", "_ = current.; _ = local")
        );
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: changed_type.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let changed_type_line = changed_type.lines().nth(3).unwrap();
        let completions = state
            .completions(
                &uri,
                3,
                changed_type_line.find("current.").unwrap() as u32 + "current.".len() as u32,
            )
            .unwrap();
        assert!(
            completions.iter().all(|item| item["label"] != "Field"),
            "member from the last-good type leaked after the current local type changed: {completions:?}"
        );

        let malformed_locals = format!(
            "{}func Broken(\n",
            updated
                .replace("Use(current:", "Use(renamed:")
                .replace("var local:", "var newLocal:")
                .replace("_ = current;", "_ = renamed;")
                .replace("_ = local", "_ = newLocal")
        );
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: malformed_locals.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let malformed_line = malformed_locals.lines().nth(3).unwrap();
        let completions = state
            .completions(
                &uri,
                3,
                malformed_line.rfind("newLocal").unwrap() as u32 + "newLocal".len() as u32,
            )
            .unwrap();
        assert!(
            completions.iter().all(|item| {
                item["label"] != "current"
                    && item["label"] != "local"
                    && item["label"] != "stale"
                    && item["label"] != "staleLocal"
            }),
            "malformed current file reused old parameters or locals: {completions:?}"
        );
        assert!(
            state
                .hover(&uri, 2, fresh_line.find("Fresh").unwrap() as u32 + 1)
                .unwrap()
                .is_null(),
            "hover used stale spans from the last-good version of an invalid file"
        );
        assert_eq!(
            state.symbol_indexes[&normalize_path(root.clone())].last_good_source(&main),
            Some(updated),
            "temporary completion repair replaced the last-good source snapshot"
        );
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: updated.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let repaired = updated.replace("Shared()", "Repaired()");
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: repaired.clone(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        std::fs::write(&same, "package main\nfunc Repaired() {}\n").unwrap();
        state.watched_files_changed(&mut output).unwrap();
        let repaired_line = repaired.lines().nth(4).unwrap();
        let repaired_character = repaired_line.find("Repaired").unwrap() as u32 + 1;
        assert!(!state.hover(&uri, 4, repaired_character).unwrap().is_null());
        assert!(
            state
                .hover(&uri, 4, repaired_line.find("Repaired").unwrap() as u32 + 1)
                .unwrap()["contents"]["value"]
                .as_str()
                .is_some_and(|value| value.contains("Repaired"))
        );

        let renamed = root.join("renamed.ond");
        std::fs::rename(&same, &renamed).unwrap();
        state.watched_files_changed(&mut output).unwrap();
        let definition = state.definition(&uri, 4, repaired_character).unwrap();
        assert!(
            definition[0]["targetUri"]
                .as_str()
                .is_some_and(|target| target.ends_with("renamed.ond")),
            "rename left a stale declaration location: {definition}"
        );
        std::fs::remove_file(&renamed).unwrap();
        state.watched_files_changed(&mut output).unwrap();
        assert!(
            state.hover(&uri, 4, repaired_character).unwrap().is_null(),
            "deleted declaration remained in the index"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validation_does_not_apply_target_frame_limits() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-core-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"lsp\"\n").unwrap();
        let source = "package main\nfunc main() { var buffer:[536870912]u32 }\n";
        let path = root.join("main.ond");
        std::fs::write(&path, source).unwrap();
        let mut state = ServerState::default();
        state.documents.insert(
            path,
            DocumentState {
                text: source.into(),
            },
        );
        let mut output = Vec::new();
        state.validate_project(&root, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("publishDiagnostics"), "{output}");
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validation_reports_type_and_return_count_errors() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-semantic-errors-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"errors\"\n").unwrap();
        let source = "package main\nfunc broken() { var value: i32 = true; }\nfunc main() {}\n";
        let path = root.join("main.ond");
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        let first_output = String::from_utf8(output.clone()).unwrap();
        assert!(
            first_output.contains("requires bool type"),
            "{first_output}"
        );

        let return_error = "package main\nfunc broken() -> i32 { return; }\nfunc main() {}\n";
        output.clear();
        state
            .change_document(
                uri,
                vec![TextDocumentContentChangeEvent {
                    text: return_error.to_string(),
                    range: None,
                }],
                &mut output,
            )
            .unwrap();
        let second_output = String::from_utf8(output).unwrap();
        assert!(
            second_output.contains("return value count"),
            "{second_output}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_root_in_library_roots_does_not_disable_diagnostics() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-self-root-diagnostics-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"errors\"\n").unwrap();
        let source = "package main\nfunc main() { var value: i32 = true; }\n";
        let path = root.join("main.ond");
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::with_dependencies(&root, vec![root.clone()]);
        let mut output = Vec::new();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("requires bool type"), "{output}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_root_source_uses_manifest_namespace_with_open_buffer_features() {
        let root = test_project_root("library-root");
        std::fs::write(root.join("ond.toml"), "[project]\nname = \"console_v1\"\n").unwrap();
        let source = "package console_v1\nfunc Value() -> i32 { return 1; }\nfunc use() { var value: bool = Value(); }\n";
        std::fs::write(
            root.join("main.ond"),
            "package console_v1\nfunc Base() {}\n",
        )
        .unwrap();
        let path = root.join("api.ond");
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        let diagnostics = String::from_utf8(output).unwrap();
        assert!(diagnostics.contains("type mismatch"), "{diagnostics}");
        assert!(
            !diagnostics.contains("must declare package main"),
            "{diagnostics}"
        );

        let repaired = source.replace("value: bool", "value: i32");
        let mut repaired_output = Vec::new();
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: repaired.clone(),
                    range: None,
                }],
                &mut repaired_output,
            )
            .unwrap();
        let repaired_published = published_diagnostics(&repaired_output, &uri);
        assert!(!repaired_published.is_empty());
        assert!(
            repaired_published
                .iter()
                .all(|diagnostics| { diagnostics.as_array().is_some_and(Vec::is_empty) })
        );
        let project = state.load_current_project(&root).unwrap();
        assert!(compiler::compile_loaded_project(project).is_ok());

        let incomplete = "package console_v1\nfunc unfinished(";
        let mut incomplete_output = Vec::new();
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: incomplete.to_string(),
                    range: None,
                }],
                &mut incomplete_output,
            )
            .unwrap();
        assert!(!published_diagnostics(&incomplete_output, &uri).is_empty());

        let mut restored_output = Vec::new();
        state
            .change_document(
                uri.clone(),
                vec![TextDocumentContentChangeEvent {
                    text: repaired.clone(),
                    range: None,
                }],
                &mut restored_output,
            )
            .unwrap();
        let restored_published = published_diagnostics(&restored_output, &uri);
        assert!(!restored_published.is_empty());
        assert!(
            restored_published
                .iter()
                .all(|diagnostics| { diagnostics.as_array().is_some_and(Vec::is_empty) })
        );
        let use_line = repaired.lines().nth(2).unwrap();
        let call_character = use_line.find("Value").unwrap() as u32;
        let completions = state.completions(&uri, 2, call_character).unwrap();
        assert!(
            completions.iter().any(|item| item["label"] == "Value"),
            "{completions:?}"
        );
        let definition = state.definition(&uri, 2, call_character).unwrap();
        assert!(
            definition[0]["targetUri"]
                .as_str()
                .unwrap()
                .ends_with("api.ond"),
            "{definition}"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn external_library_inside_library_root_keeps_its_package_namespace() {
        let root = test_project_root("library-root-external-package");
        let libraries = root.join("vendor");
        std::fs::create_dir_all(libraries.join("shared")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname = \"console_v1\"\n").unwrap();
        std::fs::write(
            root.join("api.ond"),
            "package console_v1\nimport \"shared\"\nfunc Value() -> i32 { return shared.Value() }\n",
        )
        .unwrap();
        std::fs::write(
            libraries.join("shared/lib.ond"),
            "package shared\nfunc Value() -> i32 { return 7 }\n",
        )
        .unwrap();

        let state = ServerState::with_dependencies(&root, vec![libraries]);
        let project = state.load_current_project(&root).unwrap();
        assert!(compiler::compile_loaded_project(project.clone()).is_ok());
        assert!(
            project
                .packages
                .iter()
                .any(|package| package.logical_path == "shared")
        );
        assert!(
            !project
                .packages
                .iter()
                .any(|package| package.logical_path == "console_v1/shared")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_root_can_import_local_and_external_same_named_packages() {
        let root = test_project_root("library-root-local-and-external");
        let libraries = root.join("vendor");
        std::fs::create_dir_all(root.join("internal")).unwrap();
        std::fs::create_dir_all(libraries.join("internal")).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname = \"console_v1\"\n").unwrap();
        std::fs::write(
            root.join("api.ond"),
            "package console_v1\nimport ext \"internal\"\nimport own \"console_v1/internal\"\nfunc Value() -> i32 { return ext.External() + own.Local() }\n",
        )
        .unwrap();
        std::fs::write(
            root.join("internal/local.ond"),
            "package internal\nfunc Local() -> i32 { return 3 }\n",
        )
        .unwrap();
        std::fs::write(
            libraries.join("internal/api.ond"),
            "package internal\nfunc External() -> i32 { return 4 }\n",
        )
        .unwrap();

        let project = ServerState::with_dependencies(&root, vec![libraries.clone()])
            .load_current_project(&root)
            .unwrap();
        assert!(compiler::compile_loaded_project(project).is_ok());

        let conflicting = root.join("conflicting-vendor");
        std::fs::create_dir_all(conflicting.join("console_v1/internal")).unwrap();
        std::fs::write(
            conflicting.join("console_v1/internal/api.ond"),
            "package internal\nfunc Other() -> i32 { return 5 }\n",
        )
        .unwrap();
        let error = ServerState::with_dependencies(&root, vec![conflicting])
            .load_current_project(&root)
            .unwrap_err();
        assert!(error.contains("dependency namespace"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_internal_package_stays_separate_from_same_named_dependency() {
        let base = test_project_root("library-root-same-package");
        let root = base.join("lib");
        let dependency = base.join("dep");
        std::fs::create_dir_all(root.join("internal")).unwrap();
        std::fs::create_dir_all(&dependency).unwrap();
        std::fs::write(
            root.join("ond.toml"),
            "[project]\nname = \"console_v1\"\n[dependencies]\ninternal = { path = \"../dep\" }\n",
        )
        .unwrap();
        std::fs::write(
            dependency.join("ond.toml"),
            "[project]\nname = \"internal\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("api.ond"),
            "package console_v1\nimport ext \"internal\"\nimport own \"console_v1/internal\"\nfunc Value() -> i32 { return ext.External() + own.Local() }\n",
        )
        .unwrap();
        std::fs::write(
            root.join("internal/local.ond"),
            "package internal\nfunc Local() -> i32 { return 3 }\n",
        )
        .unwrap();
        std::fs::write(
            dependency.join("api.ond"),
            "package internal\nfunc External() -> i32 { return 4 }\n",
        )
        .unwrap();

        let project = ServerState::default().load_current_project(&root).unwrap();
        assert!(compiler::compile_loaded_project(project.clone()).is_ok());
        let local = project
            .packages
            .iter()
            .find(|package| package.logical_path == "console_v1/internal")
            .unwrap();
        assert!(
            project
                .sources
                .file(local.files[0])
                .path()
                .ends_with("local.ond")
        );
        let external = project
            .packages
            .iter()
            .find(|package| package.logical_path == "internal")
            .unwrap();
        assert!(
            project
                .sources
                .file(external.files[0])
                .path()
                .ends_with("api.ond")
        );
        assert!(
            project
                .sources
                .file(external.files[0])
                .path()
                .starts_with(&dependency)
        );
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn executable_root_still_reports_a_missing_main_function() {
        let root = test_project_root("executable-root");
        std::fs::write(root.join("ond.toml"), "[project]\nname = \"console_v1\"\n").unwrap();
        let source = "package main\nfunc helper() {}\n";
        let path = root.join("main.ond");
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();
        let diagnostics = String::from_utf8(output).unwrap();
        assert!(
            diagnostics.contains("must define `func main()`"),
            "{diagnostics}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_import_cycle_diagnostic_is_published_to_physical_source() {
        let root = test_project_root("library-import-cycle");
        std::fs::write(root.join("ond.toml"), "[project]\nname = \"console_v1\"\n").unwrap();
        std::fs::create_dir_all(root.join("internal")).unwrap();
        let source =
            "package console_v1\nimport \"console_v1/internal\"\nfunc Base() { internal.Use() }\n";
        let path = root.join("main.ond");
        std::fs::write(&path, source).unwrap();
        std::fs::write(
            root.join("internal/internal.ond"),
            "package internal\nimport \"console_v1\"\nfunc Use() { console_v1.Base() }\n",
        )
        .unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.to_string(), &mut output)
            .unwrap();
        let diagnostics = published_diagnostics(&output, &uri);
        assert!(
            diagnostics
                .iter()
                .any(|items| items.as_array().is_some_and(|items| {
                    items.iter().any(|item| {
                        item["message"]
                            .as_str()
                            .is_some_and(|message| message.contains("import cycle detected"))
                    })
                })),
            "{output:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rename_uses_open_buffers_and_edits_all_project_files() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-rename-project-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("ond.toml"),
            "[project]\nname = \"rename_project\"\n",
        )
        .unwrap();
        let declaration = "package main\nfunc Shared() {}\n";
        let use_from_disk = "package main\nfunc main() { Missing() }\n";
        let use_from_buffer = "package main\nfunc main() { Shared() }\n";
        std::fs::write(root.join("shared.ond"), declaration).unwrap();
        let main_path = root.join("main.ond");
        std::fs::write(&main_path, use_from_disk).unwrap();
        let uri = uri_from_path(&main_path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), use_from_buffer.to_string(), &mut output)
            .unwrap();

        let edit = state.rename(&uri, 1, 14, "Renamed").unwrap();
        let changes = edit["changes"].as_object().unwrap();
        assert_eq!(changes.len(), 2, "{edit}");
        assert_eq!(
            changes
                .values()
                .map(|edits| edits.as_array().unwrap().len())
                .sum::<usize>(),
            2,
            "{edit}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_loading_errors_are_published_as_diagnostics() {
        let base = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-load-error-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let root = base.join("project");
        let dependency = base.join("dependency");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&dependency).unwrap();
        std::fs::write(
            root.join("ond.toml"),
            "[project]\nname = \"app\"\n[dependencies]\nbroken = { path = \"../dependency\" }\n",
        )
        .unwrap();
        std::fs::write(dependency.join("ond.toml"), "").unwrap();
        let source = "package main\nfunc main() {}\n";
        let path = root.join("main.ond");
        std::fs::write(&path, source).unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("must declare `[project] name`"), "{output}");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn library_only_project_retains_all_source_filenames_and_private_context() {
        let root = test_project_root("library-only-tests");
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"portable\"\n").unwrap();
        std::fs::create_dir_all(root.join("deflate")).unwrap();
        std::fs::write(
            root.join("deflate/api.ond"),
            "package deflate\nfunc private(){}\n",
        )
        .unwrap();
        let test_path = root.join("deflate/api_test.ond");
        let source = "package deflate\nfunc TestPrivate(){private()}\n";
        std::fs::write(&test_path, source).unwrap();
        let uri = uri_from_path(&test_path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.into(), &mut output)
            .unwrap();
        let project = state.load_current_project(&root).unwrap();
        assert!(project.packages.iter().any(|p| {
            p.logical_path == "portable/deflate"
                && p.files
                    .iter()
                    .any(|file| same_path(project.sources.file(*file).path(), &test_path))
        }));
        assert!(compiler::compile_loaded_project(project).is_ok());
        let diagnostics = published_diagnostics(&output, &uri);
        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .all(|value| value.as_array().is_some_and(Vec::is_empty)),
            "{diagnostics:?}"
        );
        let definition = state.definition(&uri, 1, 20).unwrap();
        assert!(
            definition[0]["targetUri"]
                .as_str()
                .unwrap()
                .ends_with("api.ond"),
            "{definition}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_only_project_suffix_file_preserves_library_namespace() {
        let root = test_project_root("library-only-root-test");
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"portable\"\n").unwrap();
        std::fs::create_dir_all(root.join("deflate")).unwrap();
        std::fs::write(
            root.join("deflate/api.ond"),
            "package deflate\nfunc Value() -> u32 { return 7 }\n",
        )
        .unwrap();
        let test_path = root.join("api_test.ond");
        let source = "package portable\nimport \"portable/deflate\"\nfunc TestRoot(){var _=deflate.Value()}\n";
        std::fs::write(&test_path, source).unwrap();
        let uri = uri_from_path(&test_path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri.clone(), source.into(), &mut output)
            .unwrap();
        let project = state.load_current_project(&root).unwrap();
        assert!(project.packages.iter().any(|package| {
            package.logical_path == "portable"
                && package
                    .files
                    .iter()
                    .any(|file| same_path(project.sources.file(*file).path(), &test_path))
        }));
        assert!(
            project
                .packages
                .iter()
                .any(|package| package.logical_path == "portable/deflate")
        );
        assert!(compiler::compile_loaded_project(project).is_ok());
        let diagnostics = published_diagnostics(&output, &uri);
        assert!(
            diagnostics
                .iter()
                .all(|value| value.as_array().is_some_and(Vec::is_empty)),
            "{diagnostics:?}"
        );
        let value_character = source.lines().nth(2).unwrap().find("Value").unwrap() as u32;
        let definition = state.definition(&uri, 2, value_character).unwrap();
        assert!(
            definition[0]["targetUri"]
                .as_str()
                .unwrap()
                .ends_with("api.ond"),
            "{definition}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_only_project_reports_return_errors_in_subpackages() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-library-return-errors-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let package = root.join("deflate");
        let broken_package = root.join("zlib");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::create_dir_all(&broken_package).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"portable\"\n").unwrap();
        let source = "package deflate\nfunc kind(value: u8) -> (u8, *u8) {\nif value == 0 { return 0; }\n}\n";
        let path = package.join("api.ond");
        std::fs::write(&path, source).unwrap();
        std::fs::write(
            broken_package.join("zlib.ond"),
            "package zlib\nimport \"missing\"\ntype Broken missing.Value\n",
        )
        .unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("return value"), "{output}");
        assert!(output.contains("missing return"), "{output}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_only_project_reports_defined_type_comparison_mismatch() {
        let root = normalize_path(std::env::temp_dir().join(format!(
            "ond-lsp-library-defined-type-comparison-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
        let package = root.join("deflate");
        let imported_package = root.join("blob");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::create_dir_all(&imported_package).unwrap();
        std::fs::write(root.join("ond.toml"), "[project]\nname=\"portable\"\n").unwrap();
        let source = "package deflate\nimport \"portable/blob\"\ntype BlockType u8\nconst blockTypeNone: BlockType = 0\nfunc readType(byte: u8) -> BlockType {\nvalue := byte\nif value == blockTypeNone { return blockTypeNone }\nreturn blockTypeNone\n}\n";
        let path = package.join("api.ond");
        std::fs::write(&path, source).unwrap();
        std::fs::write(imported_package.join("blob.ond"), "package blob\n").unwrap();
        let uri = uri_from_path(&path).unwrap();
        let mut state = ServerState::default();
        let mut output = Vec::new();
        state
            .open_document(uri, source.to_string(), &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("type mismatch: expected"), "{output}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hover_returns_null_for_an_incomplete_document() {
        let path = normalize_path(
            std::env::current_dir()
                .expect("current directory")
                .join(PathBuf::from("incomplete-hover-test.ond")),
        );
        let uri = uri_from_path(&path).expect("test URI");
        let mut state = ServerState::default();
        state.documents.insert(
            path,
            DocumentState {
                text: "package main\nfunc unfinished(".to_string(),
            },
        );

        let hover = state
            .hover(&uri, 1, 5)
            .expect("hover request should not fail");
        assert_eq!(hover, Value::Null);
    }

    #[test]
    fn definition_uses_the_open_single_file_buffer() {
        let path = normalize_path(
            std::env::current_dir()
                .expect("current directory")
                .join(PathBuf::from("definition-buffer-test.ond")),
        );
        let uri = uri_from_path(&path).expect("test URI");
        let mut state = ServerState::default();
        state.documents.insert(
            path,
            DocumentState {
                text: "package main\nfunc run(value: u32) {\n    value = value\n}\n".to_string(),
            },
        );

        let definition = state.definition(&uri, 2, 4).expect("definition request");
        assert_eq!(definition[0]["targetSelectionRange"]["start"]["line"], 1);
    }

    #[test]
    fn definition_returns_null_for_an_incomplete_document() {
        let path = normalize_path(
            std::env::current_dir()
                .expect("current directory")
                .join(PathBuf::from("incomplete-definition-test.ond")),
        );
        let uri = uri_from_path(&path).expect("test URI");
        let mut state = ServerState::default();
        state.documents.insert(
            path,
            DocumentState {
                text: "package main\nfunc unfinished(".to_string(),
            },
        );
        assert_eq!(state.definition(&uri, 1, 5).unwrap(), Value::Null);
    }
}
