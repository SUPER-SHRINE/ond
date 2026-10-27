use crate::{CompileRequest, CompileResult};
use crate::{DriverResult, cache, graph::Graph, input};
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

#[derive(Serialize, Deserialize)]
struct GenerationCache {
    key: String,
    packages: BTreeMap<String, serde_json::Value>,
}

/// Compile with a fingerprint covering this compiler and its runtime implementation.
/// Production callers should use `compiler_identity()`; a version string is insufficient.
pub fn compile(request: CompileRequest, compiler_id: &str) -> DriverResult<CompileResult> {
    let settings = crate::settings::read_settings(&request.project_root)?;
    let source_loaded = input::load(&request)?;
    let mut loaded = source_loaded.clone();
    let application_root = input::logical_packages(&mut loaded)?;
    let graph = Graph::load(&loaded, application_root.as_deref())?;
    let roles: BTreeMap<_, _> = loaded
        .packages
        .iter()
        .map(|package| {
            let configuration = package
                .files
                .iter()
                .find_map(|file| loaded.sources.roles(*file))
                .and_then(|roles| roles.configuration.clone());
            (package.logical_path.clone(), configuration)
        })
        .collect();
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
        .unwrap_or_else(|| loaded.root.join(settings.compile.cache_dir));
    fs::create_dir_all(&directory).map_err(|e| vec![e.to_string()])?;
    let directory = directory.canonicalize().map_err(|e| vec![e.to_string()])?;
    let manifest_bytes = fs::read(&loaded.manifest_path).map_err(|e| vec![e.to_string()])?;
    let context = (&request.contract, compiler_id, cache::hash(&manifest_bytes));
    let project_sources: BTreeMap<_, _> = loaded
        .packages
        .iter()
        .map(|package| {
            let mut sources = package
                .files
                .iter()
                .map(|id| {
                    let file = loaded.sources.file(*id);
                    (
                        file.path()
                            .strip_prefix(&package.directory)
                            .expect("project source")
                            .to_string_lossy()
                            .replace('\\', "/"),
                        cache::hash(file.text().as_bytes()),
                    )
                })
                .collect::<Vec<_>>();
            sources.sort_by(|a, b| a.0.cmp(&b.0));
            (package.logical_path.clone(), sources)
        })
        .collect();
    let project_key = cache::hash(&cache::encode(&(
        &context,
        &project_sources,
        &graph.imports,
        &roles,
        loaded.sources.dependency_inputs(),
    ))?);
    if let Some(mut result) = load_project_cache(&directory, &project_key, &context, &graph) {
        legacy_labels(&mut result, application_root.as_deref());
        return Ok(result);
    }

    let generation_cache_path = directory.join(format!("generation-{project_key}.json"));
    let cached_generation = cache::read_json::<GenerationCache>(&generation_cache_path)
        .ok()
        .filter(|cached| {
            cached.key == project_key && cached.packages.keys().eq(graph.imports.keys())
        });
    if let Some(cached) = cached_generation
        && let Some(mut result) = reuse_generation(
            &cached,
            &directory,
            &context,
            &project_sources,
            &graph,
            &roles,
        )?
    {
        legacy_labels(&mut result, application_root.as_deref());
        return Ok(result);
    }
    let (frontend, generation) =
        ond_compiler_core::compile_loaded_project_with_generation(source_loaded)
            .map_err(|e| input::errors(e, &loaded))?;
    let generation = GenerationCache {
        key: project_key.clone(),
        packages: generation
            .packages
            .into_iter()
            .map(|(path, json)| {
                serde_json::from_str(&json)
                    .map(|value| (path, value))
                    .map_err(|error| vec![error.to_string()])
            })
            .collect::<DriverResult<_>>()?,
    };
    let mut ids = BTreeMap::<String, String>::new();
    let mut dependencies = BTreeMap::new();
    let mut artifacts = BTreeMap::<String, PackageArtifact>::new();
    let mut references = BTreeMap::<String, ArtifactRef>::new();
    let mut missing = BTreeSet::new();

    for path in &graph.order {
        let sources = &project_sources[path];
        let deps = dependency_ids(path, &graph, &ids);
        let id = package_id(
            &context,
            path,
            sources,
            &deps,
            &generation.packages[path],
            roles[path].as_ref(),
        )?;
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

    if !missing.is_empty() {
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
                    interface: ond_compiler_core::export::package(&frontend.hir, &path)
                        .map_err(|e| vec![e])?,
                    contract: request.contract.clone(),
                    compiler_id: compiler_id.into(),
                    build_id: ids[&path].clone(),
                    dependencies: dependencies[&path].clone(),
                    imports: graph.imports[&path].clone(),
                    initializer: package
                        .initializer
                        .as_ref()
                        .map(|f| f.name.clone())
                        .or_else(|| {
                            object
                                .symbols
                                .iter()
                                .find(|symbol| {
                                    symbol.name.ends_with(".\u{1f}global.init")
                                        && symbol.kind == ond_protocol::object::SymbolKind::Function
                                })
                                .map(|symbol| symbol.name.clone())
                        }),
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
        contract: request.contract,
        compiler_id: compiler_id.into(),
        packages: references.into_values().collect(),
        program,
    };
    let mut result = CompileResult {
        manifest: manifest.clone(),
        frontend_ran: true,
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
    cache::atomic_write(&generation_cache_path, &cache::encode(&generation)?)?;
    legacy_labels(&mut result, application_root.as_deref());
    Ok(result)
}

fn legacy_labels(result: &mut CompileResult, root: Option<&str>) {
    for path in result.rebuilt.iter_mut().chain(&mut result.reused) {
        if Some(path.as_str()) == root {
            *path = ".".into();
        }
    }
    result.rebuilt.sort();
    result.reused.sort();
}

fn package_id(
    context: &(&ond_protocol::Contract, &str, String),
    path: &str,
    sources: &[(String, String)],
    deps: &BTreeMap<String, String>,
    generation: &serde_json::Value,
    roles: Option<&ond_compiler_core::roles::RoleConfiguration>,
) -> DriverResult<String> {
    let sources = sources
        .iter()
        .map(|(path, content_hash)| serde_json::json!({"path":path,"content_hash":content_hash}))
        .collect::<Vec<_>>();
    let mut input = serde_json::json!({"contract":context.0,"compiler_id":context.1,"manifest_hash":context.2,"package":path,"sources":sources,"dependencies":deps,"generation":generation});
    if let Some(roles) = roles {
        input["roles"] = serde_json::to_value(roles).map_err(|error| vec![error.to_string()])?;
    }
    Ok(cache::hash(&cache::encode(&input)?))
}

fn reuse_generation(
    cached: &GenerationCache,
    directory: &std::path::Path,
    context: &(&ond_protocol::Contract, &str, String),
    sources: &BTreeMap<String, Vec<(String, String)>>,
    graph: &Graph,
    roles: &BTreeMap<String, Option<ond_compiler_core::roles::RoleConfiguration>>,
) -> DriverResult<Option<CompileResult>> {
    let mut ids = BTreeMap::new();
    let mut references = Vec::new();
    let mut artifacts = BTreeMap::new();
    for path in &graph.order {
        let deps = dependency_ids(path, graph, &ids);
        let id = package_id(
            context,
            path,
            &sources[path],
            &deps,
            &cached.packages[path],
            roles[path].as_ref(),
        )?;
        let Some((reference, artifact)) = cache::lookup(directory, &id) else {
            return Ok(None);
        };
        if artifact.contract != *context.0
            || artifact.compiler_id != context.1
            || artifact.package != *path
            || artifact.build_id != id
            || artifact.dependencies != deps
            || artifact.imports != graph.imports[path]
        {
            return Ok(None);
        }
        ids.insert(path.clone(), id);
        references.push(reference);
        artifacts.insert(path.clone(), artifact);
    }
    references.sort_by(|a, b| a.package.cmp(&b.package));
    let manifest = CompilationManifest {
        contract: context.0.clone(),
        compiler_id: context.1.into(),
        packages: references,
        program: ProgramMetadata {
            entry_symbol: "main.main".into(),
            initializers: graph
                .initialization_order
                .iter()
                .filter_map(|path| artifacts[path].initializer.clone())
                .collect(),
        },
    };
    cache::atomic_write(
        &directory.join(format!("project-{}.ondmanifest", cached.key)),
        &cache::encode(&ProjectCache {
            key: cached.key.clone(),
            manifest: manifest.clone(),
        })?,
    )?;
    Ok(Some(CompileResult {
        manifest,
        frontend_ran: false,
        rebuilt: Vec::new(),
        reused: ids.into_keys().collect(),
    }))
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
                    if package.directory == loaded.root {
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
    context: &(&ond_protocol::Contract, &str, String),
    graph: &Graph,
) -> Option<CompileResult> {
    let cache: ProjectCache =
        cache::read_json(&directory.join(format!("project-{key}.ondmanifest"))).ok()?;
    let manifest = cache.manifest;
    if cache.key != key
        || manifest.contract != *context.0
        || manifest.compiler_id != context.1
        || manifest.packages.len() != graph.order.len()
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
