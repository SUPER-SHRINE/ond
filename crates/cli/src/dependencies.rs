use crate::{DriverResult, cache};
use ond_compiler_core::{ExternalSource, ExternalSources};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default, Deserialize)]
struct Manifest {
    project: Option<Project>,
    #[serde(default)]
    dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    resolve: Resolve,
}

#[derive(Debug, Deserialize)]
struct Project {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Dependency {
    Short(String),
    Detailed {
        path: Option<PathBuf>,
        git: Option<String>,
        rev: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct Resolve {
    #[serde(default = "default_ignores")]
    ignore: Vec<PathBuf>,
}

impl Default for Resolve {
    fn default() -> Self {
        Self {
            ignore: default_ignores(),
        }
    }
}

fn default_ignores() -> Vec<PathBuf> {
    vec![".git".into(), ".ond".into(), "target".into()]
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct LockFile {
    version: u32,
    #[serde(default)]
    package: BTreeMap<String, LockedGit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct LockedGit {
    git: String,
    commit: String,
}

#[derive(Debug, Default)]
pub struct ResolvedDependencies {
    pub sources: ExternalSources,
    pub ignored_directories: Vec<PathBuf>,
}

pub fn resolve_dependencies(project_root: &Path) -> DriverResult<ResolvedDependencies> {
    let manifest_path = project_root.join("ond.toml");
    let manifest = read_manifest(&manifest_path)?;
    let existing_lock = read_lock(&project_root.join("ond.lock"))?;
    let mut resolver = Resolver {
        project_root,
        existing_lock,
        new_lock: LockFile {
            version: 1,
            ..LockFile::default()
        },
        roots_by_namespace: BTreeMap::new(),
        visited_roots: BTreeSet::new(),
        sources: BTreeMap::new(),
    };
    resolver.resolve_manifest(project_root, &manifest)?;
    resolver.write_lock()?;
    Ok(ResolvedDependencies {
        sources: resolver.sources,
        ignored_directories: ignored_directories(project_root, &manifest.resolve)?,
    })
}

pub fn resolve_project_sources(project_root: &Path) -> DriverResult<ResolvedDependencies> {
    let root = project_root
        .canonicalize()
        .map_err(|error| vec![format!("project {}: {error}", project_root.display())])?;
    let manifest = read_manifest(&root.join("ond.toml"))?;
    let namespace = project_namespace(&root, &manifest)?;
    let mut sources = ExternalSources::new();
    scan_dependency(&namespace, &root, &manifest.resolve, &mut sources)?;
    Ok(ResolvedDependencies {
        sources,
        ignored_directories: ignored_directories(&root, &manifest.resolve)?,
    })
}

struct Resolver<'a> {
    project_root: &'a Path,
    existing_lock: LockFile,
    new_lock: LockFile,
    roots_by_namespace: BTreeMap<String, PathBuf>,
    visited_roots: BTreeSet<PathBuf>,
    sources: ExternalSources,
}

impl Resolver<'_> {
    fn resolve_manifest(&mut self, owner: &Path, manifest: &Manifest) -> DriverResult<()> {
        for (key, dependency) in &manifest.dependencies {
            validate_dependency_key(key)?;
            let root = self.materialize(owner, key, dependency)?;
            let canonical = root.canonicalize().map_err(|error| {
                vec![format!("dependency `{key}` at {}: {error}", root.display())]
            })?;
            if !canonical.join("ond.toml").is_file() {
                return Err(vec![format!(
                    "dependency `{key}` does not contain `ond.toml`: {}",
                    canonical.display()
                )]);
            }
            let dependency_manifest = read_manifest(&canonical.join("ond.toml"))?;
            let namespace = dependency_namespace(key, &canonical, &dependency_manifest)?;
            if let Some(previous) = self
                .roots_by_namespace
                .insert(namespace.clone(), canonical.clone())
                && previous != canonical
            {
                return Err(vec![format!(
                    "dependency namespace `{namespace}` resolves to both {} and {}",
                    previous.display(),
                    canonical.display()
                )]);
            }
            if !self.visited_roots.insert(canonical.clone()) {
                continue;
            }
            scan_dependency(
                &namespace,
                &canonical,
                &dependency_manifest.resolve,
                &mut self.sources,
            )?;
            self.resolve_manifest(&canonical, &dependency_manifest)?;
        }
        Ok(())
    }

    fn materialize(
        &mut self,
        owner: &Path,
        name: &str,
        dependency: &Dependency,
    ) -> DriverResult<PathBuf> {
        match dependency {
            Dependency::Short(value) if looks_like_git(value) => {
                self.git_dependency(name, value, None)
            }
            Dependency::Short(value) => Ok(owner.join(value)),
            Dependency::Detailed {
                path: Some(path),
                git: None,
                rev: None,
            } => Ok(if path.is_absolute() {
                path.clone()
            } else {
                owner.join(path)
            }),
            Dependency::Detailed {
                path: None,
                git: Some(git),
                rev,
            } => self.git_dependency(name, git, rev.as_deref()),
            _ => Err(vec![format!(
                "dependency `{name}` must specify exactly one of `path` or `git`; `rev` is valid only with `git`"
            )]),
        }
    }

    fn git_dependency(
        &mut self,
        name: &str,
        url: &str,
        revision: Option<&str>,
    ) -> DriverResult<PathBuf> {
        let digest = cache::hash(url.as_bytes());
        let checkout = self
            .project_root
            .join(".ond/dependencies")
            .join(format!("{name}-{}", &digest[..12]));
        let locked = self
            .existing_lock
            .package
            .get(name)
            .filter(|entry| entry.git == url)
            .map(|entry| entry.commit.clone());

        let cached = checkout.join(".git").is_dir();
        if !cached {
            if checkout.exists() {
                return Err(vec![format!(
                    "dependency cache exists but is not a Git checkout: {}",
                    checkout.display()
                )]);
            }
            let checkout_argument = git_process_path(&checkout);
            run_git(
                self.project_root,
                ["clone", "--", url, checkout_argument.as_str()],
                name,
            )?;
        }

        let requested = if let Some(commit) = locked.as_deref() {
            Some(commit.to_string())
        } else if cached {
            run_git(&checkout, ["fetch", "--tags", "--prune", "origin"], name)?;
            match revision {
                Some(revision) => Some(revision.to_string()),
                None => Some(git_output(
                    &checkout,
                    ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
                    name,
                )?),
            }
        } else {
            revision.map(str::to_string)
        };
        if let Some(requested) = requested.as_deref() {
            run_git(&checkout, ["checkout", "--detach", requested], name)?;
        }
        let commit = git_output(&checkout, ["rev-parse", "HEAD"], name)?;
        self.new_lock.package.insert(
            name.to_string(),
            LockedGit {
                git: url.to_string(),
                commit,
            },
        );
        Ok(checkout)
    }

    fn write_lock(&self) -> DriverResult<()> {
        if self.new_lock.package.is_empty() {
            return Ok(());
        }
        let text =
            toml::to_string_pretty(&self.new_lock).map_err(|error| vec![error.to_string()])?;
        cache::atomic_write(&self.project_root.join("ond.lock"), text.as_bytes())
    }
}

fn read_manifest(path: &Path) -> DriverResult<Manifest> {
    let text = fs::read_to_string(path)
        .map_err(|error| vec![format!("failed to read `{}`: {error}", path.display())])?;
    toml::from_str(&text)
        .map_err(|error| vec![format!("invalid manifest `{}`: {error}", path.display())])
}

fn read_lock(path: &Path) -> DriverResult<LockFile> {
    if !path.is_file() {
        return Ok(LockFile::default());
    }
    let text = fs::read_to_string(path)
        .map_err(|error| vec![format!("failed to read `{}`: {error}", path.display())])?;
    let lock: LockFile = toml::from_str(&text)
        .map_err(|error| vec![format!("invalid lock file `{}`: {error}", path.display())])?;
    if lock.version != 1 {
        return Err(vec![format!(
            "unsupported ond.lock version: {}",
            lock.version
        )]);
    }
    Ok(lock)
}

fn scan_dependency(
    name: &str,
    root: &Path,
    resolve: &Resolve,
    sources: &mut ExternalSources,
) -> DriverResult<()> {
    let ignored = ignored_directories(root, resolve)?;
    scan_directory(name, root, root, &ignored, sources)
}

fn scan_directory(
    name: &str,
    root: &Path,
    directory: &Path,
    ignored: &[PathBuf],
    sources: &mut ExternalSources,
) -> DriverResult<()> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| vec![format!("failed to read `{}`: {error}", directory.display())])?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| vec![error.to_string()])?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| vec![format!("failed to inspect `{}`: {error}", path.display())])?;
        if file_type.is_symlink() {
            return Err(vec![format!(
                "dependency `{name}` contains unsupported symlink: {}",
                path.display()
            )]);
        }
        if file_type.is_dir() {
            if ignored.iter().any(|ignored| path.starts_with(ignored)) {
                continue;
            }
            if path != root && path.join("ond.toml").is_file() {
                continue;
            }
            scan_directory(name, root, &path, ignored, sources)?;
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("ond")
        {
            let relative = path
                .strip_prefix(root)
                .expect("dependency source is below root");
            let logical = format!("{name}/{}", relative.to_string_lossy().replace('\\', "/"));
            let text = fs::read_to_string(&path)
                .map_err(|error| vec![format!("failed to read `{}`: {error}", path.display())])?;
            let source = ExternalSource {
                path: path.clone(),
                text,
            };
            if let Some(previous) = sources.insert(logical.clone(), source.clone())
                && previous != source
            {
                return Err(vec![format!("conflicting dependency source: {logical}")]);
            }
        }
    }
    Ok(())
}

