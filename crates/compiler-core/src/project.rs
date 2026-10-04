use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, Diagnostics, FrontendError};
use crate::source::{FileId, SourceDb, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(pub usize);

#[derive(Debug, Clone)]
pub struct PackageSources {
    pub id: PackageId,
    pub logical_path: String,
    pub directory: PathBuf,
    pub files: Vec<FileId>,
}

#[derive(Debug, Clone)]
pub struct LoadedProject {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub ignored_paths: Vec<String>,
    pub sources: SourceDb,
    pub packages: Vec<PackageSources>,
}

pub fn load_project(root: impl AsRef<Path>) -> Result<LoadedProject, FrontendError> {
    load_project_with_overrides(root, &BTreeMap::new())
}

pub fn load_project_with_overrides(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
) -> Result<LoadedProject, FrontendError> {
    load_project_with_overrides_and_exclusions(root, overrides, &[])
}

pub fn load_project_with_overrides_and_exclusions(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<PathBuf, String>,
    excluded_dirs: &[PathBuf],
) -> Result<LoadedProject, FrontendError> {
    let root = root.as_ref();
    let root = if root.is_file() && root.file_name().is_some_and(|name| name == "ond.toml") {
        root.parent().unwrap_or(root)
    } else {
        root
    };
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let manifest_path = root.join("ond.toml");

    let mut diagnostics = Diagnostics::new();
    if !manifest_path.is_file() {
        diagnostics.push(Diagnostic::error(
            Span::synthetic(),
            format!(
                "project root `{}` does not contain `ond.toml`",
                root.display()
            ),
        ));
        return Err(FrontendError::new(diagnostics.into_vec()));
    }

    let mut sources = SourceDb::default();
    let mut files_by_package = BTreeMap::<String, Vec<FileId>>::new();
    let mut dir_by_package = BTreeMap::<String, PathBuf>::new();

    let options = SourceLoadOptions {
        overrides,
        excluded_dirs,
    };
    collect_ond_files(
        &root,
        &root,
        options,
        &mut sources,
        &mut files_by_package,
        &mut dir_by_package,
        &mut diagnostics,
    );

    if !diagnostics.is_empty() {
        return Err(FrontendError::new(diagnostics.into_vec()));
    }

    let mut packages = Vec::new();
    for (index, (logical_path, files)) in files_by_package.into_iter().enumerate() {
        let directory = dir_by_package
            .remove(&logical_path)
            .unwrap_or_else(|| root.clone());
        packages.push(PackageSources {
            id: PackageId(index),
            logical_path,
            directory,
            files,
        });
    }

    let ignored_paths = excluded_dirs
        .iter()
        .filter_map(|path| path.strip_prefix(&root).ok())
        .map(path_to_logical)
        .filter(|path| path != ".")
        .collect();

    Ok(LoadedProject {
        root,
        manifest_path,
        ignored_paths,
        sources,
        packages,
    })
}

#[derive(Clone, Copy)]
struct SourceLoadOptions<'a> {
    overrides: &'a BTreeMap<PathBuf, String>,
    excluded_dirs: &'a [PathBuf],
}

fn collect_ond_files(
    root: &Path,
    directory: &Path,
    options: SourceLoadOptions<'_>,
    sources: &mut SourceDb,
    files_by_package: &mut BTreeMap<String, Vec<FileId>>,
    dir_by_package: &mut BTreeMap<String, PathBuf>,
    diagnostics: &mut Diagnostics,
) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(err) => {
            diagnostics.push(Diagnostic::error(
                Span::synthetic(),
                format!("failed to read directory `{}`: {err}", directory.display()),
            ));
            return;
        }
    };

    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                diagnostics.push(Diagnostic::error(
                    Span::synthetic(),
                    format!("failed to inspect `{}`: {err}", path.display()),
                ));
                continue;
            }
        };

        if file_type.is_dir() {
            if should_skip_dir(&path)
                || options
                    .excluded_dirs
                    .iter()
                    .any(|excluded| path.starts_with(excluded))
            {
                continue;
            }
            if path != root && path.join("ond.toml").is_file() {
                continue;
            }
            collect_ond_files(
                root,
                &path,
                options,
                sources,
                files_by_package,
                dir_by_package,
                diagnostics,
            );
            continue;
        }

        if !file_type.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("ond") {
            continue;
        }

        let text = match read_source_file(&path, options.overrides) {
            Ok(text) => text,
            Err(err) => {
                diagnostics.push(Diagnostic::error(
                    Span::synthetic(),
                    format!("failed to read `{}`: {err}", path.display()),
                ));
                continue;
            }
        };

        let relative_dir = path
            .parent()
            .and_then(|parent| parent.strip_prefix(root).ok())
            .unwrap_or_else(|| Path::new(""));
        let logical_path = path_to_logical(relative_dir);
        let file_id = sources.add_file(path.clone(), text);
        files_by_package
            .entry(logical_path.clone())
            .or_default()
            .push(file_id);
        dir_by_package.entry(logical_path).or_insert_with(|| {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.to_path_buf())
        });
    }
}

fn read_source_file(
    path: &Path,
    overrides: &BTreeMap<PathBuf, String>,
) -> Result<String, std::io::Error> {
    if let Some(text) = overrides.get(path) {
        return Ok(text.clone());
    }

    if let Ok(canonical) = path.canonicalize() {
        if let Some(text) = overrides.get(&canonical) {
            return Ok(text.clone());
        }
    }

    fs::read_to_string(path)
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | ".ond" | "target" | ".idea")
    )
}

fn path_to_logical(path: &Path) -> String {
    let display = path.to_string_lossy().replace('\\', "/");
    if display.is_empty() {
        ".".to_string()
    } else {
        display
    }
}
