use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::diagnostic::{Diagnostic, FrontendError};
use crate::project::{
    LoadedProject, PackageId, PackageSources, load_project_with_overrides_and_exclusions,
};
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSource {
    pub path: PathBuf,
    pub text: String,
}

pub type ExternalSources = BTreeMap<String, ExternalSource>;

/// Read optional packages from file-backed library roots.
///
/// Logical source names are relative to each library root. Packages supplied by
/// multiple roots must be byte-for-byte identical, and a root may not contain
/// `.ond` files directly.
pub fn load_library_sources(
    project: &Path,
    roots: &[PathBuf],
    overrides: &BTreeMap<PathBuf, String>,
) -> Result<ExternalSources, FrontendError> {
    let mut result = ExternalSources::new();
    let mut packages = BTreeMap::<String, BTreeMap<String, ExternalSource>>::new();

    for root in roots {
        let root = root
            .canonicalize()
            .map_err(|error| failure(format!("library {}: {error}", root.display())))?;
        let mut files = ExternalSources::new();
        scan_library_root(&root, &root, overrides, &mut files)?;

        let mut groups = BTreeMap::<String, ExternalSources>::new();
        for (name, source) in files {
            let package = package_for_source_name(&name);
            groups.entry(package).or_default().insert(name, source);
        }

        for (package, files) in groups {
            if package == "." {
                return Err(failure(
                    "library root must contain package subdirectories, not root .ond files",
                ));
            }
            if let Some(previous) = packages.get(&package) {
                if source_texts(previous) != source_texts(&files) {
                    return Err(failure(format!("conflicting library package: {package}")));
                }
                continue;
            }

            let directory = project.join(&package);
            if directory.is_dir() {
                let existing = read_project_package(&directory, &package, overrides)?;
                if !existing.is_empty() {
                    if existing != source_texts(&files) {
                        return Err(failure(format!(
                            "library conflicts with project package: {package}"
                        )));
                    }
                    packages.insert(package, files);
                    continue;
                }
            }

            result.extend(files.clone());
            packages.insert(package, files);
        }
    }

    Ok(result)
}

/// Load a project and merge required virtual sources, optional virtual sources,
/// and file-backed external libraries using the same rules for every frontend.
pub fn load_project_with_inputs(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
    required_sources: &BTreeMap<String, String>,
    optional_sources: &BTreeMap<String, String>,
    external_sources: &ExternalSources,
) -> Result<LoadedProject, FrontendError> {
    load_project_with_inputs_impl(
        root,
        overrides,
        excluded_dirs,
        required_sources,
        optional_sources,
        external_sources,
        false,
    )
}

/// Load the same inputs as [`load_project_with_inputs`], but retain all optional
/// packages when an in-memory edit makes the project temporarily unparsable.
pub fn load_project_with_inputs_allow_incomplete(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
    required_sources: &BTreeMap<String, String>,
    optional_sources: &BTreeMap<String, String>,
    external_sources: &ExternalSources,
) -> Result<LoadedProject, FrontendError> {
    load_project_with_inputs_impl(
        root,
        overrides,
        excluded_dirs,
        required_sources,
        optional_sources,
        external_sources,
        true,
    )
}

fn load_project_with_inputs_impl(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
    required_sources: &BTreeMap<String, String>,
    optional_sources: &BTreeMap<String, String>,
    external_sources: &ExternalSources,
    allow_incomplete: bool,
) -> Result<LoadedProject, FrontendError> {
    let mut loaded = load_project_with_overrides_and_exclusions(root, overrides, excluded_dirs)?;
    loaded.packages.retain(|package| {
        !excluded_dirs
            .iter()
            .any(|directory| package.directory.starts_with(directory))
    });

    let physical = loaded
        .packages
        .iter()
        .map(|package| package.logical_path.clone())
        .collect::<BTreeSet<_>>();
    let required = required_sources
        .keys()
        .map(|name| package_for_source_name(name))
        .collect::<BTreeSet<_>>();
    let mut optional = BTreeSet::new();
    let mut logical_sources = BTreeSet::new();

    for (name, text) in required_sources {
        if optional_sources.contains_key(name) || external_sources.contains_key(name) {
            return Err(failure(format!(
                "source is both required and optional: {name}"
            )));
        }
        validate_source_name(name)?;
        logical_sources.insert(name.clone());
        let path = loaded.root.join(name);
        merge_source(
            &mut loaded,
            name,
            path,
            text,
            SourceKind::Required,
            &mut PackageClassification {
                physical: &physical,
                required: &required,
                optional: &mut optional,
            },
        )?;
    }

    for (name, text) in optional_sources {
        if !logical_sources.insert(name.clone()) {
            return Err(failure(format!("duplicate optional source: {name}")));
        }
        validate_source_name(name)?;
        let path = loaded.root.join(name);
        merge_source(
            &mut loaded,
            name,
            path,
            text,
            SourceKind::OptionalVirtual,
            &mut PackageClassification {
                physical: &physical,
                required: &required,
                optional: &mut optional,
            },
        )?;
    }

    for (name, source) in external_sources {
        if !logical_sources.insert(name.clone()) {
            return Err(failure(format!("duplicate optional source: {name}")));
        }
        validate_source_name(name)?;
        merge_source(
            &mut loaded,
            name,
            source.path.clone(),
            &source.text,
            SourceKind::External,
            &mut PackageClassification {
                physical: &physical,
                required: &required,
                optional: &mut optional,
            },
        )?;
    }

    loaded
        .packages
        .sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    retain_reachable_packages(&mut loaded, &optional, allow_incomplete)?;
    assign_package_ids(&mut loaded);
    Ok(loaded)
}

