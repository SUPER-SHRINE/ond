//! Declaration checks shared by independently validated source groups.
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::ir::{ast, hir};
use crate::project::{LoadedProject, PackageId};
use crate::source::{FileId, Span};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn duplicate_declarations(project: &ast::Project) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();
    for package in &project.packages {
        let mut names = BTreeMap::new();
        let mut methods = BTreeSet::new();
        let mut operators = BTreeSet::new();
        for declaration in package.files.iter().flat_map(|file| &file.decls) {
            let identifiers: Vec<&ast::Ident> = match declaration {
                ast::TopLevelDecl::Const(decl) => {
                    decl.specs.iter().flat_map(|spec| &spec.names).collect()
                }
                ast::TopLevelDecl::Var(decl) => {
                    decl.specs.iter().flat_map(|spec| &spec.names).collect()
                }
                ast::TopLevelDecl::Type(decl) => decl.specs.iter().map(|spec| &spec.name).collect(),
                ast::TopLevelDecl::Func(decl) => {
                    if let Some(receiver) = decl.receiver.as_ref() {
                        if let Some((receiver, _)) = super::types::method_receiver_name(receiver)
                            && !methods.insert((receiver.to_string(), decl.name.name.clone()))
                        {
                            diagnostics.push(Diagnostic::error(
                                decl.name.span,
                                format!("duplicate method `{receiver}.{}`", decl.name.name),
                            ));
                        }
                        Vec::new()
                    } else {
                        vec![&decl.name]
                    }
                }
                ast::TopLevelDecl::Operator(decl) => {
                    if let Some(receiver) = decl.signature.params.first()
                        && let Some((receiver, _)) = super::types::method_receiver_name(receiver)
                        && !operators.insert((
                            receiver.to_string(),
                            decl.name,
                            decl.signature.params.len(),
                        ))
                    {
                        diagnostics.push(Diagnostic::error(
                            decl.span,
                            "duplicate operator declaration for the same receiver and arity",
                        ));
                    }
                    Vec::new()
                }
            };
            for name in identifiers {
                report_duplicate_top_level(&mut diagnostics, &mut names, &name.name, name.span);
            }
        }
    }
    diagnostics
}

pub(super) fn collect_symbols(
    ast: &ast::Project,
    logical_to_package: &BTreeMap<String, PackageId>,
) -> Vec<hir::Symbol> {
    let mut symbols = Vec::new();
    for package in &ast.packages {
        let package_id = logical_to_package[&package.logical_path];
        for file in &package.files {
            for declaration in &file.decls {
                let (kind, names): (_, Vec<&ast::Ident>) = match declaration {
                    ast::TopLevelDecl::Const(decl) => (
                        hir::SymbolKind::Constant,
                        decl.specs
                            .iter()
                            .flat_map(|spec| spec.names.iter())
                            .collect(),
                    ),
                    ast::TopLevelDecl::Var(decl) => (
                        hir::SymbolKind::Global,
                        decl.specs
                            .iter()
                            .flat_map(|spec| spec.names.iter())
                            .collect(),
                    ),
                    ast::TopLevelDecl::Type(decl) => (
                        hir::SymbolKind::Type,
                        decl.specs.iter().map(|spec| &spec.name).collect(),
                    ),
                    ast::TopLevelDecl::Func(decl) => (
                        hir::SymbolKind::Function,
                        decl.receiver
                            .is_none()
                            .then_some(&decl.name)
                            .into_iter()
                            .collect(),
                    ),
                    ast::TopLevelDecl::Operator(_) => (hir::SymbolKind::Function, Vec::new()),
                };
                for name in names {
                    if name.name == "_" {
                        continue;
                    }
                    symbols.push(hir::Symbol {
                        id: hir::SymbolId(symbols.len() as u32),
                        package: package_id,
                        name: name.name.clone(),
                        canonical_name: super::canonical_symbol_name(
                            &package.logical_path,
                            &name.name,
                        ),
                        kind,
                        exported: name
                            .name
                            .as_bytes()
                            .first()
                            .is_some_and(u8::is_ascii_uppercase),
                        span: name.span,
                    });
                }
            }
        }
    }
    symbols
}

pub(super) fn import_declaration_conflicts(
    owner: &super::TypePackage,
    symbols: &[hir::Symbol],
    imports: &BTreeMap<FileId, BTreeMap<String, PackageId>>,
) -> Diagnostics {
    // Declaration visibility does not depend on which bodies are being lowered.
    let names = symbols
        .iter()
        .filter(|symbol| symbol.package == owner.id)
        .map(|symbol| (symbol.name.as_str(), symbol.span))
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Diagnostics::new();
    for (file, imports) in imports {
        for name in imports.keys() {
            if let Some(declaration) = names.get(name.as_str())
                && owner
                    .import_spans
                    .get(&(*file, name.clone()))
                    .is_some_and(|import| {
                        owner.declaration_conflicts_with_import(*declaration, *import)
                    })
            {
                diagnostics.push(Diagnostic::error(
                    *declaration,
                    format!("top-level declaration conflicts with imported package `{name}`"),
                ));
            }
        }
    }
    diagnostics
}

