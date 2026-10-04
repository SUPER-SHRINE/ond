use crate::{DriverResult, input};
use ond_compiler_core::LoadedProject;
use std::collections::{BTreeMap, BTreeSet};

pub struct Graph {
    pub imports: BTreeMap<String, Vec<String>>,
    pub order: Vec<String>,
    pub initialization_order: Vec<String>,
}
impl Graph {
    pub fn load(loaded: &LoadedProject) -> DriverResult<Self> {
        let ast =
            ond_compiler_core::ond::parse_project(loaded).map_err(|e| input::errors(e, loaded))?;
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
        Self::from_imports(imports).map_err(|errors| {
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
                .collect()
        })
    }
    pub fn from_imports(imports: BTreeMap<String, Vec<String>>) -> DriverResult<Self> {
        if !imports.contains_key(".") {
            return Err(vec!["missing root package".into()]);
        }
        let mut graph = Self {
            imports,
            order: Vec::new(),
            initialization_order: Vec::new(),
        };
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        visit(
            ".",
            &graph.imports,
            &mut visiting,
            &mut visited,
            &mut graph.order,
        )?;
        graph.initialization_order = graph.order.clone();
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
