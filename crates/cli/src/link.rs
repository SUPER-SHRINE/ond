use crate::{DriverResult, cache, graph::Graph};
use crate::{LinkRequest, LinkResult};
use ond_protocol::link::LinkedImage;
use std::collections::BTreeMap;

pub fn link(request: LinkRequest) -> DriverResult<LinkedImage> {
    Ok(link_with_debug(request)?.image)
}

pub fn link_with_debug(request: LinkRequest) -> DriverResult<LinkResult> {
    request.manifest.contract.validate().map_err(|e| vec![e])?;
    if request.manifest.compiler_id != crate::compiler_identity()? {
        return Err(vec![
            "manifest compiler identity does not match this compiler; recompile first".into(),
        ]);
    }
    if !request.main_return_address.is_multiple_of(4) {
        return Err(vec!["main return address must be word-aligned".into()]);
    }
    if request.layout.entry_symbol != "__ond.entry"
        || request.layout.heap_base_symbol != "__ond_heap_base"
        || request.layout.stack_bottom_symbol != "__ond_stack_bottom"
    {
        return Err(vec!["link layout does not use Ond startup symbols".into()]);
    }
    let mut artifacts = BTreeMap::new();
    for reference in &request.manifest.packages {
        let artifact = cache::read_artifact(reference)?;
        if artifact.compiler_id != request.manifest.compiler_id {
            return Err(vec![format!(
                "mixed compiler identities: {}",
                artifact.package
            )]);
        }
        if artifacts
            .insert(artifact.package.clone(), artifact)
            .is_some()
        {
            return Err(vec![format!("duplicate package: {}", reference.package)]);
        }
    }
    for artifact in artifacts.values() {
        if artifact
            .imports
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            != artifact.dependencies.keys().collect()
        {
            return Err(vec![format!("inconsistent imports: {}", artifact.package)]);
        }
        for (path, id) in &artifact.dependencies {
            if artifacts.get(path).is_none_or(|a| &a.build_id != id) {
                return Err(vec![format!(
                    "missing or stale dependency: {} -> {path}",
                    artifact.package
                )]);
            }
        }
    }
    let graph = Graph::from_imports(
        artifacts
            .iter()
            .map(|(p, a)| (p.clone(), a.imports.clone()))
            .collect(),
    )?;
    let expected: Vec<_> = graph
        .initialization_order
        .iter()
        .filter_map(|p| artifacts[p].initializer.clone())
        .collect();
    if request.manifest.program.entry_symbol != "main.main"
        || request.manifest.program.initializers != expected
    {
        return Err(vec![
            "program startup metadata does not match package graph".into(),
        ]);
    }
    let debug_objects = artifacts
        .values()
        .map(|artifact| artifact.debug.clone())
        .collect::<Vec<_>>();
    let mut objects = vec![ond_compiler::program::program_startup(
        &expected,
        "main.main",
        &ond_compiler::kagura_encoding::absolute_thunk(request.main_return_address),
    )];
    objects.extend(artifacts.into_values().map(|a| a.object));
    let generated = ond_compiler::mir_backend::runtime::supply_with_debug(objects, debug_objects)
        .map_err(|e| vec![e.to_string()])?;
    let mut linked = ond_compiler::linker::link_objects_with_debug(
        &generated.objects,
        &generated.debug,
        &request.layout,
    )
    .map_err(|e| vec![e.to_string()])?;
    let build_id = cache::hash(&cache::encode(&linked.image)?);
    linked.image.build_id = build_id.clone();
    linked.debug.build_id = build_id;
    Ok(LinkResult {
        image: linked.image,
        debug: linked.debug,
    })
}