fn ignored_directories(root: &Path, resolve: &Resolve) -> DriverResult<Vec<PathBuf>> {
    let mut result = Vec::new();
    for relative in &resolve.ignore {
        if relative.as_os_str().is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(vec![format!(
                "resolve.ignore entry must be a project-relative directory: {}",
                relative.display()
            )]);
        }
        result.push(root.join(relative));
    }
    let internal = root.join(".ond");
    if !result.contains(&internal) {
        result.push(internal);
    }
    Ok(result)
}

fn dependency_namespace(key: &str, root: &Path, manifest: &Manifest) -> DriverResult<String> {
    project_namespace(root, manifest).map_err(|_| {
        vec![format!(
            "dependency `{key}` must declare `[project] name` as a valid Ond identifier in {}",
            root.join("ond.toml").display()
        )]
    })
}

fn project_namespace(root: &Path, manifest: &Manifest) -> DriverResult<String> {
    let Some(project) = manifest.project.as_ref() else {
        return Err(vec![format!(
            "project must declare `[project] name` in {}",
            root.join("ond.toml").display()
        )]);
    };
    validate_namespace(&project.name).map_err(|_| {
        vec![format!(
            "invalid project name `{}` in {}; `[project].name` must be an Ond identifier",
            project.name,
            root.join("ond.toml").display()
        )]
    })?;
    Ok(project.name.clone())
}

