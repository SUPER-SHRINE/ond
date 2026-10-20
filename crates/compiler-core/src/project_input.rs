use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::diagnostic::{Diagnostic, Diagnostics, FrontendError};
use crate::project::{LoadedProject, PackageId, PackageSources};
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSource {
    pub path: PathBuf,
    pub text: String,
}

pub type ExternalSources = BTreeMap<String, ExternalSource>;

/// Load project source overlays and resolved dependency sources through the shared frontend boundary.
pub fn load_project_with_inputs(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
    required_sources: &BTreeMap<String, String>,
    external_sources: &ExternalSources,
) -> Result<LoadedProject, FrontendError> {
    load_project_with_inputs_impl(
        root,
        overrides,
        excluded_dirs,
        required_sources,
        external_sources,
        false,
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
    external_sources: &ExternalSources,
) -> Result<LoadedProject, FrontendError> {
    load_project_with_inputs_impl(
        root,
        overrides,
        excluded_dirs,
        required_sources,
        external_sources,
        true,
        true,
    )
}

pub fn load_project_with_test_inputs(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
    required_sources: &BTreeMap<String, String>,
    external_sources: &ExternalSources,
) -> Result<LoadedProject, FrontendError> {
    load_project_with_inputs_impl(
        root,
        overrides,
        excluded_dirs,
        required_sources,
        external_sources,
        false,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn load_project_with_inputs_impl(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
    required_sources: &BTreeMap<String, String>,
    external_sources: &ExternalSources,
    allow_incomplete: bool,
    include_tests: bool,
) -> Result<LoadedProject, FrontendError> {
    let mut loaded = crate::project::load_project_with_test_sources(
        root,
        overrides,
        excluded_dirs,
        include_tests,
    )?;
    let project_name = crate::project::project_name(&loaded.manifest_path)?;
    loaded.packages.retain(|package| {
        !excluded_dirs
            .iter()
            .any(|directory| package.directory.starts_with(directory))
    });

    let root_is_app = crate::project::is_application_root(
        loaded
            .packages
            .iter()
            .filter(|package| package.directory == loaded.root)
            .flat_map(|package| package.files.iter())
            .map(|id| {
                let file = loaded.sources.file(*id);
                (file.path(), file.text())
            })
            .chain(
                required_sources
                    .iter()
                    .filter(|(name, _)| {
                        !name.contains('/')
                            && (include_tests || !crate::project::is_test_source(Path::new(name)))
                    })
                    .map(|(name, text)| (Path::new(name), text.as_str())),
            ),
    );
    let root_package = if root_is_app { "." } else { &project_name };

    let physical = loaded
        .packages
        .iter()
        .map(|package| package.logical_path.clone())
        .collect::<BTreeSet<_>>();
    let required = required_sources
        .keys()
        .map(|name| own_package_for_source_name(name, &project_name, root_package))
        .collect::<BTreeSet<_>>();
    let mut optional = BTreeSet::new();

    for (name, text) in required_sources {
        if !include_tests && crate::project::is_test_source(Path::new(name)) {
            continue;
        }
        validate_source_name(name)?;
        let path = loaded.root.join(name);
        merge_source(
            &mut loaded,
            name,
            own_package_for_source_name(name, &project_name, root_package),
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

    {
        let sources = external_sources;
        merge_optional_packages(
            &mut loaded,
            sources,
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
    let external_roots = external_sources
        .keys()
        .filter_map(|name| name.split_once('/').map(|(root, _)| root.to_string()))
        .collect::<BTreeSet<_>>();
    let application_roots = external_roots
        .into_iter()
        .filter(|root| {
            crate::project::is_application_root(
                external_sources
                    .iter()
                    .filter(|(name, _)| {
                        name.rsplit_once('/')
                            .is_some_and(|(parent, _)| parent == root)
                    })
                    .map(|(_, source)| (source.path.as_path(), source.text.as_str())),
            )
        })
        .collect();
    retain_reachable_packages(&mut loaded, &optional, &application_roots, allow_incomplete)?;
    assign_package_ids(&mut loaded);
    Ok(loaded)
}

fn merge_optional_packages(
    loaded: &mut LoadedProject,
    sources: &ExternalSources,
    classification: &mut PackageClassification<'_>,
) -> Result<(), FrontendError> {
    let mut external_packages = BTreeMap::<String, Vec<(&String, &ExternalSource)>>::new();
    for (name, source) in sources {
        if !crate::project::is_test_source(Path::new(name)) {
            validate_source_name(name)?;
            external_packages
                .entry(package_for_source_name(name))
                .or_default()
                .push((name, source));
        }
    }
    for (package, files) in external_packages {
        if let Some(existing) = loaded
            .packages
            .iter()
            .find(|item| item.logical_path == package)
        {
            let own_files = existing
                .files
                .iter()
                .filter(|id| !crate::project::is_test_source(loaded.sources.file(**id).path()))
                .map(|id| {
                    let source = loaded.sources.file(*id);
                    (
                        source
                            .display_path()
                            .map(Path::new)
                            .unwrap_or_else(|| source.path())
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        source.text().to_string(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let external_files = files
                .iter()
                .map(|(name, source)| {
                    (
                        Path::new(name)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        source.text.clone(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if own_files != external_files {
                return Err(failure(format!(
                    "library conflicts with project package: {package}"
                )));
            }
            continue;
        }
        for (name, source) in files {
            merge_source(
                loaded,
                name,
                package.clone(),
                source.path.clone(),
                &source.text,
                SourceKind::External,
                classification,
            )?;
        }
    }

    Ok(())
}

fn merge_source(
    loaded: &mut LoadedProject,
    name: &str,
    package: String,
    path: PathBuf,
    text: &str,
    kind: SourceKind,
    packages: &mut PackageClassification<'_>,
) -> Result<(), FrontendError> {
    if kind == SourceKind::Required
        && let Some(file) = loaded
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
    let file =
        loaded
            .sources
            .add_file_with_display_path(path.clone(), text.to_string(), name.to_string());
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
    application_roots: &BTreeSet<String>,
    allow_incomplete: bool,
) -> Result<(), FrontendError> {
    if optional.is_empty() {
        return Ok(());
    }
    let package_paths = loaded
        .packages
        .iter()
        .map(|package| package.logical_path.clone())
        .collect::<BTreeSet<_>>();
    let mut reachable = package_paths
        .difference(optional)
        .cloned()
        .collect::<BTreeSet<_>>();
    let root_imports = match imports_for_packages(loaded, &reachable) {
        Ok(imports) => imports,
        Err(_) if allow_incomplete => return Ok(()),
        Err(error) => return Err(render_parse_error(loaded, error)),
    };
    let mut pending = root_imports.into_values().flatten().collect::<Vec<_>>();

    while let Some(package) = pending.pop() {
        if application_roots.contains(&package) {
            return Err(failure(format!(
                "cannot import application root: {package}"
            )));
        }
        if !package_paths.contains(&package) || !reachable.insert(package.clone()) {
            continue;
        }
        if !optional.contains(&package) {
            continue;
        }
        let imports = match imports_for_packages(loaded, &BTreeSet::from([package])) {
            Ok(imports) => imports,
            Err(_) if allow_incomplete => return Ok(()),
            Err(error) => return Err(render_parse_error(loaded, error)),
        };
        pending.extend(imports.into_values().flatten());
    }

    loaded
        .packages
        .retain(|package| reachable.contains(&package.logical_path));
    Ok(())
}

fn render_parse_error(loaded: &LoadedProject, error: FrontendError) -> FrontendError {
    FrontendError::new(
        error
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                Diagnostic::error(Span::synthetic(), diagnostic.render(&loaded.sources))
            })
            .collect(),
    )
}

fn imports_for_packages(
    loaded: &LoadedProject,
    selected: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<String>>, FrontendError> {
    let mut diagnostics = Diagnostics::new();
    let mut imports_by_package = BTreeMap::new();
    for package in loaded
        .packages
        .iter()
        .filter(|package| selected.contains(&package.logical_path))
    {
        let mut imports = Vec::new();
        for &file_id in &package.files {
            match crate::ond::parse_source_file(file_id, loaded.sources.file(file_id).text()) {
                Ok(file) => {
                    for import in file.imports {
                        match crate::decode_import_path(&import.path) {
                            Ok(path) => imports.push(path),
                            Err(message) => return Err(failure(message)),
                        }
                    }
                }
                Err(error) => diagnostics.extend(error),
            }
        }
        imports_by_package.insert(package.logical_path.clone(), imports);
    }
    if diagnostics.is_empty() {
        Ok(imports_by_package)
    } else {
        Err(FrontendError::new(diagnostics.into_vec()))
    }
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

fn package_for_source_name(name: &str) -> String {
    name.rsplit_once('/')
        .map_or(".", |(package, _)| package)
        .to_string()
}

fn own_package_for_source_name(name: &str, project_name: &str, root_package: &str) -> String {
    let relative = package_for_source_name(name);
    if relative == "." {
        root_package.to_string()
    } else {
        format!("{project_name}/{relative}")
    }
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

        let sources = file_inputs(&library, &["shared/lib.ond", "unused/lib.ond"]);
        let loaded =
            load_project_with_inputs(&project, &BTreeMap::new(), &[], &BTreeMap::new(), &sources)
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
        let sources = file_inputs(&library, &["shared/lib.ond"]);
        let mut overrides = BTreeMap::new();
        overrides.insert(
            main_path.canonicalize().unwrap(),
            "package main\nimport \"shared\"\nfunc main() {\n    true\n}\n".to_string(),
        );

        assert!(
            load_project_with_inputs(&project, &overrides, &[], &BTreeMap::new(), &sources,)
                .is_err()
        );
        let loaded = load_project_with_inputs_allow_incomplete(
            &project,
            &overrides,
            &[],
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

    fn file_inputs(root: &Path, names: &[&str]) -> ExternalSources {
        names
            .iter()
            .map(|name| {
                let path = root.join(name).canonicalize().unwrap();
                (
                    name.to_string(),
                    ExternalSource {
                        text: fs::read_to_string(&path).unwrap(),
                        path,
                    },
                )
            })
            .collect()
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