fn merge_source(
    loaded: &mut LoadedProject,
    name: &str,
    path: PathBuf,
    text: &str,
    kind: SourceKind,
    packages: &mut PackageClassification<'_>,
) -> Result<(), FrontendError> {
    if let Some(file) = loaded
        .sources
        .files()
        .iter()
        .find(|file| file.path() == path)
    {
        if file.text() != text {
            return Err(failure(format!(
                "virtual source conflicts with existing file: {name}"
            )));
        }
        return Ok(());
    }

    if kind != SourceKind::External && path.exists() {
        return Err(failure(format!(
            "virtual source conflicts with excluded path: {name}"
        )));
    }

    let relative = Path::new(name);
    let package = package_for_source_name(name);
    let file = loaded.sources.add_file(path.clone(), text.to_string());
    if kind != SourceKind::Required
        && !packages.physical.contains(&package)
        && !packages.required.contains(&package)
    {
        packages.optional.insert(package.clone());
    } else {
        packages.optional.remove(&package);
    }
    if let Some(existing) = loaded
        .packages
        .iter_mut()
        .find(|existing| existing.logical_path == package)
    {
        existing.files.push(file);
    } else {
        loaded.packages.push(PackageSources {
            id: PackageId(0),
            logical_path: package,
            directory: path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| loaded.root.join(relative.parent().unwrap_or(Path::new("")))),
            files: vec![file],
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Required,
    OptionalVirtual,
    External,
}

struct PackageClassification<'a> {
    physical: &'a BTreeSet<String>,
    required: &'a BTreeSet<String>,
    optional: &'a mut BTreeSet<String>,
}

fn retain_reachable_packages(
    loaded: &mut LoadedProject,
    optional: &BTreeSet<String>,
    allow_incomplete: bool,
) -> Result<(), FrontendError> {
    if optional.is_empty() {
        return Ok(());
    }
    let ast = match crate::ond::parse_project(loaded) {
        Ok(ast) => ast,
        Err(_) if allow_incomplete => return Ok(()),
        Err(error) => {
            return Err(FrontendError::new(
                error
                    .diagnostics()
                    .iter()
                    .map(|diagnostic| {
                        Diagnostic::error(Span::synthetic(), diagnostic.render(&loaded.sources))
                    })
                    .collect(),
            ));
        }
    };
    let graph = ast
        .packages
        .iter()
        .map(|package| {
            let imports = package
                .files
                .iter()
                .flat_map(|file| &file.imports)
                .map(|import| crate::decode_import_path(&import.path))
                .collect::<Result<Vec<_>, _>>()
                .map_err(failure)?;
            Ok((package.logical_path.clone(), imports))
        })
        .collect::<Result<BTreeMap<_, _>, FrontendError>>()?;
    let mut reachable = graph
        .keys()
        .filter(|package| !optional.contains(*package))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut pending = reachable.iter().cloned().collect::<Vec<_>>();
    while let Some(package) = pending.pop() {
        for import in graph.get(&package).into_iter().flatten() {
            if reachable.insert(import.clone()) {
                pending.push(import.clone());
            }
        }
    }
    loaded
        .packages
        .retain(|package| reachable.contains(&package.logical_path));
    Ok(())
}

fn assign_package_ids(loaded: &mut LoadedProject) {
    for (index, package) in loaded.packages.iter_mut().enumerate() {
        package.id = PackageId(index);
        package.files.sort_by(|left, right| {
            loaded
                .sources
                .file(*left)
                .path()
                .cmp(loaded.sources.file(*right).path())
        });
    }
}

fn validate_source_name(name: &str) -> Result<(), FrontendError> {
    let path = Path::new(name);
    if name.contains('\\')
        || name.contains(':')
        || path.is_absolute()
        || path.extension().is_none_or(|extension| extension != "ond")
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || name
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(failure(format!("invalid virtual source path: {name}")));
    }
    Ok(())
}

fn scan_library_root(
    root: &Path,
    directory: &Path,
    overrides: &BTreeMap<PathBuf, String>,
    files: &mut ExternalSources,
) -> Result<(), FrontendError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| failure(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| failure(error.to_string()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry
            .file_type()
            .map_err(|error| failure(error.to_string()))?;
        let path = entry.path();
        if kind.is_symlink() {
            return Err(failure(format!(
                "library symlink is not supported: {}",
                path.display()
            )));
        }
        if kind.is_dir() {
            if !matches!(
                entry.file_name().to_str(),
                Some(".git" | "target" | ".idea")
            ) {
                scan_library_root(root, &path, overrides, files)?;
            }
        } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "ond") {
            let name = path
                .strip_prefix(root)
                .expect("scanned path is below its library root")
                .to_str()
                .ok_or_else(|| failure("non-UTF8 library path"))?
                .replace('\\', "/");
            let text = read_source(&path, overrides)?;
            files.insert(name, ExternalSource { path, text });
        }
    }
    Ok(())
}