pub(super) fn validate_project(
    loaded: &LoadedProject,
    ast: &ast::Project,
    selected: Option<&BTreeSet<crate::source::FileId>>,
    packages: &[hir::Package],
    symbols: &[hir::Symbol],
    diagnostics: &mut Diagnostics,
) {
    // Semantic analysis consumes the loaded snapshot, including in-memory inputs.
    // Filesystem discovery/validation belongs to the project loader.
    let root_main = loaded
        .packages
        .iter()
        .filter(|p| p.logical_path == ".")
        .any(|package| {
            package
                .files
                .iter()
                .any(|id| loaded.sources.file(*id).path() == package.directory.join("main.ond"))
        });
    let check_entry = selected.is_none_or(|selected| {
        ast.packages.iter().any(|package| {
            package.logical_path == "."
                && (selected.is_empty()
                    || package
                        .files
                        .iter()
                        .any(|file| selected.contains(&file.file_id)))
        })
    });
    if check_entry && !root_main {
        diagnostics.push(Diagnostic::error(
            Span::synthetic(),
            "project root must contain `main.ond`",
        ));
    }

    let main_package = packages.iter().find(|package| package.logical_path == ".");
    let main_functions = main_package
        .map(|package| {
            package
                .items
                .iter()
                .filter_map(|item| match item {
                    hir::Item::Func(function) if function.name == "main" => Some(function),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if check_entry
        && main_functions.is_empty()
        && !symbols
            .iter()
            .any(|s| s.canonical_name == "main.main" && s.kind == hir::SymbolKind::Function)
    {
        diagnostics.push(Diagnostic::error(
            Span::synthetic(),
            "`main` package must define `func main()`",
        ));
    } else if check_entry {
        for function in main_functions {
            if !function.signature.parameters.is_empty() || !function.signature.returns.is_empty() {
                diagnostics.push(Diagnostic::error(
                    function.span,
                    "`func main()` must not have parameters or return values",
                ));
            }
        }
    }

    if selected.is_none() {
        diagnostics.extend(import_cycles(ast));
    }
}

pub(super) fn report_duplicate_top_level(
    diagnostics: &mut Diagnostics,
    seen: &mut BTreeMap<String, Span>,
    name: &str,
    span: Span,
) {
    if name == "_" {
        return;
    }
    if let Some(previous) = seen.insert(name.to_string(), span) {
        diagnostics.push(
            Diagnostic::error(span, format!("duplicate top-level declaration `{name}`")).with_note(
                format!(
                    "previous declaration starts at byte {} in the same package",
                    previous.start
                ),
            ),
        );
    }
}

pub(super) fn import_cycles(project: &ast::Project) -> Diagnostics {
    let graph = project
        .packages
        .iter()
        .map(|package| {
            let mut imports = Vec::new();
            for import in package.files.iter().flat_map(|file| &file.imports) {
                if import.alias.as_ref().is_some_and(|alias| alias.name == "_") {
                    continue;
                }
                if let Ok(path) = crate::string_literal::import_path(&import.path)
                    && path != "."
                    && project
                        .packages
                        .iter()
                        .any(|package| package.logical_path == path)
                    && !imports.contains(&path)
                {
                    imports.push(path);
                }
            }
            (package.logical_path.as_str(), imports)
        })
        .collect::<BTreeMap<_, _>>();
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Visit {
        Visiting,
        Done,
    }

    fn dfs(
        package: &str,
        graph: &BTreeMap<&str, Vec<String>>,
        states: &mut BTreeMap<String, Visit>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        states.insert(package.to_string(), Visit::Visiting);
        stack.push(package.to_string());
        for import in &graph[package] {
            match states.get(import).copied() {
                Some(Visit::Visiting) => {
                    let start = stack.iter().position(|id| id == import).unwrap_or(0);
                    let cycle = stack[start..]
                        .iter()
                        .chain(std::iter::once(import))
                        .cloned()
                        .collect::<Vec<_>>();
                    return Some(cycle);
                }
                Some(Visit::Done) => {}
                None => {
                    if let Some(cycle) = dfs(import, graph, states, stack) {
                        return Some(cycle);
                    }
                }
            }
        }

        stack.pop();
        states.insert(package.to_string(), Visit::Done);
        None
    }

    let mut states = BTreeMap::new();
    let mut stack = Vec::new();
    let mut diagnostics = Diagnostics::new();
    for package in &project.packages {
        if states.contains_key(&package.logical_path) {
            continue;
        }
        if let Some(cycle) = dfs(&package.logical_path, &graph, &mut states, &mut stack) {
            diagnostics.push(Diagnostic::error(
                Span::synthetic(),
                format!("import cycle detected: {}", cycle.join(" -> ")),
            ));
            break;
        }
    }
    diagnostics
}
