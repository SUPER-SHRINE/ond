use std::collections::BTreeMap;
use std::path::Path;

use crate::diagnostic::FrontendError;
use crate::ir::{ast, hir, mir, object};
use crate::mir_backend;
use crate::project::LoadedProject;

#[derive(Debug, Clone)]
pub struct Compilation {
    pub loaded: LoadedProject,
    pub ast: ast::Project,
    pub hir: hir::Project,
    pub mir: mir::Project,
    pub objects: Vec<object::Object>,
    pub debug_objects: Vec<ond_protocol::debug::DebugObject>,
}

pub fn compile_project(root: impl AsRef<Path>) -> Result<Compilation, FrontendError> {
    compile_project_with_overrides(root, &BTreeMap::new())
}

pub fn compile_project_with_overrides(
    root: impl AsRef<Path>,
    overrides: &BTreeMap<std::path::PathBuf, String>,
) -> Result<Compilation, FrontendError> {
    let loaded = ond_compiler_core::project::load_project_with_overrides(root, overrides)?;
    compile_loaded_project(loaded)
}

pub fn compile_loaded_project(loaded: LoadedProject) -> Result<Compilation, FrontendError> {
    let frontend = ond_compiler_core::compile_loaded_project(loaded)?;
    let mut debug_paths = BTreeMap::new();
    for package in &frontend.loaded.packages {
        for file_id in &package.files {
            let file = frontend.loaded.sources.file(*file_id);
            let relative = file.path().strip_prefix(&package.directory).unwrap_or(file.path());
            let relative = relative.to_string_lossy().replace('\\', "/");
            let path = if package.logical_path == "." {
                relative
            } else {
                format!("{}/{relative}", package.logical_path)
            };
            debug_paths.insert(*file_id, path);
        }
    }
    let generated = mir_backend::codegen_with_debug(
        &frontend.mir,
        &frontend.loaded.sources,
        &debug_paths,
    )?;
    Ok(Compilation {
        loaded: frontend.loaded,
        ast: frontend.ast,
        hir: frontend.hir,
        mir: frontend.mir,
        objects: generated.objects,
        debug_objects: generated.debug,
    })
}
