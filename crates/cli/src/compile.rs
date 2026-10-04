use crate::{CompileRequest, CompileResult};
use crate::{DriverResult, cache, graph::Graph, input};
use ond_protocol::{ArtifactRef, CompilationManifest, PackageArtifact, ProgramMetadata};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

/// Compile with a fingerprint covering this compiler and its runtime implementation.
/// Production callers should use `compiler_identity()`; a version string is insufficient.
pub fn compile(request: CompileRequest, compiler_id: &str) -> DriverResult<CompileResult> {
    let loaded = input::load(&request)?;
    let graph = Graph::load(&loaded)?;
    let directory = request
        .cache_dir
        .as_ref()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                loaded.root.join(p)
            }
        })
        .unwrap_or_else(|| loaded.root.join("target/ond-cache"));
    fs::create_dir_all(&directory).map_err(|e| vec![e.to_string()])?;
    let directory = directory.canonicalize().map_err(|e| vec![e.to_string()])?;
    let manifest_bytes = fs::read(&loaded.manifest_path).map_err(|e| vec![e.to_string()])?;
    let context = (&request.contract, compiler_id, cache::hash(&manifest_bytes));
    let mut ids = BTreeMap::<String, String>::new();
    let mut dependencies = BTreeMap::new();
    let mut artifacts = BTreeMap::<String, PackageArtifact>::new();
    let mut references = BTreeMap::<String, ArtifactRef>::new();
    let mut missing = BTreeSet::new();
    for path in &graph.order {
        let package = loaded
            .packages
            .iter()
            .find(|p| &p.logical_path == path)
            .expect("graph package");
        let sources: Vec<_> = package
            .files
            .iter()
            .map(|id| {
                let file = loaded.sources.file(*id);
                (
                    file.path()
                        .strip_prefix(&loaded.root)
                        .expect("project source")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    cache::hash(file.text().as_bytes()),
                )
            })
            .collect();
        let deps: BTreeMap<_, _> = graph.imports[path]
            .iter()
            .map(|dep| (dep.clone(), ids[dep].clone()))
            .collect();
        let id = cache::hash(&cache::encode(&(&context, path, sources, &deps))?);
        if let Some((reference, artifact)) = cache::lookup(&directory, &id).filter(|(_, a)| {
            a.compiler_id == compiler_id
                && a.package == *path
                && a.build_id == id
                && a.dependencies == deps
                && a.imports == graph.imports[path]
        }) {
            references.insert(path.clone(), reference);
            artifacts.insert(path.clone(), artifact);
        } else {
            missing.insert(path.clone());
        }
        ids.insert(path.clone(), id);
        dependencies.insert(path.clone(), deps);
    }
    if !missing.is_empty() {
        // Until export-data import exists, semantic analysis/MIR lowering stays whole-project.
        let frontend = ond_compiler_core::compile_loaded_project(loaded.clone())
            .map_err(|e| input::errors(e, &loaded))?;
        let mut debug_paths = BTreeMap::new();
        for package in &frontend.loaded.packages {
            for file_id in &package.files {
                let file = frontend.loaded.sources.file(*file_id);
                let relative = file
                    .path()
                    .strip_prefix(&package.directory)
                    .unwrap_or(file.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                let path = if package.logical_path == "." {
                    relative
                } else {
                    format!("{}/{relative}", package.logical_path)
                };
                debug_paths.insert(*file_id, path);
            }
        }
        let generated = ond_compiler::mir_backend::codegen_selected_with_debug(
            &frontend.mir,
            &missing,
            Some((&frontend.loaded.sources, &debug_paths)),
        )
        .map_err(|e| input::errors(e, &loaded))?;
        let mut debug_by_package: BTreeMap<_, _> = generated
            .debug
            .into_iter()
            .map(|debug| (debug.package.clone(), debug))
            .collect();
        for object in generated.objects {
            let path = object.name.clone();
            let package = frontend
                .mir
                .packages
                .iter()
                .find(|p| p.logical_path == path)
                .expect("emitted package");
            artifacts.insert(
                path.clone(),
                PackageArtifact {
                    interface: ond_compiler_core::export::package(&frontend.hir, &path)
                        .map_err(|e| vec![e])?,
                    contract: request.contract.clone(),
                    compiler_id: compiler_id.into(),
                    build_id: ids[&path].clone(),
                    dependencies: dependencies[&path].clone(),
                    imports: graph.imports[&path].clone(),
                    initializer: package.initializer.as_ref().map(|f| f.name.clone()),
                    package: path,
                    debug: debug_by_package
                        .remove(&object.name)
                        .expect("emitted package debug data"),
                    object,
                },
            );
        }
        // Publish only after semantic analysis and all requested Object emission succeed.
        for path in &missing {
            references.insert(path.clone(), cache::publish(&directory, &artifacts[path])?);
        }
    }
    let program = ProgramMetadata {
        entry_symbol: "main.main".into(),
        initializers: graph
            .initialization_order
            .iter()
            .filter_map(|path| artifacts[path].initializer.clone())
            .collect(),
    };
    let reused = references
        .keys()
        .filter(|p| !missing.contains(*p))
        .cloned()
        .collect();
    Ok(CompileResult {
        manifest: CompilationManifest {
            contract: request.contract,
            compiler_id: compiler_id.into(),
            packages: references.into_values().collect(),
            program,
        },
        frontend_ran: !missing.is_empty(),
        rebuilt: missing.into_iter().collect(),
        reused,
    })
}
