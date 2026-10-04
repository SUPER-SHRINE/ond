use super::*;
use ond_protocol::{Contract, link::LinkPlan};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ond-cli-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let f = Self(root);
        f.put("ond.toml", "");
        f.put("main.ond", "package main\nimport \"left\"\nimport \"right\"\nfunc main(){var _=left.Value()+right.Value()}");
        f.put(
            "left/left.ond",
            "package left\nimport \"leaf\"\nfunc Value()->i32{return leaf.Value()}",
        );
        f.put(
            "right/right.ond",
            "package right\nimport \"leaf\"\nfunc Value()->i32{return leaf.Value()}",
        );
        f.put("leaf/leaf.ond", "package leaf\nfunc Value()->i32{return 1}");
        f.put(
            "other/other.ond",
            "package other\nfunc Value()->i32{return 5}",
        );
        f
    }
    fn put(&self, name: &str, source: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    fn request(&self) -> CompileRequest {
        CompileRequest {
            excluded_dirs: Vec::new(),
            contract: Contract::default(),
            project_root: self.0.clone(),
            sources: BTreeMap::new(),
            libraries: BTreeMap::new(),
            cache_dir: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn diamond_cache_rebuilds_reverse_dependents_even_for_body_only_change() {
    let f = Fixture::new();
    let cold = compile(f.request(), "test-compiler").unwrap();
    assert_eq!(cold.rebuilt, [".", "leaf", "left", "other", "right"]);
    let warm = compile(f.request(), "test-compiler").unwrap();
    assert!(!warm.frontend_ran);
    assert!(warm.rebuilt.is_empty());
    assert_eq!(
        cache::encode(&cold.manifest).unwrap(),
        cache::encode(&warm.manifest).unwrap()
    );
    f.put("leaf/leaf.ond", "package leaf\nfunc Value()->i32{return 2}");
    let changed = compile(f.request(), "test-compiler").unwrap();
    assert_eq!(changed.rebuilt, [".", "leaf", "left", "right"]);
    assert_eq!(changed.reused, ["other"]);
    // Corrupt cached Object bytes: regenerate that package, with its same source build identity.
    let reference = changed
        .manifest
        .packages
        .iter()
        .find(|p| p.package == "other")
        .unwrap();
    fs::write(&reference.path, b"broken").unwrap();
    let repaired = compile(f.request(), "test-compiler").unwrap();
    assert_eq!(repaired.rebuilt, ["other"]);
    assert!(cache::read_artifact(&repaired.manifest.packages[3]).is_ok());
}
#[test]
fn compiler_manifest_and_source_set_changes_invalidate_cache() {
    let f = Fixture::new();
    compile(f.request(), "compiler-a").unwrap();
    assert_eq!(compile(f.request(), "compiler-b").unwrap().rebuilt.len(), 5);
    f.put("other/extra.ond", "package other\nconst Extra=42");
    assert_eq!(
        compile(f.request(), "compiler-b").unwrap().rebuilt,
        ["other"]
    );
    fs::rename(f.0.join("other/extra.ond"), f.0.join("other/renamed.ond")).unwrap();
    assert_eq!(
        compile(f.request(), "compiler-b").unwrap().rebuilt,
        ["other"]
    );
    fs::remove_file(f.0.join("other/renamed.ond")).unwrap();
    // Exact original source set can reuse the earlier artifact.
    assert!(
        compile(f.request(), "compiler-b")
            .unwrap()
            .rebuilt
            .is_empty()
    );
    f.put("ond.toml", "# changed project settings\n");
    assert_eq!(compile(f.request(), "compiler-b").unwrap().rebuilt.len(), 5);
}
#[test]
fn virtual_sources_conflicts_diagnostics_and_failed_build() {
    let f = Fixture::new();
    let valid = compile(f.request(), "compiler").unwrap();
    let mut request = f.request();
    request.sources.insert(
        "other/extra.ond".into(),
        "package other\nconst Extra=42".into(),
    );
    assert_eq!(
        compile(request.clone(), "compiler").unwrap().rebuilt,
        ["other"]
    );
    assert!(
        compile(request.clone(), "compiler")
            .unwrap()
            .rebuilt
            .is_empty()
    );
    request
        .sources
        .insert("../escape.ond".into(), "package escape".into());
    assert!(
        compile(request, "compiler")
            .unwrap_err()
            .join(" ")
            .contains("invalid virtual source")
    );
    let mut request = f.request();
    request
        .sources
        .insert("main.ond".into(), "different".into());
    assert!(
        compile(request, "compiler")
            .unwrap_err()
            .join(" ")
            .contains("conflicts")
    );
    let mut request = f.request();
    request.sources.insert(
        "main.ond".into(),
        fs::read_to_string(f.0.join("main.ond")).unwrap(),
    );
    assert!(compile(request, "compiler").unwrap().rebuilt.is_empty());
    f.put(
        "leaf/leaf.ond",
        "package leaf\nfunc Value()->i32{return unknown}",
    );
    let errors = compile(f.request(), "compiler").unwrap_err().join("\n");
    assert!(
        errors.contains("leaf.ond:") && errors.contains("unknown"),
        "{errors}"
    );
    for reference in &valid.manifest.packages {
        cache::read_artifact(reference).unwrap();
    }
    f.put("leaf/leaf.ond", "package leaf\nfunc Value()->i32{return 1}");
    assert!(compile(f.request(), "compiler").unwrap().rebuilt.is_empty());
    f.put(
        "leaf/leaf.ond",
        "package leaf\nimport \"left\"\nfunc Value()->i32{return left.Value()}",
    );
    assert!(
        compile(f.request(), "compiler")
            .unwrap_err()
            .join(" ")
            .contains("cyclic")
    );
}
fn request_link(manifest: ond_protocol::CompilationManifest) -> LinkRequest {
    LinkRequest {
        manifest,
        main_return_address: 0,
        layout: LinkPlan {
            read_only: None,
            memory_base: 4096,
            memory_size: 262144,
            stack_size: 65536,
            entry_symbol: "__ond.entry".into(),
            heap_base_symbol: "__ond_heap_base".into(),
            stack_bottom_symbol: "__ond_stack_bottom".into(),
        },
    }
}

#[test]
fn explicit_trap_reason_is_emitted_only_to_linked_debug_metadata() {
    let f = Fixture::new();
    f.put(
        "main.ond",
        "package main\nfunc index(values: [1]u8, at: u32) -> u8 { return values[at] }\nfunc divide(value: u32, by: u32) -> u32 { return value / by }\nfunc cast(value: f32) -> i32 { return value as i32 }\nfunc main() { trap \"player left world\" }\n",
    );
    let id = compiler_identity().unwrap();
    let compiled = compile(f.request(), &id).unwrap();
    let main_ref = compiled
        .manifest
        .packages
        .iter()
        .find(|reference| reference.package == ".")
        .unwrap();
    let artifact = cache::read_artifact(main_ref).unwrap();
    assert!(artifact.object.sections.iter().all(|section| {
        !section
            .data
            .windows("player left world".len())
            .any(|window| window == b"player left world")
    }));

    let linked = link_with_debug(request_link(compiled.manifest)).unwrap();
    assert!(!linked.image.build_id.is_empty());
    assert_eq!(linked.image.build_id, linked.debug.build_id);
    let trap = linked
        .debug
        .ranges
        .iter()
        .find(|range| range.operation == ond_protocol::debug::DebugOperation::ExplicitTrap)
        .unwrap();
    assert_eq!(trap.message.as_deref(), Some("player left world"));
    assert_eq!(trap.source.as_ref().unwrap().path, "main.ond");
    assert!(linked.debug.functions.iter().any(|function| {
        function.symbol == "main.main"
            && trap.start >= function.address
            && trap.end <= function.address + function.size
            && !function.unwind.is_empty()
    }));
    for operation in [
        ond_protocol::debug::DebugOperation::BoundsCheck,
        ond_protocol::debug::DebugOperation::DivisionByZero,
        ond_protocol::debug::DebugOperation::InvalidConversion,
        ond_protocol::debug::DebugOperation::StackOverflow,
    ] {
        assert!(
            linked
                .debug
                .ranges
                .iter()
                .any(|range| range.operation == operation && range.message.is_none()),
            "missing classified range: {operation:?}"
        );
    }
}

#[test]
fn serialized_packages_link_identically_to_full_codegen_without_runtime_initializers() {
    let f = Fixture::new();
    f.put(
        "leaf/leaf.ond",
        "package leaf\nvar Seed:i32=8\nfunc Value()->i32{var n=Seed;return n/3}",
    );
    f.put(
        "left/left.ond",
        "package left\nimport \"leaf\"\nfunc Value()->i32{return leaf.Value()}",
    );
    f.put(
        "main.ond",
        "package main\nimport \"right\"\nimport \"left\"\nfunc main(){var _=left.Value()+right.Value()}",
    );
    let id = compiler_identity().unwrap();
    let cold = compile(f.request(), &id).unwrap();
    let first = link(request_link(cold.manifest.clone())).unwrap();
    let warm = compile(f.request(), &id).unwrap();
    let second = link(request_link(warm.manifest.clone())).unwrap();
    assert_eq!(
        cache::encode(&first).unwrap(),
        cache::encode(&second).unwrap()
    );
    let full = ond_compiler_core::compile_project(&f.0).unwrap();
    let metadata = ond_compiler::program::ProgramMetadata::from_mir(&full.mir).unwrap();
    assert_eq!(cold.manifest.program.initializers, metadata.initializers);
    let mut objects = vec![ond_compiler::program::program_startup(
        &metadata.initializers,
        &metadata.entry_symbol,
        &ond_compiler::kagura_encoding::absolute_thunk(0),
    )];
    objects.extend(ond_compiler::mir_backend::codegen_objects(&full.mir).unwrap());
    let mut reference =
        ond_compiler::linker::link_objects(&objects, &request_link(cold.manifest.clone()).layout)
            .unwrap();
    reference.build_id = first.build_id.clone();
    assert_eq!(
        cache::encode(&first).unwrap(),
        cache::encode(&reference).unwrap()
    );
    let mut bad = cold.manifest.clone();
    bad.packages.retain(|p| p.package != "leaf");
    assert!(
        link(request_link(bad))
            .unwrap_err()
            .join(" ")
            .contains("missing or stale")
    );
    let mut bad = cold.manifest.clone();
    bad.contract.revision += 1;
    assert!(
        link(request_link(bad))
            .unwrap_err()
            .join(" ")
            .contains("unsupported")
    );
    let mut bad = cold.manifest.clone();
    bad.compiler_id = "old-compiler".into();
    assert!(
        link(request_link(bad))
            .unwrap_err()
            .join(" ")
            .contains("compiler identity")
    );
    f.put(
        "leaf/leaf.ond",
        "package leaf\nvar Seed:i32=9\nfunc Value()->i32{var n=Seed;return n/3}",
    );
    let updated = compile(f.request(), &id).unwrap();
    let mut mixed = cold.manifest;
    *mixed
        .packages
        .iter_mut()
        .find(|p| p.package == "leaf")
        .unwrap() = updated
        .manifest
        .packages
        .iter()
        .find(|p| p.package == "leaf")
        .unwrap()
        .clone();
    assert!(
        link(request_link(mixed))
            .unwrap_err()
            .join(" ")
            .contains("stale")
    );
}
