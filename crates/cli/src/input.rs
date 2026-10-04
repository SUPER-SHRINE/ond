use crate::{CompileRequest, DriverResult};
use ond_compiler_core::LoadedProject;
use std::collections::BTreeMap;

pub fn load(request: &CompileRequest) -> DriverResult<LoadedProject> {
    request.contract.validate().map_err(|error| vec![error])?;
    ond_compiler_core::load_project_with_inputs(
        &request.project_root,
        &BTreeMap::new(),
        &request.excluded_dirs,
        &request.sources,
        &request.libraries,
        &BTreeMap::new(),
    )
    .map_err(|error| {
        error
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect()
    })
}

pub fn errors(error: ond_compiler_core::FrontendError, loaded: &LoadedProject) -> Vec<String> {
    error
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.render(&loaded.sources))
        .collect()
}
