//! Project role settings and immutable source-position contexts.
use crate::{
    diagnostic::{Diagnostic, FrontendError},
    source::{FileId, SourceDb, Span},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleConfiguration {
    pub names: BTreeSet<String>,
    pub selected: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    names: Vec<String>,
}

/// Resolved manifest inputs shared by the build driver and editor snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DependencyInputs {
    pub selected_roles: BTreeMap<PathBuf, Option<String>>,
    pub manifests: BTreeMap<PathBuf, String>,
}

pub fn read_configuration(path: &Path) -> Result<Option<RoleConfiguration>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    configuration_from_text(path, &text)
}

pub fn configuration_from_text(
    path: &Path,
    text: &str,
) -> Result<Option<RoleConfiguration>, String> {
    let manifest: toml::Value =
        toml::from_str(text).map_err(|error| format!("{}: {error}", path.display()))?;
    if manifest
        .get("project")
        .and_then(|project| project.get("final"))
        .is_some_and(|value| !value.is_bool())
    {
        return Err(format!("{}: project.final must be bool", path.display()));
    }
    let Some(value) = manifest.get("roles") else {
        return Ok(None);
    };
    let settings: Settings = value
        .clone()
        .try_into()
        .map_err(|error| format!("{}: invalid roles settings: {error}", path.display()))?;
    let names: BTreeSet<_> = settings.names.iter().cloned().collect();
    if names.is_empty() || names.len() != settings.names.len() {
        return Err("roles.names must be nonempty and contain no duplicates".into());
    }
    for name in &names {
        if !crate::ond::role_identifier(name) {
            return Err(format!("invalid role identifier `{name}`"));
        }
    }
    Ok(Some(RoleConfiguration {
        names,
        selected: None,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleRegion {
    pub start: usize,
    pub end: usize,
    pub targets: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRoles {
    pub project: PathBuf,
    pub configuration: Option<RoleConfiguration>,
    pub regions: Vec<RoleRegion>,
    pub context_valid: bool,
}

impl SourceRoles {
    pub fn context(&self, offset: usize) -> Option<&BTreeSet<String>> {
        self.regions
            .iter()
            .rev()
            .find(|region| region.start <= offset && offset < region.end)
            .map(|region| &region.targets)
    }
    pub fn selected(&self, offset: usize) -> bool {
        match self.context(offset) {
            Some(targets) => self
                .configuration
                .as_ref()
                .and_then(|config| config.selected.as_ref())
                .is_some_and(|selected| targets.contains(selected)),
            None => true,
        }
    }
}

fn failure(message: String) -> FrontendError {
    FrontendError::new(vec![Diagnostic::error(Span::synthetic(), message)])
}

/// Capture project settings before any parsing or semantic-cache lookup.
pub(crate) fn capture(sources: &mut SourceDb, root: &Path) -> Result<(), FrontendError> {
    let mut inputs = sources.dependency_inputs().clone();
    let mut configurations: BTreeMap<PathBuf, Option<RoleConfiguration>> = BTreeMap::new();
    let mut configuration = |project: &Path| -> Result<Option<RoleConfiguration>, FrontendError> {
        let text = if let Some(text) = inputs.manifests.get(project) {
            text.clone()
        } else {
            let path = project.join("ond.toml");
            let text = std::fs::read_to_string(&path)
                .map_err(|error| failure(format!("{}: {error}", path.display())))?;
            inputs.manifests.insert(project.to_path_buf(), text.clone());
            text
        };
        let mut configuration =
            configuration_from_text(&project.join("ond.toml"), &text).map_err(failure)?;
        if let Some(selected) = inputs.selected_roles.get(project).and_then(Option::as_ref) {
            let value = configuration.as_mut().ok_or_else(|| {
                failure(format!(
                    "{}: project has no role definitions",
                    project.display()
                ))
            })?;
            if !value.names.contains(selected) {
                return Err(failure(format!(
                    "{}: unknown role `{selected}`",
                    project.display()
                )));
            }
            value.selected = Some(selected.clone());
        }
        Ok(configuration)
    };
    configurations.insert(root.to_path_buf(), configuration(root)?);
    for index in 0..sources.files().len() {
        let file = sources.file(FileId(index));
        let project = file
            .path()
            .parent()
            .into_iter()
            .flat_map(Path::ancestors)
            .find(|directory| directory.join("ond.toml").is_file())
            .unwrap_or(root)
            .to_path_buf();
        let configuration = if let Some(config) = configurations.get(&project) {
            config.clone()
        } else {
            let config = configuration(&project)?;
            configurations.insert(project.clone(), config.clone());
            config
        };
        let regions = crate::ond::role_regions(FileId(index), file.text(), configuration.as_ref());
        let context_valid = regions.is_ok();
        let regions = regions.unwrap_or_default();
        sources.set_roles(
            FileId(index),
            SourceRoles {
                project,
                configuration,
                regions,
                context_valid,
            },
        );
    }
    sources.set_dependency_inputs(inputs);
    Ok(())
}

/// Whether two declarations can occur in the same role projection.
pub fn overlap(sources: &SourceDb, a: Span, b: Span) -> bool {
    match (
        sources
            .roles(a.file)
            .and_then(|source| source.context(a.start)),
        sources
            .roles(b.file)
            .and_then(|source| source.context(b.start)),
    ) {
        (Some(a), Some(b)) => !a.is_disjoint(b),
        (Some(targets), None) | (None, Some(targets)) => !targets.is_empty(),
        _ => true,
    }
}

pub fn visible(sources: &SourceDb, declaration: Span, caller: Span) -> bool {
    if declaration.is_synthetic() || caller.is_synthetic() {
        return true;
    }
    let reference = sources.roles(caller.file);
    if reference
        .and_then(|role| role.context(caller.start))
        .is_some_and(BTreeSet::is_empty)
    {
        return false;
    }
    let Some(owner) = sources.roles(declaration.file) else {
        return true;
    };
    if reference.is_some_and(|reference| reference.project != owner.project) {
        return true;
    }
    let declaration = owner.context(declaration.start);
    match (
        reference.and_then(|reference| reference.context(caller.start)),
        declaration,
    ) {
        (_, None) => true,
        (Some(context), Some(targets)) => !context.is_empty() && context.is_subset(targets),
        (None, Some(_)) => false,
    }
}
