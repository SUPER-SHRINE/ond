//! 全source fileを通常のpackage入力として扱う契約。
use super::*;
use ond_compiler_core::mir;
#[path = "../../compiler-core/src/memory_test_vm.rs"]
mod memory_vm;

const MAIN: &str = "package main\nvar Result:u32\nfunc main(){var value:Device;_=value.secret;Result=TestArgument(TestResult())+TestGeneric[u32](2)+value.TestMethod()+Collect()}\n";
const TEST: &str = "package main\ntype Device struct{secret:u32}\nfunc TestArgument(x:u32)->u32{return x}\nfunc TestResult()->u32{return 1}\nfunc TestGeneric[T](x:T)->T{return x}\nfunc(x:Device) TestMethod()->u32{return x.secret+3}\nfunc privateHelper()->u32{return 4}\n";
const FEATURE: &str = "package main\nfunc Collect()->u32{return privateHelper()}\n";
fn source_project() -> Fixture {
    let f = Fixture::new();
    f.put("ond.toml", "[project]\nname = \"p\"\n");
    f.put("main.ond", MAIN);
    f.put("test.ond", TEST);
    f.put("feature_test.ond", FEATURE);
    for directory in ["left", "right", "leaf", "other"] {
        fs::remove_dir_all(f.0.join(directory)).unwrap();
    }
    f
}
#[test]
fn pkg01_source_files_have_complete_public_hash_input() {
    let f = source_project();
    let id = "0".repeat(64);
    let loaded = crate::input::load(&f.request()).unwrap();
    let mut paths = loaded
        .sources
        .files()
        .iter()
        .map(|file| file.path().file_name().unwrap().to_str().unwrap())
        .collect::<Vec<_>>();
    paths.sort_unstable();
    assert_eq!(
        paths,
        ["feature_test.ond", "main.ond", "test.ond"],
        "PKG-01.source-files"
    );
    let (_, generation) =
        ond_compiler_core::compile_loaded_project_with_generation(loaded).unwrap();
    let generation: serde_json::Value = serde_json::from_str(&generation.packages["p"]).unwrap();
    let sources = [
        ("feature_test.ond", FEATURE),
        ("main.ond", MAIN),
        ("test.ond", TEST),
    ]
    .map(
        |(path, text)| serde_json::json!({"path":path,"content_hash":cache::hash(text.as_bytes())}),
    );
    let input = serde_json::json!({"contract":{"revision":7,"target":"kagura-v1","abi":"ond-kagura-v1"},"compiler_id":id,"manifest_hash":cache::hash(b"[project]\nname = \"p\"\n"),"package":"p","sources":sources,"dependencies":{},"generation":generation});
    let result = compile(f.request(), &id).unwrap();
    assert_eq!(
        result.manifest.packages[0].build_id,
        cache::hash(&cache::encode(&input).unwrap()),
        "PKG-01.source-files"
    );
}
#[test]
fn pkg04_source_file_scope_executes_private_helpers_and_fields() {
    let f = source_project();
    let compilation = ond_compiler_core::compile_project(&f.0).expect("PKG-04.source-file-scope");
    assert_eq!(
        memory_vm::execute_with_global(&compilation.mir, "main.main", "main.Result").unwrap(),
        [10]
    );
}
#[test]
fn syn10_test_prefix_accepts_ordinary_function_signatures() {
    let f = source_project();
    let compilation =
        ond_compiler_core::compile_project(&f.0).expect("SYN-10.function-name-prefix");
    assert_eq!(
        memory_vm::execute(&compilation.mir, "main.TestArgument", &[7]).unwrap(),
        [7]
    );
    assert_eq!(
        memory_vm::execute(&compilation.mir, "main.TestResult", &[]).unwrap(),
        [1]
    );
    assert_eq!(
        memory_vm::execute_with_global(&compilation.mir, "main.main", "main.Result").unwrap(),
        [10]
    );
}
#[test]
fn ordinary_sources_compile_link_without_test_metadata_or_generated_wrappers() {
    let f = source_project();
    let result = compile(f.request(), &compiler_identity().unwrap()).unwrap();
    assert!(
        serde_json::to_value(&result.manifest)
            .unwrap()
            .get("tests")
            .is_none()
    );
    for reference in &result.manifest.packages {
        let artifact = cache::read_artifact(reference).unwrap();
        assert!(
            serde_json::to_value(&artifact)
                .unwrap()
                .get("tests")
                .is_none()
        );
        assert!(
            !artifact
                .object
                .symbols
                .iter()
                .any(|symbol| symbol.name.starts_with("__ond.test."))
        );
        if artifact.package == "p" {
            assert!(artifact.interface.exports.contains_key("TestArgument"));
            assert!(artifact.interface.exports.contains_key("TestResult"));
            assert!(!artifact.interface.exports.contains_key("privateHelper"));
        }
    }
    let linked = link_with_debug(request_link(result.manifest)).unwrap();
    assert!(
        !linked
            .image
            .symbols
            .keys()
            .any(|symbol| symbol.starts_with("__ond.test."))
    );
    assert!(!linked.debug.ranges.iter().any(|range| {
        range
            .message
            .as_deref()
            .is_some_and(|message| message.contains("select a test case"))
    }));
    assert!(!f.0.join(".ond/__test_entry__").exists());
}
#[test]
fn suffix_sources_in_dependencies_export_ordinary_api_and_preserve_private_scope() {
    let f = source_project();
    f.put(
        "ond.toml",
        "[project]\nname=\"p\"\n[dependencies]\nlib={path=\"library\"}\n",
    );
    f.put("library/ond.toml", "[project]\nname=\"lib\"\n");
    f.put("library/test.ond", "package lib\nfunc private()->u32{return 7}\nfunc TestValue(x:u32)->u32{return private()+x}\nfunc TestId[T](x:T)->T{return x}\n");
    f.put(
        "library/extra_test.ond",
        "package lib\nfunc Value()->u32{return TestId[u32](TestValue(2))}\n",
    );
    f.put(
        "main.ond",
        "package main\nimport \"lib\"\nvar Result:u32\nfunc main(){Result=lib.Value()}\n",
    );
    let result = compile(f.request(), &compiler_identity().unwrap()).unwrap();
    let artifact = cache::read_artifact(
        result
            .manifest
            .packages
            .iter()
            .find(|p| p.package == "lib")
            .unwrap(),
    )
    .unwrap();
    assert!(artifact.interface.exports.contains_key("TestValue"));
    assert!(artifact.interface.exports.contains_key("Value"));
    assert!(!artifact.interface.exports.contains_key("private"));
    link_with_debug(request_link(result.manifest)).unwrap();
    let compilation =
        ond_compiler_core::compile_loaded_project(crate::input::load(&f.request()).unwrap())
            .unwrap();
    assert_eq!(
        memory_vm::execute_with_global(&compilation.mir, "main.main", "main.Result").unwrap(),
        [9]
    );
    f.put(
        "main.ond",
        "package main\nimport \"lib\"\nfunc main(){_=lib.private()}\n",
    );
    assert!(compile(f.request(), "compiler").is_err());
}
#[test]
fn suffix_source_edits_renames_and_failure_recovery_invalidate_caches() {
    let f = source_project();
    let id = compiler_identity().unwrap();
    let cold = compile(f.request(), &id).unwrap();
    assert!(!compile(f.request(), &id).unwrap().frontend_ran);
    f.put("test.ond", &TEST.replace("return 4", "return 5"));
    let edited = compile(f.request(), &id).unwrap();
    assert!(edited.frontend_ran);
    assert_ne!(
        cold.manifest.packages[0].build_id,
        edited.manifest.packages[0].build_id
    );
    fs::rename(f.0.join("feature_test.ond"), f.0.join("feature.ond")).unwrap();
    let renamed = compile(f.request(), &id).unwrap();
    assert!(renamed.frontend_ran);
    assert_ne!(
        edited.manifest.packages[0].build_id,
        renamed.manifest.packages[0].build_id
    );
    let saved = fs::read(&renamed.manifest.packages[0].path).unwrap();
    f.put("test.ond", &TEST.replace("return 4", "return missing"));
    assert!(compile(f.request(), &id).is_err());
    assert_eq!(fs::read(&renamed.manifest.packages[0].path).unwrap(), saved);
    f.put("test.ond", TEST);
    compile(f.request(), &id).unwrap();
    fs::remove_file(f.0.join("test.ond")).unwrap();
    assert!(compile(f.request(), &id).is_err());
}
#[test]
fn suffix_files_require_the_same_entry_and_syntax_contract() {
    let f = source_project();
    f.put("bad_test.ond", "not Ond source");
    assert!(compile(f.request(), "compiler").is_err());
    fs::remove_file(f.0.join("bad_test.ond")).unwrap();
    fs::rename(f.0.join("main.ond"), f.0.join("entry.ond")).unwrap();
    assert!(
        compile(f.request(), "compiler")
            .unwrap_err()
            .join(" ")
            .contains("main.ond")
    );
}

