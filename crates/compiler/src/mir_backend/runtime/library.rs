//! Load once per request batch; compute dependency closure before instruction emission.
use super::*;
use ond_compiler_core::{LoadedProject, PackageId, PackageSources, source::SourceDb};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
pub(super) fn load(source: &str) -> Result<mir::Project, FrontendError> {
    let root = PathBuf::from("<ond-runtime>");
    let mut sources = SourceDb::default();
    let file = sources.add_file(root.join("runtime.ond"), source.into());
    // The frontend currently accepts complete projects, not library-only inputs.
    // This synthetic host is discarded before dependency selection/object emission.
    let host = sources.add_file(
        root.join("main.ond"),
        "package main\nfunc main() {}\n".into(),
    );
    let loaded = LoadedProject {
        root: root.clone(),
        manifest_path: root.join("ond.toml"),
        ignored_paths: Vec::new(),
        sources,
        packages: vec![
            PackageSources {
                id: PackageId(0),
                logical_path: ".".into(),
                directory: root.clone(),
                files: vec![host],
            },
            PackageSources {
                id: PackageId(1),
                logical_path: "runtime".into(),
                directory: root,
                files: vec![file],
            },
        ],
    };
    let mut project = ond_compiler_core::compile_loaded_project(loaded)
        .map_err(|e| failure(format!("frontend failed in runtime_support.ond: {e}")))?
        .mir;
    project.packages.retain(|p| p.logical_path == "runtime");
    project.initialization_order.clear();
    if project
        .packages
        .iter()
        .any(|p| !p.globals.is_empty() || p.initializer.is_some())
    {
        return Err(failure(
            "helper library must not have global storage or startup side effects",
        ));
    }
    // The existing library expresses its non-returning invalid-conversion leaf via
    // division by zero. Give this compiler-owned primitive an explicit MIR trap so
    // soft-float does not depend on integer division (or recursively on itself).
    for f in &mut project.packages[0].functions {
        if f.name == "runtime.__sf_invalid" {
            f.locals.clear();
            f.values.clear();
            f.entry = mir::BlockId(0);
            f.blocks = vec![mir::BasicBlock {
                id: mir::BlockId(0),
                parameters: vec![],
                instructions: vec![],
                terminator: mir::Terminator {
                    kind: mir::TerminatorKind::Trap(mir::TrapKind::DivisionByZero),
                    span: f.span,
                },
            }];
        }
    }
    Ok(project)
}
pub(super) fn select(project: &mut mir::Project, roots: &[String]) -> Result<(), FrontendError> {
    let functions = &project.packages[0].functions;
    let by_name = functions
        .iter()
        .map(|f| (f.name.clone(), f))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut pending = roots.to_vec();
    while let Some(name) = pending.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        let f = by_name
            .get(&name)
            .ok_or_else(|| failure(format!("missing dependency {name}")))?;
        for i in f.blocks.iter().flat_map(|b| &b.instructions) {
            match &i.kind {
                mir::InstructionKind::Call {
                    callee: mir::Callee::Direct(name),
                    ..
                }
                | mir::InstructionKind::Constant(mir::Constant::Function {
                    symbol: name, ..
                }) => pending.push(name.clone()),
                _ => {}
            }
        }
    }
    let names = seen
        .iter()
        .map(|name| {
            (
                name.clone(),
                format!("{PREFIX}{}", name.strip_prefix("runtime.").unwrap_or(name)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let package = &mut project.packages[0];
    package.functions.retain(|f| seen.contains(&f.name));
    package.logical_path = format!("{PREFIX}library");
    for f in &mut package.functions {
        f.name = names[&f.name].clone();
        for i in f.blocks.iter_mut().flat_map(|b| &mut b.instructions) {
            match &mut i.kind {
                mir::InstructionKind::Call {
                    callee: mir::Callee::Direct(name),
                    ..
                }
                | mir::InstructionKind::Constant(mir::Constant::Function {
                    symbol: name, ..
                }) => *name = names[name].clone(),
                _ => {}
            }
        }
    }
    mir::validate_project(project)
        .map_err(|e| failure(format!("invalid selected helper MIR: {e:?}")))
}
