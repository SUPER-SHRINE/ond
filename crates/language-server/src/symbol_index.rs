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
                    .flat_map(|file| file.decls.iter())
                    .flat_map(indexed_symbols_for_decl)
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
        let mut parsed_files = BTreeMap::new();
        for (index, source) in loaded.sources.files().iter().enumerate() {
            let path = source.path().to_path_buf();
            let text = source.text().to_string();
            let file_id = FileId(index);
            let indexed_text = match compiler::parse_source_file(file_id, &text) {
                Ok(file) => {
                    last_good_sources.insert(path.clone(), text.clone());
                    parsed_files.insert(file_id, file);
                    text
                }
                Err(_) => {
                    failed_files.insert(path.clone());
                    if let Some(previous) =
                        previous.and_then(|previous| previous.last_good_sources.get(&path))
                    {
                        last_good_sources.insert(path.clone(), previous.clone());
                        if let Ok(file) = compiler::parse_source_file(file_id, previous) {
                            parsed_files.insert(file_id, file);
                        }
                        previous.clone()
                    } else {
                        text
                    }
                }
            };
            sources.add_file(path, indexed_text);
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
        ast::Type::Interface { .. } => "interface".to_string(),
        ast::Type::Func { .. } => "func".to_string(),
    }
}

fn is_exported(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase())
}