#[test]
fn obsolete_cached_artifact_is_regenerated_but_explicit_old_revision_is_rejected() {
    let f = Fixture::new();
    let id = compiler_identity().unwrap();
    let cold = compile(f.request(), &id).unwrap();
    let reference = &cold.manifest.packages[0];
    let mut artifact = cache::read_artifact(reference).unwrap();
    artifact.contract.revision = 6;
    // cache索引の参照先も旧形式へ対応づけ、checksumだけの拒否と区別する。
    let old = cache::publish(reference.path.parent().unwrap(), &artifact).unwrap();
    for entry in fs::read_dir(reference.path.parent().unwrap()).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "ondmanifest")
        {
            let mut index: serde_json::Value =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            if let Some(packages) = index["manifest"]["packages"].as_array_mut() {
                for stored in packages {
                    if stored["package"] == reference.package {
                        *stored = serde_json::to_value(&old).unwrap();
                    }
                }
                fs::write(path, cache::encode(&index).unwrap()).unwrap();
            }
        }
    }
    let recovered = compile(f.request(), &id).unwrap();
    assert!(recovered.frontend_ran);
    assert!(recovered.rebuilt.contains(&".".into()));
    let mut explicit = recovered.manifest;
    explicit.contract.revision = 6;
    assert!(link_with_debug(request_link(explicit)).is_err());
}
