use super::*;
use ond_protocol::{Contract, link::LinkPlan};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
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
        Self::new_in(&root)
    }
    fn new_in(root: &Path) -> Self {
        fs::create_dir(root).unwrap();
        let f = Self(root.to_path_buf());
        f.put("ond.toml", "[project]\nname = \"test\"\n");
        f.put("main.ond", "package main\nimport \"test/left\"\nimport \"test/right\"\nfunc main(){var _=left.Value()+right.Value()}");
        f.put(
            "left/left.ond",
            "package left\nimport \"test/leaf\"\nfunc Value()->i32{return leaf.Value()}",
        );
        f.put(
            "right/right.ond",
            "package right\nimport \"test/leaf\"\nfunc Value()->i32{return leaf.Value()}",
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
            role: None,
            excluded_dirs: Vec::new(),
            contract: Contract::default(),
            project_root: self.0.clone(),
            sources: BTreeMap::new(),
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
    assert_eq!(
        cold.rebuilt,
        [".", "test/leaf", "test/left", "test/other", "test/right"]
    );
    let warm = compile(f.request(), "test-compiler").unwrap();
    assert!(!warm.frontend_ran);
    assert!(warm.rebuilt.is_empty());
    assert_eq!(
        cache::encode(&cold.manifest).unwrap(),
        cache::encode(&warm.manifest).unwrap()
    );
    f.put("leaf/leaf.ond", "package leaf\nfunc Value()->i32{return 2}");
    let changed = compile(f.request(), "test-compiler").unwrap();
    assert_eq!(
        changed.rebuilt,
        [".", "test/leaf", "test/left", "test/right"]
    );
    assert_eq!(changed.reused, ["test/other"]);
    // Corrupt cached Object bytes: regenerate that package, with its same source build identity.
    let reference = changed
        .manifest
        .packages
        .iter()
        .find(|p| p.package == "test/other")
        .unwrap();
    fs::write(&reference.path, b"broken").unwrap();
    let repaired = compile(f.request(), "test-compiler").unwrap();
    assert_eq!(repaired.rebuilt, ["test/other"]);
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
        ["test/other"]
    );
    fs::rename(f.0.join("other/extra.ond"), f.0.join("other/renamed.ond")).unwrap();
    assert_eq!(
        compile(f.request(), "compiler-b").unwrap().rebuilt,
        ["test/other"]
    );
    fs::remove_file(f.0.join("other/renamed.ond")).unwrap();
    // Exact original source set can reuse the earlier artifact.
    assert!(
        compile(f.request(), "compiler-b")
            .unwrap()
            .rebuilt
            .is_empty()
    );
    f.put(
        "ond.toml",
        "[project]\nname = \"test\"\n# changed project settings\n",
    );
    assert_eq!(compile(f.request(), "compiler-b").unwrap().rebuilt.len(), 5);
}

