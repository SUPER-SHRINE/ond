use compiler::ir::ast;
use compiler::source::{FileId, SourceDb};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedSymbolKind {
    Function,
    Constant,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: IndexedSymbolKind,
    pub detail: String,
    pub exported: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectSymbolIndex {
    pub(crate) loaded: compiler::LoadedProject,
    pub(crate) ast: ast::Project,
    packages: BTreeMap<String, Vec<IndexedSymbol>>,
    last_good_sources: BTreeMap<PathBuf, String>,
    failed_files: BTreeSet<PathBuf>,
}

impl ProjectSymbolIndex {
    fn with_file_state(
        loaded: compiler::LoadedProject,
        ast: ast::Project,
        last_good_sources: BTreeMap<PathBuf, String>,
        failed_files: BTreeSet<PathBuf>,
    ) -> Self {
        let packages = ast
            .packages
            .iter()
            .map(|package| {
                let symbols = package
                    .files
                    .iter()
                    .flat_map(|file| {
                        let test = compiler::project::is_test_source(
                            loaded.sources.file(file.file_id).path(),
                        );
                        file.decls.iter().flat_map(indexed_symbols_for_decl).map(
                            move |mut symbol| {
                                symbol.exported &= !test;
                                symbol
                            },
                        )
                    })
                    .collect();
                (package.logical_path.clone(), symbols)
            })
            .collect();
        Self {
            loaded,
            ast,
            packages,
            last_good_sources,
            failed_files,
        }
    }

    pub(crate) fn refresh(loaded: compiler::LoadedProject, previous: Option<&Self>) -> Self {
        let mut last_good_sources = BTreeMap::new();
        let mut failed_files = BTreeSet::new();
        let mut sources = SourceDb::default();
        sources.set_dependency_inputs(loaded.sources.dependency_inputs().clone());
        let mut parsed_files = BTreeMap::new();
        for (index, source) in loaded.sources.files().iter().enumerate() {
            let path = source.path().to_path_buf();
            let text = source.text().to_string();
            let file_id = FileId(index);
            let indexed_text =
                match compiler::ond::parse_context_file(&loaded.sources, file_id, true) {
                    Ok(file) => {
                        last_good_sources.insert(path.clone(), text.clone());
                        parsed_files.insert(file_id, file);
                        text
                    }
                    Err(_) => {
                        failed_files.insert(path.clone());
                        if let Some(previous) = previous
                            .filter(|previous| {
                                if previous.loaded.sources.dependency_inputs()
                                    != loaded.sources.dependency_inputs()
                                {
                                    return false;
                                }
                                previous
                                    .loaded
                                    .sources
                                    .files()
                                    .iter()
                                    .position(|source| source.path() == path)
                                    .is_some_and(|index| {
                                        previous.loaded.sources.roles(FileId(index))
                                            == loaded.sources.roles(file_id)
                                    })
                            })
                            .and_then(|previous| previous.last_good_sources.get(&path))
                        {
                            last_good_sources.insert(path.clone(), previous.clone());
                            let mut prior = compiler::source::SourceDb::default();
                            for source in loaded.sources.files().iter().take(index + 1) {
                                prior.add_file(
                                    source.path().into(),
                                    if source.path() == path {
                                        previous.clone()
                                    } else {
                                        source.text().into()
                                    },
                                );
                            }
                            if let Some(roles) = loaded.sources.roles(file_id) {
                                prior.set_roles(file_id, roles.clone());
                            }
                            if let Ok(file) =
                                compiler::ond::parse_context_file(&prior, file_id, true)
                            {
                                parsed_files.insert(file_id, file);
                            }
                            previous.clone()
                        } else {
                            text
                        }
                    }
                };
            let id = sources.add_file(path, indexed_text);
            if let Some(roles) = loaded.sources.roles(file_id) {
                sources.set_roles(id, roles.clone());
            }
        }

        let mut loaded = loaded;
        loaded.sources = sources;
        let packages = loaded
            .packages
            .iter()
            .map(|package| ast::Package {
                logical_path: package.logical_path.clone(),
                files: package
                    .files
                    .iter()
                    .filter_map(|file_id| parsed_files.remove(file_id))
                    .collect(),
            })
            .collect();
        let ast = ast::Project { packages };
        Self::with_file_state(loaded, ast, last_good_sources, failed_files)
    }

    pub(crate) fn for_source(mut self, path: &std::path::Path) -> Self {
        let selected = compiler::project::is_test_source(path)
            .then(|| {
                self.ast
                    .packages
                    .iter()
                    .find(|package| {
                        package.files.iter().any(|file| {
                            crate::util::same_path(
                                self.loaded.sources.file(file.file_id).path(),
                                path,
                            )
                        })
                    })
                    .map(|p| p.logical_path.clone())
            })
            .flatten();
        for package in &mut self.ast.packages {
            if selected.as_ref() != Some(&package.logical_path) {
                package.files.retain(|file| {
                    !compiler::project::is_test_source(
                        self.loaded.sources.file(file.file_id).path(),
                    )
                });
            }
        }
        Self::with_file_state(
            self.loaded,
            self.ast,
            self.last_good_sources,
            self.failed_files,
        )
    }

    pub(crate) fn for_position(
        mut self,
        path: &std::path::Path,
        line: u32,
        character: u32,
    ) -> Self {
        if let Some((index, source)) = self
            .loaded
            .sources
            .files()
            .iter()
            .enumerate()
            .find(|(_, source)| crate::util::same_path(source.path(), path))
            && let Ok(offset) = crate::protocol::lsp_offset(
                source.text(),
                &crate::protocol::Position { line, character },
            )
        {
            self.ast = crate::role_context::project_at(
                &self.ast,
                &self.loaded.sources,
                compiler::source::Span::new(FileId(index), offset, offset),
            );
        }
        Self::with_file_state(
            self.loaded,
            self.ast,
            self.last_good_sources,
            self.failed_files,
        )
    }

    pub(crate) fn file_failed_to_parse(&self, path: &std::path::Path) -> bool {
        self.failed_files
            .iter()
            .any(|failed| crate::util::same_path(failed, path))
    }

    #[cfg(test)]
    pub(crate) fn last_good_source(&self, path: &std::path::Path) -> Option<&str> {
        self.last_good_sources
            .iter()
            .find(|(candidate, _)| crate::util::same_path(candidate, path))
            .map(|(_, source)| source.as_str())
    }

    pub fn package_symbols(&self, logical_path: &str) -> &[IndexedSymbol] {
        self.packages
            .get(logical_path)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

fn indexed_symbols_for_decl(decl: &ast::TopLevelDecl) -> Vec<IndexedSymbol> {
    match decl {
        ast::TopLevelDecl::Const(decl) => decl
            .specs
            .iter()
            .flat_map(|spec| {
                spec.names.iter().map(|name| IndexedSymbol {
                    name: name.name.clone(),
                    kind: IndexedSymbolKind::Constant,
                    detail: spec.ty.as_ref().map_or_else(
                        || "const".to_string(),
                        |ty| format!("const {}: {}", name.name, type_name(ty)),
                    ),
                    exported: is_exported(&name.name),
                })
            })
            .collect(),
        ast::TopLevelDecl::Type(decl) => decl
            .specs
            .iter()
            .map(|spec| IndexedSymbol {
                name: spec.name.name.clone(),
                kind: IndexedSymbolKind::Type,
                detail: format!("type {} {}", spec.name.name, type_name(&spec.ty)),
                exported: is_exported(&spec.name.name),
            })
            .collect(),
        ast::TopLevelDecl::Func(decl) if decl.receiver.is_none() => vec![IndexedSymbol {
            name: decl.name.name.clone(),
            kind: IndexedSymbolKind::Function,
            detail: function_signature(decl),
            exported: is_exported(&decl.name.name),
        }],
        ast::TopLevelDecl::Func(_) => Vec::new(),
        ast::TopLevelDecl::Var(_) => Vec::new(),
        ast::TopLevelDecl::Operator(_) => Vec::new(),
    }
}

fn function_signature(function: &ast::FuncDecl) -> String {
    let params = function
        .signature
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name.name, type_name(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut signature = format!("{}({params})", function.name.name);
    match function.signature.results.as_slice() {
        [] => {}
        [result] => signature.push_str(&format!(" -> {}", type_name(result))),
        results => {
            let results = results.iter().map(type_name).collect::<Vec<_>>().join(", ");
            signature.push_str(&format!(" -> ({results})"));
        }
    }
    signature
}

fn type_name(ty: &ast::Type) -> String {
    match ty {
        ast::Type::Named(path) => path
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("."),
        ast::Type::Apply {
            base, arguments, ..
        } => format!(
            "{}[{}]",
            base.segments
                .iter()
                .map(|segment| segment.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            arguments
                .iter()
                .map(type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ast::Type::Resolved(_, _) => "<resolved>".to_string(),
        ast::Type::Pointer(inner, _) => format!("*{}", type_name(inner)),
        ast::Type::Array { element, .. } => format!("[...]{}", type_name(element)),
        ast::Type::Struct { .. } => "struct".to_string(),
        ast::Type::Func { .. } => "func".to_string(),
    }
}

fn is_exported(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
}

#[cfg(test)]
mod source_filename_tests {
    use super::*;
    #[test]
    fn source_index_keeps_package_private_context_and_exports_public_declarations() {
        let root = std::path::Path::new("test-build-index").to_path_buf();
        let mut sources = SourceDb::default();
        let main = sources.add_file(
            root.join("main.ond"),
            "package main\nfunc private(){}\nfunc main(){}".into(),
        );
        let tests = sources.add_file(
            root.join("main_test.ond"),
            "package main\nfunc TestLocal(){private()}\nfunc Helper(){}".into(),
        );
        let other = sources.add_file(
            root.join("other/other_test.ond"),
            "package other\nfunc TestOther(){}".into(),
        );
        let loaded = compiler::LoadedProject {
            root: root.clone(),
            manifest_path: root.join("ond.toml"),
            ignored_paths: Vec::new(),
            sources,
            packages: vec![
                compiler::PackageSources {
                    id: compiler::PackageId(0),
                    logical_path: ".".into(),
                    directory: root.clone(),
                    files: vec![main, tests],
                },
                compiler::PackageSources {
                    id: compiler::PackageId(1),
                    logical_path: "other".into(),
                    directory: root.join("other"),
                    files: vec![other],
                },
            ],
        };
        let index = ProjectSymbolIndex::refresh(loaded, None);
        let normal = index.clone().for_source(&root.join("main.ond"));
        assert!(
            normal
                .package_symbols(".")
                .iter()
                .any(|s| s.name == "Helper" && s.exported)
        );
        let test = index.for_source(&root.join("main_test.ond"));
        assert!(
            test.package_symbols(".")
                .iter()
                .any(|s| s.name == "private")
        );
        assert!(
            test.package_symbols(".")
                .iter()
                .any(|s| s.name == "Helper" && s.exported)
        );
        assert!(
            test.package_symbols("other")
                .iter()
                .any(|s| s.name == "TestOther" && s.exported)
        );
    }
}
