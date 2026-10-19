use crate::{CompileRequest, DriverResult};
use ond_compiler_core::{LoadedProject, PackageId, PackageSources};
use std::collections::BTreeMap;

pub fn load(request: &CompileRequest) -> DriverResult<LoadedProject> {
    request.contract.validate().map_err(|error| vec![error])?;
    let loader = if request.test {
        ond_compiler_core::load_project_with_test_inputs
    } else {
        ond_compiler_core::load_project_with_inputs
    };
    let mut loaded = loader(
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
            .collect::<Vec<_>>()
    })?;
    if request.test
        && !loaded
            .packages
            .iter()
            .any(|package| package.logical_path == ".")
    {
        // The test linker enters through case wrappers. Give the existing graph
        // and compiler an in-memory entry package without changing library files.
        let directory = loaded.root.join(".ond/__test_entry__");
        let file = loaded.sources.add_file(
            directory.join("main.ond"),
            "package main\nfunc main() {}\n".to_string(),
        );
        loaded.packages.insert(
            0,
            PackageSources {
                id: PackageId(0),
                logical_path: ".".to_string(),
                directory,
                files: vec![file],
            },
        );
        for (index, package) in loaded.packages.iter_mut().enumerate() {
            package.id = PackageId(index);
        }
    }
    Ok(loaded)
}

pub fn errors(error: ond_compiler_core::FrontendError, loaded: &LoadedProject) -> Vec<String> {
    error
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.render(&loaded.sources))
        .collect()
}