#[test]
fn generic_specializations_are_part_of_cache_identity() {
    let f = Fixture::new();
    f.put(
        "main.ond",
        "package main\nimport \"test/shared\"\nimport \"test/types\"\nfunc main(){\nvar model: types.Model\nvar number: i32 = shared.Outer[i32](1)\n}",
    );
    f.put(
        "shared/shared.ond",
        "package shared\nfunc Inner[T](value:T)->T{return value}\nfunc Outer[T](value:T)->T{return Inner[T](value)}",
    );
    f.put("types/types.ond", "package types\ntype Model struct{}\n");

    let id = compiler_identity().unwrap();
    let cold = compile(f.request(), &id).unwrap();
    link(request_link(cold.manifest.clone())).unwrap();
    f.put(
        "main.ond",
        "package main\nimport \"test/shared\"\nimport \"test/types\"\nfunc main(){\nvar model: types.Model\nvar number: i32 = shared.Outer[i32](1)\nvar again: i32 = shared.Outer[i32](2)\n}\n// caller edit without a new specialization\n",
    );
    let same_specializations = compile(f.request(), &id).unwrap();
    assert_eq!(same_specializations.rebuilt, ["."]);
    assert!(same_specializations.reused.contains(&"test/shared".into()));
    let original_shared = cold
        .manifest
        .packages
        .iter()
        .find(|package| package.package == "test/shared")
        .unwrap();
    assert_eq!(
        original_shared.build_id,
        same_specializations
            .manifest
            .packages
            .iter()
            .find(|package| package.package == "test/shared")
            .unwrap()
            .build_id
    );

    f.put(
        "main.ond",
        "package main\nimport \"test/shared\"\nimport \"test/types\"\nfunc main(){\nvar model: types.Model\nvar number: i32 = shared.Outer[i32](1)\nvar copy: types.Model = shared.Outer[types.Model](model)\n}",
    );
    let changed = compile(f.request(), &id).unwrap();
    let old_shared = cold
        .manifest
        .packages
        .iter()
        .find(|package| package.package == "test/shared")
        .unwrap();
    let new_shared = changed
        .manifest
        .packages
        .iter()
        .find(|package| package.package == "test/shared")
        .unwrap();
    assert_ne!(old_shared.build_id, new_shared.build_id);
    link(request_link(changed.manifest.clone())).unwrap();
    let mut mixed = changed.manifest.clone();
    *mixed
        .packages
        .iter_mut()
        .find(|package| package.package == "test/shared")
        .unwrap() = old_shared.clone();
    assert!(
        link(request_link(mixed))
            .unwrap_err()
            .join(" ")
            .contains("stale")
    );

    let warm = compile(f.request(), &id).unwrap();
    assert!(warm.rebuilt.is_empty());
    assert!(!warm.frontend_ran);
    let warm_image = link(request_link(warm.manifest)).unwrap();

    f.put("aaa/aaa.ond", "package aaa\ntype Added struct{}\n");
    let shifted = compile(f.request(), &id).unwrap();
    assert!(shifted.reused.contains(&"test/shared".into()));
    let shifted_image = link(request_link(shifted.manifest)).unwrap();

    let mut fresh_request = f.request();
    fresh_request.cache_dir = Some(f.0.join("fresh-cache"));
    let fresh = compile(fresh_request, &id).unwrap();
    let fresh_image = link(request_link(fresh.manifest)).unwrap();
    assert_eq!(
        cache::encode(&shifted_image).unwrap(),
        cache::encode(&fresh_image).unwrap()
    );
    assert_eq!(
        cache::encode(&warm_image).unwrap(),
        cache::encode(&link(request_link(changed.manifest)).unwrap()).unwrap()
    );
}

