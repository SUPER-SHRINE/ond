use crate::{DriverResult, cache};
use ond_compiler_core::{ExternalSource, ExternalSources};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
mod roles;

#[derive(Debug, Default, Deserialize)]
struct Manifest {
    project: Option<Project>,
    #[serde(default)]
    dependencies: BTreeMap<String, Dependency>,
    #[serde(default)]
    resolve: Resolve,
    #[serde(skip)]
    roles: Option<ond_compiler_core::roles::RoleConfiguration>,
    #[serde(skip)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct Project {
    name: String,
    #[serde(default, rename = "final")]
    final_project: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Dependency {
    Short(String),
    Detailed {
        path: Option<PathBuf>,
        git: Option<String>,
        rev: Option<String>,
        role: Option<toml::Value>,
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
#[serde(deny_unknown_fields)]
struct LockFile {
    version: u32,
    package: Vec<LockedGit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LockedGit {
    owner: String,
    name: String,
    git: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    rev: Option<String>,
    commit: String,
}

#[derive(Debug, Default)]
pub struct ResolvedDependencies {
    pub sources: ExternalSources,
    pub ignored_directories: Vec<PathBuf>,
    pub role_inputs: ond_compiler_core::roles::DependencyInputs,
}

pub fn resolve_dependencies(project_root: &Path) -> DriverResult<ResolvedDependencies> {
    resolve_dependencies_with_role(project_root, None)
}

pub fn resolve_dependencies_with_role(
    project_root: &Path,
    selected: Option<&str>,
) -> DriverResult<ResolvedDependencies> {
    let canonical_root = project_root
        .canonicalize()
        .map_err(|error| vec![format!("project {}: {error}", project_root.display())])?;
    let project_root = canonical_root.as_path();
    let manifest_path = project_root.join("ond.toml");
    let manifest = read_manifest(&manifest_path)?;
    let own_namespace = project_namespace(project_root, &manifest)?;
    let existing_lock = read_lock(&project_root.join("ond.lock"))?;
    let mut role_graph = roles::RoleGraph::default();
    role_graph.root(project_root, &manifest, &own_namespace, selected)?;
    let mut resolver = Resolver {
        role_graph,
        project_root,
        existing_lock,
        new_lock: LockFile {
            version: 2,
            ..LockFile::default()
        },
        roots_by_namespace: BTreeMap::from([(own_namespace, project_root.to_path_buf())]),
        visited_roots: BTreeSet::from([canonical_root.clone()]),
        sources: BTreeMap::new(),
    };
    resolver.resolve_manifest(project_root, &manifest)?;
    resolver.write_lock()?;
    Ok(ResolvedDependencies {
        sources: resolver.sources,
        ignored_directories: ignored_directories(project_root, &manifest.resolve)?,
        role_inputs: resolver.role_graph.inputs,
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
        role_inputs: Default::default(),
    })
}

struct Resolver<'a> {
    role_graph: roles::RoleGraph,
    project_root: &'a Path,
    existing_lock: LockFile,
    new_lock: LockFile,
    roots_by_namespace: BTreeMap<String, PathBuf>,
    visited_roots: BTreeSet<PathBuf>,
    sources: ExternalSources,
}

impl Resolver<'_> {
    fn resolve_manifest(&mut self, owner: &Path, manifest: &Manifest) -> DriverResult<()> {
        let owner_name = project_namespace(owner, manifest)?;
        for (key, dependency) in &manifest.dependencies {
            validate_dependency_key(key)?;
            let root = self.materialize(owner, &owner_name, key, dependency)?;
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
            self.role_graph
                .edge(owner, manifest, key, &canonical, &dependency_manifest)?;
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
        owner_name: &str,
        name: &str,
        dependency: &Dependency,
    ) -> DriverResult<PathBuf> {
        match dependency {
            Dependency::Short(value) if looks_like_git(value) => {
                self.git_dependency(owner_name, name, value, None)
            }
            Dependency::Short(value) => Ok(owner.join(value)),
            Dependency::Detailed {
                path: Some(path),
                git: None,
                rev: None,
                ..
            } => Ok(if path.is_absolute() {
                path.clone()
            } else {
                owner.join(path)
            }),
            Dependency::Detailed {
                path: None,
                git: Some(git),
                rev,
                ..
            } => self.git_dependency(owner_name, name, git, rev.as_deref()),
            _ => Err(vec![format!(
                "dependency `{name}` must specify exactly one of `path` or `git`; `rev` is valid only with `git`"
            )]),
        }
    }

    fn git_dependency(
        &mut self,
        owner: &str,
        name: &str,
        url: &str,
        revision: Option<&str>,
    ) -> DriverResult<PathBuf> {
        if revision.is_some_and(str::is_empty) {
            return Err(vec!["dependency revision must not be empty".into()]);
        }
        let locked = self
            .existing_lock
            .package
            .iter()
            .find(|entry| entry.owner == owner && entry.name == name)
            .cloned();
        if self.existing_lock.version != 0
            && locked
                .as_ref()
                .is_none_or(|entry| entry.git != url || entry.rev.as_deref() != revision)
        {
            return Err(vec![format!(
                "lock does not match Git dependency {owner}/{name}"
            )]);
        }

        // Edge identity stays owner/name in the lock; identical content uses
        // one checkout even when reached through different dependency edges.
        let resolved = locked.or_else(|| {
            self.new_lock
                .package
                .iter()
                .find(|entry| entry.git == url && entry.rev.as_deref() == revision)
                .cloned()
        });
        let checkout_path = |commit: Option<&str>| -> DriverResult<PathBuf> {
            let digest = cache::hash(&cache::encode(&(url, revision, commit))?);
            Ok(self
                .project_root
                .join(".ond/dependencies")
                .join(format!("git-{}", &digest[..12])))
        };
        let mut checkout = checkout_path(resolved.as_ref().map(|entry| entry.commit.as_str()))?;

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

        let requested = if let Some(entry) = &resolved {
            let commit = git_output(
                &checkout,
                [
                    "rev-parse",
                    "--verify",
                    &format!("{}^{{commit}}", entry.commit),
                ],
                name,
            )?;
            if commit != entry.commit {
                return Err(vec![format!(
                    "locked object is not a commit: {}",
                    entry.commit
                )]);
            }
            Some(commit)
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
        let commit = git_output(&checkout, ["rev-parse", "HEAD^{commit}"], name)?;
        let shared = checkout_path(Some(&commit))?;
        if checkout != shared {
            if shared.exists() {
                if git_output(&shared, ["rev-parse", "HEAD^{commit}"], name)? != commit {
                    return Err(vec!["dependency checkout identity mismatch".into()]);
                }
            } else {
                fs::rename(&checkout, &shared).map_err(|error| {
                    vec![format!(
                        "dependency checkout {}: {error}",
                        checkout.display()
                    )]
                })?;
            }
            checkout = shared;
        }
        self.new_lock.package.push(LockedGit {
            owner: owner.to_string(),
            name: name.to_string(),
            git: url.to_string(),
            rev: revision.map(str::to_string),
            commit,
        });
        Ok(checkout)
    }

    fn write_lock(&self) -> DriverResult<()> {
        if self.existing_lock.version != 0
            && self.existing_lock.package.len() != self.new_lock.package.len()
        {
            return Err(vec!["lock contains unused Git dependency entries".into()]);
        }
        let mut lock = LockFile {
            version: 2,
            package: self.new_lock.package.clone(),
        };
        lock.package
            .sort_by(|a, b| (&a.owner, &a.name).cmp(&(&b.owner, &b.name)));
        let text = toml::to_string_pretty(&lock).map_err(|error| vec![error.to_string()])?;
        cache::atomic_write(&self.project_root.join("ond.lock"), text.as_bytes())
    }
}

fn read_manifest(path: &Path) -> DriverResult<Manifest> {
    let text = fs::read_to_string(path)
        .map_err(|error| vec![format!("failed to read `{}`: {error}", path.display())])?;
    let mut manifest: Manifest = toml::from_str(&text)
        .map_err(|error| vec![format!("invalid manifest `{}`: {error}", path.display())])?;
    manifest.roles = ond_compiler_core::roles::configuration_from_text(path, &text)
        .map_err(|error| vec![error])?;
    manifest.text = text;
    Ok(manifest)
}

fn read_lock(path: &Path) -> DriverResult<LockFile> {
    if !path.is_file() {
        return Ok(LockFile::default());
    }
    let text = fs::read_to_string(path)
        .map_err(|error| vec![format!("failed to read `{}`: {error}", path.display())])?;
    let lock: LockFile = toml::from_str(&text)
        .map_err(|error| vec![format!("invalid lock file `{}`: {error}", path.display())])?;
    if lock.version != 2 {
        return Err(vec![format!(
            "unsupported ond.lock version: {}",
            lock.version
        )]);
    }
    let mut identities = BTreeSet::new();
    for entry in &lock.package {
        if !identities.insert((&entry.owner, &entry.name))
            || entry.rev.as_deref().is_some_and(str::is_empty)
            || !matches!(entry.commit.len(), 40 | 64)
            || !entry
                .commit
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(vec!["invalid or duplicate Git lock entry".into()]);
        }
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
            && !ond_compiler_core::project::is_test_source(&path)
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
    if name == "_"
        || !ond_compiler_core::parse_source_file(
            ond_compiler_core::source::FileId(0),
            &format!("package {name}\n"),
        )
        .is_ok_and(|file| file.package.name == name)
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
                "[project]\nname = \"app\"\n[dependencies]\nrelative = {{ path = \"../relative\" }}\nabsolute = {{ path = {:?} }}\n",
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
            "[project]\nname = \"app\"\n[dependencies]\nlib = { path = \"../library\" }\n",
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
            format!("[project]\nname = \"app\"\n[dependencies]\nagain = {{ path = \".\" }}\ngitlib = {{ git = {repository_url:?} }}\n"),
        )
        .unwrap();

        let app = app.canonicalize().unwrap();
        let first = resolve_dependencies(&app).unwrap();
        assert!(first.sources.contains_key("remote/gitlib.ond"));
        let lock = fs::read_to_string(app.join("ond.lock")).unwrap();
        assert!(lock.contains("commit ="));
        assert_eq!(toml::from_str::<LockFile>(&lock).unwrap().package.len(), 1);
        let second = resolve_dependencies(&app).unwrap();
        assert_eq!(first.sources, second.sources);
    }

    #[test]
    fn diamond_git_edges_share_a_checkout_without_merging_locked_commits() {
        let temp = tempfile_root("git-diamond");
        let app = temp.join("app");
        let repository = temp.join("repository");
        for path in [&app, &repository, &temp.join("left"), &temp.join("right")] {
            fs::create_dir_all(path).unwrap();
        }
        fs::write(
            repository.join("ond.toml"),
            "[project]\nname = \"common\"\n",
        )
        .unwrap();
        fs::write(
            repository.join("common.ond"),
            "package common\nfunc Value()->u32{return 1}\n",
        )
        .unwrap();
        git(&repository, &["init"]);
        git(
            &repository,
            &["config", "user.email", "ond@example.invalid"],
        );
        git(&repository, &["config", "user.name", "Ond Test"]);
        git(&repository, &["add", "."]);
        git(&repository, &["commit", "-m", "initial"]);
        let url = repository.to_string_lossy().replace('\\', "/");
        fs::write(app.join("ond.toml"),"[project]\nname = \"app\"\n[dependencies]\nleft={path=\"../left\"}\nright={path=\"../right\"}\n").unwrap();
        for (owner, key) in [("left", "common"), ("right", "shared")] {
            fs::write(temp.join(owner).join("ond.toml"),format!("[project]\nname = \"{owner}\"\n[dependencies]\n{key}={{git={url:?},rev=\"HEAD\"}}\n")).unwrap();
        }
        let first = resolve_dependencies(&app).expect("same Git content through diamond edges");
        let lock = read_lock(&app.join("ond.lock")).unwrap();
        assert_eq!(lock.package.len(), 2);
        assert_eq!(lock.package[0].commit, lock.package[1].commit);
        assert_eq!(resolve_dependencies(&app).unwrap().sources, first.sources);
        let original = first.sources["common/common.ond"].path.clone();
        fs::write(
            repository.join("common.ond"),
            "package common\nfunc Value()->u32{return 2}\n",
        )
        .unwrap();
        git(&repository, &["add", "."]);
        git(&repository, &["commit", "-m", "next"]);
        let next = git_output(&repository, ["rev-parse", "HEAD"], "common").unwrap();
        let mut lock = lock;
        lock.package[1].commit = next;
        fs::write(app.join("ond.lock"), toml::to_string(&lock).unwrap()).unwrap();
        assert!(
            resolve_dependencies(&app)
                .unwrap_err()
                .join(" ")
                .contains("namespace")
        );
        assert!(fs::read_to_string(original).unwrap().contains("return 1"));
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
            "[project]\nname = \"app\"\n[dependencies]\nconsumer_alias = { path = \"../library\" }\n",
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
            "[project]\nname = \"app\"\n[dependencies]\nlib = { path = \"../library\" }\n",
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

#[cfg(test)]
mod conformance_lock_tests {
    use super::*;
    const K: &str = "version = 2\n[[package]]\nowner = \"app\"\nname = \"math_dep\"\ngit = \"https://example.invalid/math.git\"\nrev = \"release\"\ncommit = \"0123456789012345678901234567890123456789\"\n";
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn read_input(text: &str) -> DriverResult<LockFile> {
        let root = Scratch(std::env::temp_dir().join(format!(
                "ond-lock-conformance-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )));
        fs::create_dir(&root.0).unwrap();
        let path = root.0.join("ond.lock");
        fs::write(&path, text).unwrap();
        read_lock(&path)
    }
    #[test]
    fn dep01_schema() {
        read_input(K).expect("DEP-01.schema: real lock reader; no repository resolution");
    }
    #[test]
    fn dep01_zero_edges() {
        read_input("version = 2\npackage = []\n").expect("DEP-01.zero-edges");
    }
    #[test]
    fn dep01_version_one() {
        read_input(K).expect("DEP-01.version-one: revision 2 baseline precondition");
        assert!(
            read_input(&K.replacen("version = 2", "version = 1", 1)).is_err(),
            "DEP-01.version-one"
        );
    }
}
