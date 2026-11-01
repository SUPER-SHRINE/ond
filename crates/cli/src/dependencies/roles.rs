//! Validation of every registered project edge, before source reachability.
use super::{Dependency, DriverResult, Manifest, Path, PathBuf};
use ond_compiler_core::roles::DependencyInputs;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct RoleGraph {
    pub(super) inputs: DependencyInputs,
    paths: BTreeMap<PathBuf, (Option<String>, String)>,
}
impl RoleGraph {
    pub(super) fn root(
        &mut self,
        root: &Path,
        manifest: &Manifest,
        name: &str,
        selected: Option<&str>,
    ) -> DriverResult<()> {
        if let Some(selected) = selected {
            let roles = manifest.roles.as_ref().ok_or_else(|| {
                vec![format!(
                    "{}: project {name} has no role definitions for --role `{selected}`",
                    root.join("ond.toml").display()
                )]
            })?;
            if !roles.names.contains(selected) {
                return Err(vec![format!(
                    "{}: unknown role `{selected}`; project {name} must declare the role in roles.names",
                    root.join("ond.toml").display()
                )]);
            }
        }
        self.inputs
            .manifests
            .insert(root.into(), manifest.text.clone());
        self.inputs
            .selected_roles
            .insert(root.into(), selected.map(str::to_string));
        self.paths
            .insert(root.into(), (selected.map(str::to_string), name.into()));
        Ok(())
    }
    pub(super) fn edge(
        &mut self,
        owner: &Path,
        manifest: &Manifest,
        name: &str,
        target: &Path,
        dependency: &Manifest,
    ) -> DriverResult<()> {
        let owner_name = &manifest
            .project
            .as_ref()
            .expect("validated owner namespace")
            .name;
        let target_project = dependency
            .project
            .as_ref()
            .expect("validated target namespace");
        let entry = format!("{} [dependencies].{name}", owner.join("ond.toml").display());
        if target_project.final_project {
            return Err(vec![format!(
                "{entry}: {owner_name} cannot register final project {}; {} project.final=true",
                target_project.name,
                target.join("ond.toml").display()
            )]);
        }
        let selected = match &manifest.dependencies[name] {
            Dependency::Detailed {
                role: Some(role), ..
            } => {
                let role = role
                    .as_str()
                    .ok_or_else(|| vec![format!("{entry}.role must be string")])?;
                if !manifest.project.as_ref().unwrap().final_project {
                    return Err(vec![format!(
                        "{entry}: nonfinal project {owner_name} cannot specify dependency role `{role}` for {}",
                        target_project.name
                    )]);
                }
                let configuration = dependency.roles.as_ref().ok_or_else(|| {
                    vec![format!(
                        "{entry}.role: project {} has no role definitions ({})",
                        target_project.name,
                        target.join("ond.toml").display()
                    )]
                })?;
                if !configuration.names.contains(role) {
                    return Err(vec![format!(
                        "{entry}.role: unknown role `{role}` in project {} ({})",
                        target_project.name,
                        target.join("ond.toml").display()
                    )]);
                }
                Some(role.to_string())
            }
            _ => None,
        };
        let role = selected.as_deref().unwrap_or("未指定 (common)");
        let parent = &self.paths[owner].1;
        let route = format!(
            "{parent} --{name} [role {role}; {entry}]→ {}",
            target_project.name
        );
        if let Some((previous, previous_path)) = self.paths.get(target) {
            if previous != &selected {
                return Err(vec![format!(
                    "conflicting role selections for project {}: role {} via {previous_path}; role {role} via {route}",
                    target_project.name,
                    previous.as_deref().unwrap_or("未指定 (common)")
                )]);
            }
        } else {
            self.paths.insert(target.into(), (selected.clone(), route));
        }
        self.inputs
            .manifests
            .insert(target.into(), dependency.text.clone());
        self.inputs.selected_roles.insert(target.into(), selected);
        Ok(())
    }
}