#[test]
fn nested_generic_cache_tracks_caller_type_layout_and_signatureless_arguments() {
    let f = Fixture::new();
    f.put(
        "main.ond",
        r#"package main
import "test/shared"
import "test/inner"
type Token struct { Value:u32 }
type Box[T] struct { Value:T }
func main() {
    var token: Token
    var box: Box[u32]
    _ = shared.Outer[Token](token)
    _ = shared.Outer[Box[u32]](box)
    _ = inner.Zero[i32]()
}
"#,
    );
    f.put(
        "shared/shared.ond",
        "package shared\nimport \"test/inner\"\nfunc Outer[T](value:T)->T{return inner.Identity[T](value)}\n",
    );
    f.put(
        "inner/inner.ond",
        "package inner\nfunc Identity[T](value:T)->T{return value}\nfunc Zero[T]()->i32{return 0}\n",
    );
    let id = compiler_identity().unwrap();
    let cold = compile(f.request(), &id).unwrap();
    link(request_link(cold.manifest.clone())).unwrap();

    f.put(
        "main.ond",
        r#"package main
import "test/shared"
import "test/inner"
type Token struct { Value:u32; Next:*Token }
type Box[T] struct { Value:T }
func main() {
    var token: Token
    var box: Box[u32]
    _ = shared.Outer[Token](token)
    _ = shared.Outer[Box[u32]](box)
    _ = inner.Zero[i32]()
}
"#,
    );
    let token_changed = compile(f.request(), &id).unwrap();
    for owner in ["test/shared", "test/inner"] {
        let before = cold
            .manifest
            .packages
            .iter()
            .find(|package| package.package == owner)
            .unwrap();
        let after = token_changed
            .manifest
            .packages
            .iter()
            .find(|package| package.package == owner)
            .unwrap();
        assert_ne!(before.build_id, after.build_id, "{owner}");
        assert!(token_changed.rebuilt.contains(&owner.to_owned()));
    }
    link(request_link(token_changed.manifest.clone())).unwrap();

    f.put(
        "main.ond",
        r#"package main
import "test/shared"
import "test/inner"
type Token struct { Value:u32; Next:*Token }
type Box[T] struct { Value:T; Extra:u32 }
func main() {
    var token: Token
    var box: Box[u32]
    _ = shared.Outer[Token](token)
    _ = shared.Outer[Box[u32]](box)
    _ = inner.Zero[i32]()
}
"#,
    );
    let layout_changed = compile(f.request(), &id).unwrap();
    for owner in ["test/shared", "test/inner"] {
        let before = token_changed
            .manifest
            .packages
            .iter()
            .find(|package| package.package == owner)
            .unwrap();
        let after = layout_changed
            .manifest
            .packages
            .iter()
            .find(|package| package.package == owner)
            .unwrap();
        assert_ne!(before.build_id, after.build_id, "{owner}");
        assert!(layout_changed.rebuilt.contains(&owner.to_owned()));
    }
    link(request_link(layout_changed.manifest.clone())).unwrap();

    f.put(
        "main.ond",
        r#"package main
import "test/shared"
import "test/inner"
type Token struct { Value:u32; Next:*Token }
type Box[T] struct { Value:T; Extra:u32 }
type Added struct { Value:u32 }
func main() {
    var token: Token
    var box: Box[u32]
    var added: Added
    _ = shared.Outer[Token](token)
    _ = shared.Outer[Box[u32]](box)
    _ = shared.Outer[Added](added)
    _ = inner.Zero[i32]()
    _ = inner.Zero[u32]()
}
"#,
    );
    let new_specializations = compile(f.request(), &id).unwrap();
    for owner in ["test/shared", "test/inner"] {
        assert!(new_specializations.rebuilt.contains(&owner.to_owned()));
    }
    link(request_link(new_specializations.manifest.clone())).unwrap();
    let warm = compile(f.request(), &id).unwrap();
    assert!(!warm.frontend_ran);
    assert!(warm.rebuilt.is_empty());
    link(request_link(warm.manifest)).unwrap();

    let mut fresh_request = f.request();
    fresh_request.cache_dir = Some(f.0.join("fresh-cache"));
    let fresh = compile(fresh_request, &id).unwrap();
    let fresh_image = link(request_link(fresh.manifest)).unwrap();
    let mixed_image = link(request_link(new_specializations.manifest)).unwrap();
    assert_eq!(
        cache::encode(&mixed_image).unwrap(),
        cache::encode(&fresh_image).unwrap()
    );
}

#[test]
fn generic_cache_tracks_folded_layout_query_results() {
    let f = Fixture::new();
    f.put(
        "shared/shared.ond",
        "package shared\nimport \"test/inner\"\nfunc Size[T]()->u32{return sizeof(T)}\nfunc Outer[T]()->u32{return inner.Align[T]()}\n",
    );
    f.put(
        "inner/inner.ond",
        "package inner\nfunc Align[T]()->u32{return alignof(T)}\n",
    );
    let id = compiler_identity().unwrap();
    let source = |token: &str, extra: &str| {
        format!(
            r#"package main
import "test/shared"
type Token struct {{ Value:{token} }}
type Box[T] struct {{ Value:T; {extra} }}
func main() {{
    _ = shared.Size[Token]()
    _ = shared.Outer[Token]()
    _ = shared.Size[Box[Token]]()
    _ = shared.Outer[Box[Token]]()
}}
"#
        )
    };
    f.put("main.ond", &source("u8", ""));
    let mut before = compile(f.request(), &id).unwrap();
    link(request_link(before.manifest.clone())).unwrap();
    for (index, (token, extra)) in [("u32", ""), ("u32", "Extra:u32")].into_iter().enumerate() {
        f.put("main.ond", &source(token, extra));
        let changed = compile(f.request(), &id).unwrap();
        // The second edit changes size alone; the alignment-only inner owner can be reused.
        let owners: &[&str] = if index == 0 {
            &["test/shared", "test/inner"]
        } else {
            &["test/shared"]
        };
        for owner in owners {
            let old = before
                .manifest
                .packages
                .iter()
                .find(|package| package.package == *owner)
                .unwrap();
            let new = changed
                .manifest
                .packages
                .iter()
                .find(|package| package.package == *owner)
                .unwrap();
            assert_ne!(old.build_id, new.build_id, "{owner}");
            assert!(changed.rebuilt.contains(&(*owner).to_owned()));
        }
        let changed_image = link(request_link(changed.manifest.clone())).unwrap();
        let warm = compile(f.request(), &id).unwrap();
        assert!(!warm.frontend_ran);
        assert!(warm.rebuilt.is_empty());
        assert_eq!(
            cache::encode(&changed_image).unwrap(),
            cache::encode(&link(request_link(warm.manifest)).unwrap()).unwrap()
        );
        let mut fresh_request = f.request();
        fresh_request.cache_dir = Some(f.0.join(format!("fresh-layout-{index}")));
        let fresh = compile(fresh_request, &id).unwrap();
        assert_eq!(
            cache::encode(&changed_image).unwrap(),
            cache::encode(&link(request_link(fresh.manifest)).unwrap()).unwrap()
        );
        before = changed;
    }
}

