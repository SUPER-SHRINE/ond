use crate::{CompileRequest, CompileResult};
use crate::{DriverResult, cache, cache_identity, graph::Graph, input};
use ond_protocol::{ArtifactRef, CompilationManifest, PackageArtifact, ProgramMetadata};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Serialize, Deserialize)]
struct ProjectCache {
    key: String,
    manifest: CompilationManifest,
}

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
    let context = (
        &request.contract,
        compiler_id,
        cache::hash(&manifest_bytes),
        request.test,
    );
    let project_sources: BTreeMap<_, _> = loaded
        .packages
        .iter()
        .map(|package| {
            let sources = package
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
                .collect::<Vec<_>>();
            (package.logical_path.clone(), sources)
        })
        .collect();
    let project_key = cache::hash(&cache::encode(&(
        &context,
        &project_sources,
        &graph.imports,
    ))?);
    if let Some(result) = load_project_cache(&directory, &project_key, &context, &graph) {
        return Ok(result);
    }

    let mut frontend = if graph.has_generic_declarations {
        Some(
            ond_compiler_core::compile_loaded_project(loaded.clone())
                .map_err(|e| input::errors(e, &loaded))?,
        )
    } else {
        None
    };
    let generic_inputs = if let Some(frontend) = &frontend {
        Some(generic_build_inputs(frontend, &graph)?)
    } else {
        None
    };
    let mut ids = BTreeMap::<String, String>::new();
    let mut dependencies = BTreeMap::new();
    let mut artifacts = BTreeMap::<String, PackageArtifact>::new();
    let mut references = BTreeMap::<String, ArtifactRef>::new();
    let mut missing = BTreeSet::new();

    for path in &graph.order {
        let sources = &project_sources[path];
        let deps = dependency_ids(path, &graph, &ids);
        let id =
            if let Some(package_identity) = generic_inputs.as_ref().and_then(|all| all.get(path)) {
                cache::hash(&cache::encode(&(
                    &context,
                    path,
                    sources,
                    &deps,
                    package_identity,
                ))?)
            } else {
                cache::hash(&cache::encode(&(&context, path, sources, &deps))?)
            };
        if let Some((reference, artifact)) =
            cache::lookup(&directory, &id).filter(|(_, artifact)| {
                artifact.compiler_id == compiler_id
                    && artifact.package == *path
                    && artifact.build_id == id
                    && artifact.dependencies == deps
                    && artifact.imports == graph.imports[path]
            })
        {
            references.insert(path.clone(), reference);
            artifacts.insert(path.clone(), artifact);
        } else {
            missing.insert(path.clone());
        }
        ids.insert(path.clone(), id);
        dependencies.insert(path.clone(), deps);
    }

    if frontend.is_none() && !missing.is_empty() {
        frontend = Some(
            ond_compiler_core::compile_loaded_project(loaded.clone())
                .map_err(|e| input::errors(e, &loaded))?,
        );
    }

    if let Some(frontend) = &frontend
        && !missing.is_empty()
    {
        let debug_paths = debug_paths(&frontend.loaded);
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
                .find(|package| package.logical_path == path)
                .expect("emitted package");
            artifacts.insert(
                path.clone(),
                PackageArtifact {
                    tests: request.test.then(|| ond_protocol::TestMetadata {
                        cases: graph
                            .tests
                            .iter()
                            .filter(|case| case.package == path)
                            .cloned()
                            .collect(),
                    }),
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
    let manifest = CompilationManifest {
        tests: request.test.then(|| ond_protocol::TestMetadata {
            cases: graph.tests.clone(),
        }),
        contract: request.contract,
        compiler_id: compiler_id.into(),
        packages: references.into_values().collect(),
        program,
    };
    let result = CompileResult {
        manifest: manifest.clone(),
        frontend_ran: frontend.is_some(),
        rebuilt: missing.into_iter().collect(),
        reused,
    };
    cache::atomic_write(
        &directory.join(format!("project-{project_key}.ondmanifest")),
        &cache::encode(&ProjectCache {
            key: project_key,
            manifest,
        })?,
    )?;
    Ok(result)
}

fn dependency_ids(
    path: &str,
    graph: &Graph,
    ids: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    graph.imports[path]
        .iter()
        .map(|dep| (dep.clone(), ids[dep].clone()))
        .collect()
}

fn generic_build_inputs(
    frontend: &ond_compiler_core::Compilation,
    graph: &Graph,
) -> DriverResult<BTreeMap<String, String>> {
    graph
        .order
        .iter()
        .map(|path| {
            let package = frontend
                .mir
                .packages
                .iter()
                .find(|package| package.logical_path == *path)
                .expect("frontend package");
            let generated_inputs = cache_identity::package_inputs(&frontend.mir, package);
            let interface =
                ond_compiler_core::export::package(&frontend.hir, path).map_err(|e| vec![e])?;
            let identity = cache::hash(&cache::encode(&(
                generated_inputs,
                cache::hash(&cache::encode(&interface)?),
            ))?);
            Ok((path.clone(), identity))
        })
        .collect()
}

fn debug_paths(
    loaded: &ond_compiler_core::LoadedProject,
) -> BTreeMap<ond_compiler_core::source::FileId, String> {
    let mut paths = BTreeMap::new();
    for package in &loaded.packages {
        for file_id in &package.files {
            let file = loaded.sources.file(*file_id);
            let relative = file
                .path()
                .strip_prefix(&package.directory)
                .unwrap_or(file.path())
                .to_string_lossy()
                .replace('\\', "/");
            let path = file.display_path().map_or_else(
                || {
                    if package.logical_path == "." {
                        relative
                    } else {
                        format!("{}/{relative}", package.logical_path)
                    }
                },
                str::to_string,
            );
            paths.insert(*file_id, path);
        }
    }
    paths
}

fn load_project_cache(
    directory: &std::path::Path,
    key: &str,
    context: &(&ond_protocol::Contract, &str, String, bool),
    graph: &Graph,
) -> Option<CompileResult> {
    let cache: ProjectCache =
        cache::read_json(&directory.join(format!("project-{key}.ondmanifest"))).ok()?;
    let manifest = cache.manifest;
    if cache.key != key
        || manifest.contract != *context.0
        || manifest.compiler_id != context.1
        || manifest.packages.len() != graph.order.len()
        || manifest.tests
            != context.3.then(|| ond_protocol::TestMetadata {
                cases: graph.tests.clone(),
            })
        || manifest.program.entry_symbol != "main.main"
    {
        return None;
    }
    let mut references = BTreeMap::new();
    for reference in &manifest.packages {
        let reference = reference.clone();
        if references
            .insert(reference.package.clone(), reference)
            .is_some()
        {
            return None;
        }
    }
    if references.keys().collect::<BTreeSet<_>>() != graph.imports.keys().collect::<BTreeSet<_>>() {
        return None;
    }
    let mut artifacts = BTreeMap::new();
    for path in &graph.order {
        let expected = references.get(path)?;
        let (actual, artifact) = cache::lookup(directory, &expected.build_id)?;
        if cache::encode(&actual).ok()? != cache::encode(expected).ok()?
            || artifact.compiler_id != context.1
            || artifact.contract != *context.0
            || artifact.package != *path
            || artifact.imports != graph.imports[path]
        {
            return None;
        }
        let dependencies = dependency_ids_from_references(path, graph, &references);
        if artifact.dependencies != dependencies {
            return None;
        }
        artifacts.insert(path.clone(), artifact);
    }
    let initializers = graph
        .initialization_order
        .iter()
        .filter_map(|path| artifacts[path].initializer.clone())
        .collect::<Vec<_>>();
    if manifest.program.initializers != initializers {
        return None;
    }
    Some(CompileResult {
        manifest,
        frontend_ran: false,
        rebuilt: Vec::new(),
        reused: references.keys().cloned().collect(),
    })
}

fn dependency_ids_from_references(
    path: &str,
    graph: &Graph,
    references: &BTreeMap<String, ArtifactRef>,
) -> BTreeMap<String, String> {
    graph.imports[path]
        .iter()
        .map(|dep| (dep.clone(), references[dep].build_id.clone()))
        .collect()
}
