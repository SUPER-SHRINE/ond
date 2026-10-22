use crate::{DriverResult, input};
use ond_compiler_core::LoadedProject;
use std::collections::{BTreeMap, BTreeSet};

pub struct Graph {
    pub imports: BTreeMap<String, Vec<String>>,
    pub order: Vec<String>,
    pub initialization_order: Vec<String>,

    pub tests: Vec<ond_protocol::TestCase>,
}
impl Graph {
    pub fn load(loaded: &LoadedProject, application_root: Option<&str>) -> DriverResult<Self> {
        let ast =
            ond_compiler_core::ond::parse_project(loaded).map_err(|e| input::errors(e, loaded))?;
        let mut tests = Vec::new();
        for package in &ast.packages {
            for file in &package.files {
                if !ond_compiler_core::project::is_test_source(
                    loaded.sources.file(file.file_id).path(),
                ) {
                    continue;
                }
                for decl in &file.decls {
                    let ond_compiler_core::ir::ast::TopLevelDecl::Func(function) = decl else {
                        continue;
                    };
                    if !function.name.name.starts_with("Test") {
                        continue;
                    }
                    if function.receiver.is_some()
                        || !function.type_params.is_empty()
                        || !function.signature.params.is_empty()
                        || !function.signature.results.is_empty()
                    {
                        return Err(vec![format!(
                            "test function `{}` must be top-level with no parameters, results, type parameters or receiver",
                            function.name.name
                        )]);
                    }
                    tests.push(ond_protocol::TestCase {
                        package: package.logical_path.clone(),
                        name: function.name.name.clone(),
                        symbol: format!(
                            "{}.{}",
                            if application_root == Some(package.logical_path.as_str()) {
                                "main"
                            } else {
                                &package.logical_path
                            },
                            function.name.name
                        ),
                    });
                }
            }
        }
        let mut imports = BTreeMap::new();
        for package in ast.packages {
            let mut paths = Vec::new();
            for import in package.files.iter().flat_map(|f| &f.imports) {
                let path =
                    ond_compiler_core::decode_import_path(&import.path).map_err(|e| vec![e])?;
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            imports.insert(package.logical_path, paths);
        }
        let mut graph = Self::from_imports_with_root(imports, application_root).map_err(|errors| {
            errors
                .into_iter()
                .map(|error| {
                    let Some(path) = error.strip_prefix("unknown imported package: ") else {
                        return error;
                    };
                    match loaded.ignored_paths.iter().find(|ignored| {
                        path == ignored.as_str()
                            || path
                                .strip_prefix(ignored.as_str())
                                .is_some_and(|suffix| suffix.starts_with('/'))
                    }) {
                        Some(ignored) => format!(
                            "{error}\nnote: `{ignored}` is excluded from source discovery by ond.toml resolve.ignore"
                        ),
                        None => error,
                    }
                })
                .collect::<Vec<_>>()
        })?;

        graph.tests = tests;
        Ok(graph)
    }
    pub fn initialization_order_for(&self, path: &str) -> DriverResult<Vec<String>> {
        let mut order = Vec::new();
        visit(
            path,
            &self.imports,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut order,
        )?;
        Ok(order)
    }
    pub fn from_imports_with_root(
        imports: BTreeMap<String, Vec<String>>,
        root: Option<&str>,
    ) -> DriverResult<Self> {
        if let Some(root) = root
            && imports
                .values()
                .any(|dependencies| dependencies.iter().any(|path| path == root))
        {
            return Err(vec![format!("cannot import application root: {root}")]);
        }
        let mut graph = Self {
            imports,
            order: Vec::new(),
            initialization_order: Vec::new(),

            tests: Vec::new(),
        };
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        if let Some(root) = root {
            visit(
                root,
                &graph.imports,
                &mut visiting,
                &mut visited,
                &mut graph.order,
            )?;
        }
        if root.is_some() {
            graph.initialization_order = graph.order.clone();
        }
        // Preserve existing semantics: even currently unreachable packages are checked/emitted.
        for path in graph.imports.keys() {
            visit(
                path,
                &graph.imports,
                &mut visiting,
                &mut visited,
                &mut graph.order,
            )?;
        }
        Ok(graph)
    }
}
fn visit(
    path: &str,
    imports: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) -> DriverResult<()> {
    if visited.contains(path) {
        return Ok(());
    }
    if !visiting.insert(path.into()) {
        return Err(vec![format!("cyclic package import: {path}")]);
    }
    let dependencies = imports
        .get(path)
        .ok_or_else(|| vec![format!("unknown imported package: {path}")])?;
    for dep in dependencies {
        if dep == "." {
            return Err(vec!["importing root package is not allowed".into()]);
        }
        visit(dep, imports, visiting, visited, order)?;
    }
    visiting.remove(path);
    visited.insert(path.into());
    order.push(path.into());
    Ok(())
}