#[test]
fn corrupt_project_cache_manifest_reuses_valid_package_objects() {
    let f = Fixture::new();
    let id = compiler_identity().unwrap();
    let cold = compile(f.request(), &id).unwrap();
    let cache_dir = f.0.join("target/ond-cache");
    let manifest = fs::read_dir(&cache_dir)
        .unwrap()
        .map(Result::unwrap)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("project-") && name.ends_with(".ondmanifest"))
        })
        .unwrap();
    fs::write(manifest, b"broken").unwrap();

    let recovered = compile(f.request(), &id).unwrap();
    assert!(!recovered.frontend_ran);
    assert!(recovered.rebuilt.is_empty());
    assert_eq!(recovered.reused.len(), cold.manifest.packages.len());
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
        ["test/other"]
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
        "package leaf\nimport \"test/left\"\nfunc Value()->i32{return left.Value()}",
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
        .find(|reference| reference.package == "test")
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
        "package left\nimport \"test/leaf\"\nfunc Value()->i32{return leaf.Value()}",
    );
    f.put(
        "main.ond",
        "package main\nimport \"test/right\"\nimport \"test/left\"\nfunc main(){var _=left.Value()+right.Value()}",
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
    bad.packages.retain(|p| p.package != "test/leaf");
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
        .find(|p| p.package == "test/leaf")
        .unwrap() = updated
        .manifest
        .packages
        .iter()
        .find(|p| p.package == "test/leaf")
        .unwrap()
        .clone();
    assert!(
        link(request_link(mixed))
            .unwrap_err()
            .join(" ")
            .contains("stale")
    );
}

#[test]
fn cached_generic_owner_links_rebuilt_caller_after_type_id_shift() {
    let f = Fixture::new();
    let main = r#"package main
import "test/api"
func main() {
    cell := api.Cell[api.Token]{}
    _ = cell.Get()
    var reader = api.Reader[api.Token]{Cell:&cell}
    _ = reader.Get()
    _ = len(cell)
    _ = api.Plain{} + cell
}
"#;
    f.put("main.ond", main);
    f.put(
        "api/api.ond",
        r#"package api
type Token struct { Value:u32 }
type Plain struct {}
type Cell[T] struct { Value:T }
type Reader[T] struct { Cell:*Cell[T] }
func (reader:Reader[T]) Get()->T { return reader.Cell.Get() }
func (cell: *Cell[T]) Get()->T { return cell.Value }
operator (cell: Cell[T]) len()->u32 { return 1 }
operator (plain: Plain) + (cell: Cell[Token])->u32 { return cell.Value.Value }
"#,
    );
    let id = compiler_identity().unwrap();
    let before = compile(f.request(), &id).unwrap();
    link(request_link(before.manifest.clone())).unwrap();
    let owner = before
        .manifest
        .packages
        .iter()
        .find(|p| p.package == "test/api")
        .unwrap();
    f.put(
        "aaa/extra.ond",
        "package aaa\ntype Extra struct { Value:u32 }",
    );
    f.put("main.ond", &format!("{main}\n// rebuild caller\n"));
    let after = compile(f.request(), &id).unwrap();
    assert!(after.reused.iter().any(|p| p == "test/api"));
    assert!(after.rebuilt.iter().any(|p| p == "."));
    assert_eq!(
        owner.path,
        after
            .manifest
            .packages
            .iter()
            .find(|p| p.package == "test/api")
            .unwrap()
            .path
    );
    let mixed = link(request_link(after.manifest)).unwrap();
    let mut fresh_request = f.request();
    fresh_request.cache_dir = Some(f.0.join("fresh-cache"));
    let fresh = compile(fresh_request, &id).unwrap();
    let cold = link(request_link(fresh.manifest)).unwrap();
    assert_eq!(
        cache::encode(&mixed).unwrap(),
        cache::encode(&cold).unwrap()
    );
}