fn read_project_package(
    directory: &Path,
    package: &str,
    overrides: &BTreeMap<PathBuf, String>,
) -> Result<BTreeMap<String, String>, FrontendError> {
    let mut existing = BTreeMap::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| failure(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| failure(error.to_string()))?
    {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "ond") && path.is_file() {
            let name = format!(
                "{package}/{}",
                path.file_name().expect("file has a name").to_string_lossy()
            );
            existing.insert(name, read_source(&path, overrides)?);
        }
    }
    Ok(existing)
}

fn read_source(
    path: &Path,
    overrides: &BTreeMap<PathBuf, String>,
) -> Result<String, FrontendError> {
    if let Some(text) = overrides.get(path) {
        return Ok(text.clone());
    }
    if let Ok(canonical) = path.canonicalize()
        && let Some(text) = overrides.get(&canonical)
    {
        return Ok(text.clone());
    }
    fs::read_to_string(path)
        .map_err(|error| failure(format!("failed to read `{}`: {error}", path.display())))
}

fn source_texts(sources: &ExternalSources) -> BTreeMap<String, String> {
    sources
        .iter()
        .map(|(name, source)| (name.clone(), source.text.clone()))
        .collect()
}

fn package_for_source_name(name: &str) -> String {
    name.rsplit_once('/')
        .map_or(".", |(package, _)| package)
        .to_string()
}

fn failure(message: impl Into<String>) -> FrontendError {
    FrontendError::new(vec![Diagnostic::error(Span::synthetic(), message)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_packages_are_optional_and_keep_their_real_paths() {
        let base = temp_directory("external-inputs");
        let project = base.join("project");
        let library = base.join("library");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(library.join("shared")).unwrap();
        fs::create_dir_all(library.join("unused")).unwrap();
        fs::write(project.join("ond.toml"), "[project]\nname = \"test\"\n").unwrap();
        fs::write(
            project.join("main.ond"),
            "package main\nimport \"shared\"\nfunc main() { shared.Run() }\n",
        )
        .unwrap();
        let shared_path = library.join("shared/lib.ond");
        fs::write(&shared_path, "package shared\nfunc Run() {}\n").unwrap();
        fs::write(
            library.join("unused/lib.ond"),
            "package unused\nfunc Skip() {}\n",
        )
        .unwrap();

        let sources =
            load_library_sources(&project, std::slice::from_ref(&library), &BTreeMap::new())
                .unwrap();
        let loaded = load_project_with_inputs(
            &project,
            &BTreeMap::new(),
            &[],
            &BTreeMap::new(),
            &BTreeMap::new(),
            &sources,
        )
        .unwrap();

        assert!(
            loaded
                .packages
                .iter()
                .any(|package| package.logical_path == "shared")
        );
        assert!(
            !loaded
                .packages
                .iter()
                .any(|package| package.logical_path == "unused")
        );
        let shared_path = shared_path.canonicalize().unwrap();
        assert!(
            loaded
                .sources
                .files()
                .iter()
                .any(|file| file.path() == shared_path)
        );
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn incomplete_editor_input_can_retain_external_packages() {
        let base = temp_directory("incomplete-inputs");
        let project = base.join("project");
        let library = base.join("library");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(library.join("shared")).unwrap();
        fs::write(project.join("ond.toml"), "[project]\nname = \"test\"\n").unwrap();
        let main_path = project.join("main.ond");
        fs::write(
            &main_path,
            "package main\nimport \"shared\"\nfunc main() { shared.Run() }\n",
        )
        .unwrap();
        fs::write(
            library.join("shared/lib.ond"),
            "package shared\nfunc Run() {}\n",
        )
        .unwrap();
        let sources = load_library_sources(&project, &[library], &BTreeMap::new()).unwrap();
        let mut overrides = BTreeMap::new();
        overrides.insert(
            main_path.canonicalize().unwrap(),
            "package main\nimport \"shared\"\nfunc main() {\n    true\n}\n".to_string(),
        );

        assert!(
            load_project_with_inputs(
                &project,
                &overrides,
                &[],
                &BTreeMap::new(),
                &BTreeMap::new(),
                &sources,
            )
            .is_err()
        );
        let loaded = load_project_with_inputs_allow_incomplete(
            &project,
            &overrides,
            &[],
            &BTreeMap::new(),
            &BTreeMap::new(),
            &sources,
        )
        .unwrap();
        assert!(
            loaded
                .packages
                .iter()
                .any(|package| package.logical_path == "shared")
        );
        fs::remove_dir_all(base).unwrap();
    }

    fn temp_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ond-core-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
