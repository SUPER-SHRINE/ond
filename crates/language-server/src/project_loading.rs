use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use compiler::source::FileId;

use crate::util::{find_project_root, normalize_path, render_frontend_error, same_path};

pub(super) fn load_project(
    root: &Path,
    overrides: &BTreeMap<PathBuf, String>,
    library_roots: &[PathBuf],
) -> Result<compiler::LoadedProject, String> {
    let resolved =
        ond_cli::resolve_dependencies(root).map_err(|diagnostics| diagnostics.join("\n"))?;
    let normalized_root = normalize_path(root.to_path_buf());
    let library_roots = library_roots
        .iter()
        .filter(|library| !same_path(library, &normalized_root))
        .cloned()
        .collect::<Vec<_>>();
    let has_root_sources = std::fs::read_dir(root)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .any(|entry| {
            entry.path().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("ond")
        });
    let self_sources = ond_cli::resolve_project_sources(root).ok();
    let library_namespace = if has_root_sources {
        self_sources
            .as_ref()
            .and_then(|sources| library_root_namespace(root, &sources.sources, overrides))
    } else {
        None
    };
    let own_library_packages = library_namespace
        .as_ref()
        .and(self_sources.as_ref())
        .map(|sources| {
            sources
                .sources
                .keys()
                .filter_map(|name| {
                    name.rsplit_once('/')
                        .map(|(package, _)| package.to_string())
                })
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let library_project = if library_namespace.is_some() {
        root.join(".ond-lsp")
    } else {
        root.to_path_buf()
    };
    let mut libraries = compiler::load_library_sources(&library_project, &library_roots, overrides)
        .map_err(render_frontend_error)?;
    for name in libraries.keys() {
        if let Some((package, _)) = name.rsplit_once('/')
            && own_library_packages.contains(package)
        {
            return Err(format!("library conflicts with project package: {package}"));
        }
    }
    let mut excluded_library_roots = library_roots
        .iter()
        .map(|library| normalize_path(library.clone()))
        .filter(|library| {
            library.starts_with(&normalized_root) && !same_path(library, &normalized_root)
        })
        .collect::<Vec<_>>();
    let mut required_sources = BTreeMap::new();
    excluded_library_roots.extend(resolved.ignored_directories);
    merge_external_sources(&mut libraries, resolved.sources, overrides)?;
    let external_paths = libraries
        .values()
        .map(|source| normalize_path(source.path.clone()))
        .collect::<BTreeSet<_>>();

    if let Some(self_sources) = self_sources {
        if has_root_sources {
            if library_namespace.is_some() {
                excluded_library_roots.extend(self_sources.ignored_directories);
            }
        } else {
            let packages = self_sources
                .sources
                .keys()
                .filter_map(|name| name.rsplit_once('/').map(|(package, _)| package))
                .collect::<BTreeSet<_>>();
            let mut synthetic_main = String::from("package main\n");
            for (index, package) in packages.into_iter().enumerate() {
                synthetic_main.push_str(&format!("import __ond_lsp_{index} \"{package}\"\n"));
            }
            synthetic_main.push_str("func main() {}\n");
            required_sources.insert("main.ond".to_string(), synthetic_main);
            for source in self_sources.sources.values() {
                if let Ok(relative) = source.path.strip_prefix(root)
                    && let Some(Component::Normal(directory)) = relative.components().next()
                {
                    let directory = root.join(directory);
                    if directory.is_dir() && !excluded_library_roots.contains(&directory) {
                        excluded_library_roots.push(directory);
                    }
                }
            }
            excluded_library_roots.extend(self_sources.ignored_directories);
            merge_external_sources(&mut libraries, self_sources.sources, overrides)?;
        }
    }
    let mut project = compiler::load_project_with_inputs_allow_incomplete(
        root,
        overrides,
        &excluded_library_roots,
        &required_sources,
        &BTreeMap::new(),
        &libraries,
    )
    .map_err(render_frontend_error)?;
    if let Some(namespace) = library_namespace {
        let mut packages = BTreeMap::<String, compiler::PackageSources>::new();
        for package in std::mem::take(&mut project.packages) {
            for file in package.files {
                let source = project.sources.file(file);
                let is_external =
                    external_paths.contains(&normalize_path(source.path().to_path_buf()));
                let is_synthetic_main = package.logical_path == "." && !source.path().is_file();
                let logical_path = if is_external || is_synthetic_main {
                    package.logical_path.clone()
                } else if package.logical_path == "." {
                    namespace.clone()
                } else {
                    format!("{namespace}/{}", package.logical_path)
                };
                let entry = packages.entry(logical_path.clone()).or_insert_with(|| {
                    compiler::PackageSources {
                        id: compiler::PackageId(0),
                        logical_path,
                        directory: package.directory.clone(),
                        files: Vec::new(),
                    }
                });
                entry.files.push(file);
            }
        }
        project.packages = packages.into_values().collect();
        project.root = root.join(".ond-lsp");
        let file = project.sources.add_file(
            project.root.join("main.ond"),
            format!("package main\nimport \"{namespace}\"\nfunc main() {{}}\n"),
        );
        project.packages.push(compiler::PackageSources {
            id: compiler::PackageId(project.packages.len()),
            logical_path: ".".to_string(),
            directory: root.to_path_buf(),
            files: vec![file],
        });
        project
            .packages
            .sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
        for (index, package) in project.packages.iter_mut().enumerate() {
            package.id = compiler::PackageId(index);
        }
    }
    Ok(project)
}

pub(super) fn synthetic_diagnostic_path(
    original_root: &Path,
    project: &compiler::LoadedProject,
) -> PathBuf {
    if same_path(original_root, &project.root) {
        return project.root.join("main.ond");
    }
    let mut sources = project
        .sources
        .files()
        .iter()
        .map(|source| source.path())
        .filter(|path| {
            path.is_file()
                && find_project_root(path)
                    .is_some_and(|source_root| same_path(&source_root, original_root))
        })
        .collect::<Vec<_>>();
    sources.sort_by_key(|path| {
        !(path.file_name().is_some_and(|name| name == "main.ond")
            && path
                .parent()
                .is_some_and(|parent| same_path(parent, original_root)))
    });
    sources
        .first()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| original_root.join("main.ond"))
}

fn declares_package(source: &str, expected: &str) -> bool {
    let mut prefix = String::new();
    for line in source.split_inclusive('\n') {
        prefix.push_str(line);
        if let Ok(file) = compiler::parse_source_file(FileId(0), &prefix) {
            return file.package.name == expected;
        }
    }
    false
}

fn library_root_namespace(
    root: &Path,
    sources: &compiler::ExternalSources,
    overrides: &BTreeMap<PathBuf, String>,
) -> Option<String> {
    let namespace = sources.iter().find_map(|(name, source)| {
        same_path(source.path.parent()?, root)
            .then(|| {
                name.split_once('/')
                    .map(|(namespace, _)| namespace.to_string())
            })
            .flatten()
    })?;
    if namespace == "main" {
        return None;
    }
    let mut found = false;
    for source in sources.values().filter(|source| {
        source
            .path
            .parent()
            .is_some_and(|parent| same_path(parent, root))
    }) {
        let text = overrides
            .get(&normalize_path(source.path.clone()))
            .cloned()
            .unwrap_or_else(|| source.text.clone());
        if !declares_package(&text, &namespace) {
            return None;
        }
        found = true;
    }
    found.then_some(namespace)
}

fn merge_external_sources(
    target: &mut compiler::ExternalSources,
    sources: compiler::ExternalSources,
    overrides: &BTreeMap<PathBuf, String>,
) -> Result<(), String> {
    for (name, mut source) in sources {
        if let Some(text) = overrides.get(&source.path).or_else(|| {
            source
                .path
                .canonicalize()
                .ok()
                .and_then(|path| overrides.get(&path))
        }) {
            source.text = text.clone();
        }
        if let Some(previous) = target.insert(name.clone(), source.clone())
            && previous != source
        {
            return Err(format!("conflicting external source: {name}"));
        }
    }
    Ok(())
}