#[path = "test_build_tests.rs"]
mod test_build;

#[test]
fn linker_rejects_application_root_imports_from_unreachable_packages() {
    let f = Fixture::new();
    let mut manifest = compile(f.request(), &compiler_identity().unwrap())
        .unwrap()
        .manifest;
    let root = manifest
        .packages
        .iter()
        .find(|p| p.package == "test")
        .unwrap()
        .clone();
    let reference = manifest
        .packages
        .iter_mut()
        .find(|p| p.package == "test/other")
        .unwrap();
    let mut artifact = cache::read_artifact(reference).unwrap();
    artifact.imports.push(root.package.clone());
    artifact
        .dependencies
        .insert(root.package.clone(), root.build_id.clone());
    *reference = cache::publish(&f.0.join("altered"), &artifact).unwrap();
    assert!(link(request_link(manifest)).is_err());
    let imports = BTreeMap::from([
        ("lib".into(), vec![]),
        ("lib/other".into(), vec!["lib".into()]),
    ]);
    crate::graph::Graph::from_imports_with_root(imports, None)
        .expect("library roots remain importable");
}

#[test]
fn relative_project_requests_keep_ignore_paths_canonical() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let f = Fixture::new_in(&cwd.join(format!(
        "ond-cli-relative-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    f.put(
        "ond.toml",
        "[project]\nname=\"test\"\n[resolve]\nignore=[\"generated\"]\n",
    );
    f.put("generated/broken.ond", "invalid source");
    let mut request = f.request();
    request.project_root = f.0.strip_prefix(&cwd).unwrap().to_path_buf();
    compile(request, &compiler_identity().unwrap())
        .expect("relative root uses the same absolute ignore boundary");
}

#[test]
fn inline_asm_generic_artifacts_link_and_cache_body_changes() {
    let f = Fixture::new();
    f.put("main.ond", "package main\nimport \"test/shared\"\nfunc main(){var value=shared.Increment[u32](1);_=value}\n");
    let shared = |immediate| {
        format!(
            "package shared\nfunc Increment[T](value:T)->T{{asm(r1<->value){{ADD r1,r1,r0,{immediate}}};return value}}\n"
        )
    };
    f.put("shared/shared.ond", &shared(1));
    let identity = compiler_identity().unwrap();
    let cold = compile(f.request(), &identity).unwrap();
    let cold_image = link(request_link(cold.manifest)).unwrap();
    let warm = compile(f.request(), &identity).unwrap();
    assert!(!warm.frontend_ran);
    assert!(warm.rebuilt.is_empty());
    assert_eq!(
        cache::encode(&cold_image).unwrap(),
        cache::encode(&link(request_link(warm.manifest)).unwrap()).unwrap()
    );
    f.put("shared/shared.ond", &shared(2));
    let changed = compile(f.request(), &identity).unwrap();
    assert!(changed.rebuilt.contains(&"test/shared".to_owned()));
    let changed_image = link(request_link(changed.manifest)).unwrap();
    assert_ne!(
        cache::encode(&cold_image).unwrap(),
        cache::encode(&changed_image).unwrap()
    );
    let mut request = f.request();
    request.cache_dir = Some(f.0.join("fresh-asm-cache"));
    let fresh = compile(request, &identity).unwrap();
    assert_eq!(
        cache::encode(&changed_image).unwrap(),
        cache::encode(&link(request_link(fresh.manifest)).unwrap()).unwrap()
    );
}