fn validate_dependency_key(key: &str) -> DriverResult<()> {
    if key.is_empty()
        || key == "."
        || key == ".."
        || !key
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        return Err(vec![format!("invalid dependency key `{key}`")]);
    }
    Ok(())
}

fn validate_namespace(name: &str) -> DriverResult<()> {
    let mut chars = name.chars();
    let valid_start = chars
        .next()
        .is_some_and(|character| character == '_' || character.is_alphabetic());
    if name == "_"
        || !valid_start
        || !chars.all(|character| character == '_' || character.is_alphanumeric())
    {
        return Err(vec![format!("invalid dependency namespace `{name}`")]);
    }
    Ok(())
}

fn looks_like_git(value: &str) -> bool {
    value.starts_with("https://")
        || value.starts_with("http://")
        || value.starts_with("ssh://")
        || value.starts_with("git@")
        || value.ends_with(".git")
}

#[cfg(windows)]
fn git_process_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    path.into_owned()
}

#[cfg(not(windows))]
fn git_process_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn run_git<'a>(
    directory: &Path,
    arguments: impl IntoIterator<Item = &'a str>,
    dependency: &str,
) -> DriverResult<()> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .map_err(|error| {
            vec![format!(
                "failed to run git for dependency `{dependency}`: {error}"
            )]
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(vec![format!(
            "git failed for dependency `{dependency}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )])
    }
}

fn git_output<'a>(
    directory: &Path,
    arguments: impl IntoIterator<Item = &'a str>,
    dependency: &str,
) -> DriverResult<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .map_err(|error| {
            vec![format!(
                "failed to run git for dependency `{dependency}`: {error}"
            )]
        })?;
    if !output.status.success() {
        return Err(vec![format!(
            "git failed for dependency `{dependency}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )]);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_and_absolute_path_dependencies_with_subpackages() {
        let temp = tempfile_root("paths");
        let app = temp.join("app");
        let relative = temp.join("relative");
        let absolute = temp.join("absolute");
        fs::create_dir_all(relative.join("json")).unwrap();
        fs::create_dir_all(&absolute).unwrap();
        fs::create_dir_all(&app).unwrap();
        fs::write(
            relative.join("ond.toml"),
            "[project]\nname = \"portable\"\n",
        )
        .unwrap();
        fs::write(relative.join("root.ond"), "package relative\n").unwrap();
        fs::write(relative.join("json/json.ond"), "package json\n").unwrap();
        fs::write(
            absolute.join("ond.toml"),
            "[project]\nname = \"platform\"\n",
        )
        .unwrap();
        fs::write(absolute.join("root.ond"), "package absolute\n").unwrap();
        fs::write(
            app.join("ond.toml"),
            format!(
                "[dependencies]\nrelative = {{ path = \"../relative\" }}\nabsolute = {{ path = {:?} }}\n",
                absolute.to_string_lossy()
            )
            .replace('\\', "/"),
        )
        .unwrap();

        let resolved = resolve_dependencies(&app).unwrap();
        assert!(resolved.sources.contains_key("portable/root.ond"));
        assert!(resolved.sources.contains_key("portable/json/json.ond"));
        assert!(resolved.sources.contains_key("platform/root.ond"));
    }

    #[test]
    fn respects_ignore_and_nested_project_boundaries() {
        let temp = tempfile_root("ignore");
        let app = temp.join("app");
        let library = temp.join("library");
        fs::create_dir_all(app.as_path()).unwrap();
        fs::create_dir_all(library.join("generated")).unwrap();
        fs::create_dir_all(library.join("nested")).unwrap();
        fs::write(
            app.join("ond.toml"),
            "[dependencies]\nlib = { path = \"../library\" }\n",
        )
        .unwrap();
        fs::write(
            library.join("ond.toml"),
            "[project]\nname = \"shared\"\n\n[resolve]\nignore = [\"generated\"]\n",
        )
        .unwrap();
        fs::write(library.join("lib.ond"), "package lib\n").unwrap();
        fs::write(library.join("generated/skip.ond"), "package generated\n").unwrap();
        fs::write(library.join("nested/ond.toml"), "").unwrap();
        fs::write(library.join("nested/skip.ond"), "package nested\n").unwrap();

        let resolved = resolve_dependencies(&app).unwrap();
        assert_eq!(
            resolved.sources.keys().collect::<Vec<_>>(),
            vec!["shared/lib.ond"]
        );
    }

    #[test]
    fn git_dependencies_are_cached_and_locked_to_a_commit() {
        let temp = tempfile_root("git");
        let app = temp.join("app");
        let repository = temp.join("repository");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&repository).unwrap();
        fs::write(
            repository.join("ond.toml"),
            "[project]\nname = \"remote\"\n",
        )
        .unwrap();
        fs::write(repository.join("gitlib.ond"), "package gitlib\n").unwrap();
        git(&repository, &["init"]);
        git(
            &repository,
            &["config", "user.email", "ond@example.invalid"],
        );
        git(&repository, &["config", "user.name", "Ond Test"]);
        git(&repository, &["add", "."]);
        git(&repository, &["commit", "-m", "initial"]);
        let repository_url = repository.to_string_lossy().replace('\\', "/");
        fs::write(
            app.join("ond.toml"),
            format!("[dependencies]\ngitlib = {{ git = {repository_url:?} }}\n"),
        )
        .unwrap();

        let first = resolve_dependencies(&app).unwrap();
        assert!(first.sources.contains_key("remote/gitlib.ond"));
        let lock = fs::read_to_string(app.join("ond.lock")).unwrap();
        assert!(lock.contains("commit ="));
        let second = resolve_dependencies(&app).unwrap();
        assert_eq!(first.sources, second.sources);
    }

    #[test]
    fn dependency_namespace_comes_from_the_dependency_manifest() {
        let temp = tempfile_root("namespace");
        let app = temp.join("app");
        let library = temp.join("library");
        fs::create_dir_all(library.join("blob")).unwrap();
        fs::create_dir_all(library.join("zlib")).unwrap();
        fs::create_dir_all(&app).unwrap();
        fs::write(
            app.join("ond.toml"),
            "[dependencies]\nconsumer_alias = { path = \"../library\" }\n",
        )
        .unwrap();
        fs::write(library.join("ond.toml"), "[project]\nname = \"portable\"\n").unwrap();
        fs::write(library.join("blob/blob.ond"), "package blob\n").unwrap();
        fs::write(
            library.join("zlib/zlib.ond"),
            "package zlib\nimport \"portable/blob\"\n",
        )
        .unwrap();

        let resolved = resolve_dependencies(&app).unwrap();
        assert!(resolved.sources.contains_key("portable/blob/blob.ond"));
        assert!(resolved.sources.contains_key("portable/zlib/zlib.ond"));
        assert!(
            !resolved
                .sources
                .keys()
                .any(|path| path.starts_with("consumer_alias/"))
        );
    }

    #[test]
    fn dependency_requires_a_project_name() {
        let temp = tempfile_root("missing-project-name");
        let app = temp.join("app");
        let library = temp.join("library");
        fs::create_dir_all(&library).unwrap();
        fs::create_dir_all(&app).unwrap();
        fs::write(
            app.join("ond.toml"),
            "[dependencies]\nlib = { path = \"../library\" }\n",
        )
        .unwrap();
        fs::write(library.join("ond.toml"), "").unwrap();

        let error = resolve_dependencies(&app).unwrap_err().join("\n");
        assert!(error.contains("must declare `[project] name`"), "{error}");
    }

    fn git(directory: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn tempfile_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ond-dependencies-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
