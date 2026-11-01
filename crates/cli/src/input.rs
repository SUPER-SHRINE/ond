use crate::{CompileRequest, DriverResult};
use ond_compiler_core::LoadedProject;
use std::collections::BTreeMap;

pub fn load(request: &CompileRequest) -> DriverResult<LoadedProject> {
    request.contract.validate().map_err(|error| vec![error])?;
    let resolved = crate::dependencies::resolve_dependencies_with_role(
        &request.project_root,
        request.role.as_deref(),
    )?;
    let mut excluded_dirs = request.excluded_dirs.clone();
    excluded_dirs.extend(resolved.ignored_directories);
    let loaded = ond_compiler_core::load_project_with_role_inputs(
        &request.project_root,
        &BTreeMap::new(),
        &excluded_dirs,
        &request.sources,
        &resolved.sources,
        ond_compiler_core::RoleLoadOptions {
            selected: request.role.clone(),
            dependencies: resolved.role_inputs,
            ..Default::default()
        },
    )
    .map_err(|error| {
        error
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>()
    })?;
    Ok(loaded)
}

pub fn errors(error: ond_compiler_core::FrontendError, loaded: &LoadedProject) -> Vec<String> {
    error
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.render(&loaded.sources))
        .collect()
}

pub(super) fn logical_packages(loaded: &mut LoadedProject) -> DriverResult<Option<String>> {
    let text =
        std::fs::read_to_string(&loaded.manifest_path).map_err(|error| vec![error.to_string()])?;
    let manifest: toml::Value = toml::from_str(&text).map_err(|error| vec![error.to_string()])?;
    let name = manifest
        .get("project")
        .and_then(|project| project.get("name"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| vec!["project name is required".into()])?
        .to_string();
    let application = loaded
        .packages
        .iter()
        .find(|package| package.logical_path == "." && package.directory == loaded.root)
        .map(|_| name.clone());
    for package in &mut loaded.packages {
        if package.logical_path == "." {
            package.logical_path = name.clone();
        }
    }
    Ok(application)
}
