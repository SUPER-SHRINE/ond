use crate::{DriverResult, cache, graph::Graph};
use crate::{LinkRequest, LinkResult};
use ond_protocol::link::LinkedImage;
use std::collections::BTreeMap;

pub fn link(request: LinkRequest) -> DriverResult<LinkedImage> {
    Ok(link_with_debug(request)?.image)
}

pub fn link_with_debug(request: LinkRequest) -> DriverResult<LinkResult> {
    link_impl(request, None).map(|(result, _)| result)
}

pub fn link_executable(
    manifest: ond_protocol::CompilationManifest,
    settings: &crate::settings::LinkSettings,
) -> DriverResult<(LinkResult, Vec<u8>)> {
    let request = LinkRequest {
        manifest,
        layout: settings.layout()?,
        main_return_address: match settings.exit {
            crate::settings::Exit::Loop => 0,
            crate::settings::Exit::Jump { address } => address,
        },
    };
    let (result, bytes) = link_impl(request, Some(settings))?;
    Ok((result, bytes.expect("executable link")))
}

fn link_impl(
    request: LinkRequest,
    settings: Option<&crate::settings::LinkSettings>,
) -> DriverResult<(LinkResult, Option<Vec<u8>>)> {
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
        if let Some(initializer) = &artifact.initializer
            && !artifact.object.symbols.iter().any(|symbol| {
                symbol.name == *initializer
                    && symbol.binding == ond_protocol::object::SymbolBinding::Exported
                    && symbol.kind == ond_protocol::object::SymbolKind::Function
                    && symbol.section == Some(ond_protocol::object::SectionKind::Text)
            })
        {
            return Err(vec![format!(
                "initializer is not owned by package {}",
                artifact.package
            )]);
        }
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
    let application_roots = artifacts
        .iter()
        .filter(|(_, artifact)| {
            artifact.object.symbols.iter().any(|symbol| {
                symbol.name == "main.main"
                    && symbol.binding == ond_protocol::object::SymbolBinding::Exported
                    && symbol.kind == ond_protocol::object::SymbolKind::Function
                    && symbol.section == Some(ond_protocol::object::SectionKind::Text)
            })
        })
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    if application_roots.len() > 1
        || request.manifest.tests.is_none() && application_roots.len() != 1
    {
        return Err(vec!["normal build requires one application root".into()]);
    }
    let application_root = application_roots.first().copied();
    let graph = Graph::from_imports_with_root(
        artifacts
            .iter()
            .map(|(p, a)| (p.clone(), a.imports.clone()))
            .collect(),
        application_root,
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
    let mut expected_tests = Vec::new();
    for artifact in artifacts.values() {
        if artifact.tests.is_some() != request.manifest.tests.is_some() {
            return Err(vec![
                "manifest test metadata does not match package build mode".into(),
            ]);
        }
        if let Some(tests) = &artifact.tests {
            let mut seen = std::collections::BTreeSet::new();
            for case in &tests.cases {
                if case.package != artifact.package
                    || !seen.insert((&case.package, &case.name, &case.symbol))
                {
                    return Err(vec![format!(
                        "invalid test case ownership or duplicate in package {}",
                        artifact.package
                    )]);
                }
            }
            expected_tests.extend(tests.cases.clone());
        }
    }
    if let Some(tests) = &request.manifest.tests {
        let key = |case: &ond_protocol::TestCase| {
            (case.package.clone(), case.name.clone(), case.symbol.clone())
        };
        let expected = expected_tests
            .iter()
            .map(key)
            .collect::<std::collections::BTreeSet<_>>();
        let actual = tests
            .cases
            .iter()
            .map(key)
            .collect::<std::collections::BTreeSet<_>>();
        if actual != expected || actual.len() != tests.cases.len() {
            return Err(vec![
                "manifest test cases do not match compiled package test cases".into(),
            ]);
        }
    }
    let mut debug_objects = request
        .manifest
        .packages
        .iter()
        .map(|reference| artifacts[&reference.package].debug.clone())
        .collect::<Vec<_>>();
    let termination = if settings.is_some_and(|s| matches!(s.exit, crate::settings::Exit::Loop)) {
        vec![ond_compiler::kagura_encoding::word(12, 0, 0, 0, -1)]
    } else {
        ond_compiler::kagura_encoding::absolute_thunk(request.main_return_address).to_vec()
    };
    let mut wrappers = Vec::new();
    if let Some(tests) = &request.manifest.tests {
        let mut seen = std::collections::BTreeSet::new();
        for (index, case) in tests.cases.iter().enumerate() {
            let expected_symbol = format!(
                "{}.{}",
                if application_root == Some(case.package.as_str()) {
                    "main"
                } else {
                    &case.package
                },
                case.name
            );
            let Some(artifact) = artifacts.get(&case.package) else {
                return Err(vec![format!("unknown test package: {}", case.package)]);
            };
            if !case.name.starts_with("Test")
                || case.symbol != expected_symbol
                || !seen.insert((&case.package, &case.name))
            {
                return Err(vec![format!(
                    "invalid or duplicate test metadata: {}.{}",
                    case.package, case.name
                )]);
            }
            if !artifact.object.symbols.iter().any(|symbol| {
                symbol.name == case.symbol
                    && symbol.binding == ond_protocol::object::SymbolBinding::Exported
                    && symbol.kind == ond_protocol::object::SymbolKind::Function
                    && symbol.section == Some(ond_protocol::object::SectionKind::Text)
            }) {
                return Err(vec![format!(
                    "test function symbol is not defined by its package: {}",
                    case.symbol
                )]);
            }
            let initializers = graph
                .initialization_order_for(&case.package)?
                .iter()
                .filter_map(|p| artifacts[p].initializer.clone())
                .collect();
            wrappers.push((
                format!("__ond.test.{index}"),
                initializers,
                case.symbol.clone(),
            ));
        }
    }
    let startup = if request.manifest.tests.is_some() {
        let object = ond_compiler::program::test_startup(&wrappers, &termination);
        debug_objects.insert(
            0,
            ond_protocol::debug::DebugObject {
                package: object.name.clone(),
                functions: Vec::new(),
                ranges: vec![ond_protocol::debug::ObjectDebugRange {
                    section: ond_protocol::object::SectionKind::Text,
                    start: 0,
                    end: 4,
                    operation: ond_protocol::debug::DebugOperation::ExplicitTrap,
                    message: Some(
                        "select a test case entry point from the .ondtest sidecar".into(),
                    ),
                    source: None,
                    function: "__ond.entry".into(),
                }],
            },
        );
        object
    } else {
        let object = ond_compiler::program::program_startup(&expected, "main.main", &termination);
        debug_objects.insert(
            0,
            ond_protocol::debug::DebugObject {
                package: object.name.clone(),
                ..Default::default()
            },
        );
        object
    };
    let mut objects = vec![startup];
    objects.extend(request.manifest.packages.iter().map(|reference| {
        artifacts
            .remove(&reference.package)
            .expect("validated artifact")
            .object
    }));
    let generated = ond_compiler::mir_backend::runtime::supply_with_debug(objects, debug_objects)
        .map_err(|e| vec![e.to_string()])?;
    if generated.objects.len() != generated.debug.len() {
        return Err(vec!["generated object/debug ownership mismatch".into()]);
    }
    let pairs = generated
        .objects
        .iter()
        .zip(&generated.debug)
        .collect::<Vec<_>>();
    let (mut linked, bytes) = if let Some(settings) = settings {
        let kinds = generated
            .objects
            .iter()
            .flat_map(|o| &o.sections)
            .filter(|s| s.memory_size != 0)
            .map(|s| s.kind)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let boot = settings.startup == crate::settings::Startup::SelfStarting;
        let prefix_size = if boot {
            ond_compiler::linker::self_boot_prefix(&kinds, 0).len() as u32
        } else {
            0
        };
        let (linked, executable) = ond_compiler::linker::link_executable_pairs_with_debug(
            &pairs,
            &request.layout,
            prefix_size,
        )
        .map_err(|e| vec![e.to_string()])?;
        let mut bytes = if boot {
            ond_compiler::linker::self_boot_prefix(&kinds, prefix_size)
        } else {
            Vec::new()
        };
        bytes.extend(executable.write().map_err(|e| vec![e])?);
        (linked, Some(bytes))
    } else {
        (
            ond_compiler::linker::link_object_pairs_with_debug(&pairs, &request.layout)
                .map_err(|e| vec![e.to_string()])?,
            None,
        )
    };
    let build_id = if let Some(bytes) = &bytes {
        cache::hash(bytes)
    } else {
        cache::hash(&cache::encode(&linked.image)?)
    };
    linked.image.build_id = build_id.clone();
    linked.debug.build_id = build_id.clone();
    let tests = request
        .manifest
        .tests
        .as_ref()
        .map(|tests| ond_protocol::TestSidecar {
            build_id,
            cases: tests
                .cases
                .iter()
                .zip(&wrappers)
                .map(|(case, (entry, _, _))| ond_protocol::TestEntry {
                    package: case.package.clone(),
                    name: case.name.clone(),
                    entry_point: linked.image.symbols[entry],
                })
                .collect(),
        });
    Ok((
        LinkResult {
            tests,
            image: linked.image,
            debug: linked.debug,
        },
        bytes,
    ))
}
