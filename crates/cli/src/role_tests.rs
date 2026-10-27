//! ROLE-02、06、07: 共通構成、公開hashと生成集合。
use crate::{CompileRequest, CompileResult, cache, compile};
use ond_compiler_core::mir;
use ond_protocol::{Contract, PackageArtifact};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
mod dependencies;
mod fixtures;
#[path = "../../compiler-core/src/memory_test_vm.rs"]
mod memory_vm;
struct Fixture(PathBuf, Option<String>);
impl Fixture {
    fn new(selected: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ond-role-driver-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let f = Self(root, (!selected.is_empty()).then(|| selected.into()));
        f.manifest("");
        f.put("main.ond", "package main\nfunc main(){}\n");
        f
    }
    fn put(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn manifest(&self, extra: &str) {
        self.put("ond.toml",&format!("[project]\nname=\"game\"\n[roles]\nnames=[\"Release\",\"Debug\",\"Development\"]\n{extra}"));
    }
    fn request(&self) -> CompileRequest {
        CompileRequest {
            role: self.1.clone(),
            test: false,
            excluded_dirs: vec![],
            contract: Contract::default(),
            project_root: self.0.clone(),
            sources: BTreeMap::new(),
            cache_dir: None,
        }
    }
    fn compile(&self) -> CompileResult {
        compile(self.request(), &"0".repeat(64)).unwrap()
    }
    fn artifact(&self, result: &CompileResult) -> PackageArtifact {
        cache::read_artifact(
            result
                .manifest
                .packages
                .iter()
                .find(|p| p.package == "game")
                .unwrap(),
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn empty_input(f: &Fixture, roles: Option<Value>, package: &str) -> Value {
    let mut input = json!({"compiler_id":"0".repeat(64),"contract":Contract::default(),"dependencies":{},"generation":{"constants":[],"instances":[],"interface":{"exports":{},"package":package},"nominals":[],"types":[{"Function":{"parameters":[],"returns":[]}}]},"manifest_hash":cache::hash(&fs::read(f.0.join("ond.toml")).unwrap()),"package":package,"sources":[{"content_hash":cache::hash(&fs::read(f.0.join("main.ond")).unwrap()),"path":"main.ond"}]});
    if let Some(roles) = roles {
        input["roles"] = roles;
    }
    input
}
#[test]
fn role_07_optional_role_input_and_complete_old_input() {
    let mut f = Fixture::new("");
    let mut ids = std::collections::BTreeSet::new();
    for selected in [None, Some("Debug"), Some("Release")] {
        f.1 = selected.map(str::to_string);
        let result = f.compile();
        let roles = json!({"names":["Debug","Development","Release"],"selected":selected});
        assert_eq!(
            result.manifest.packages[0].build_id,
            cache::hash(&cache::encode(&empty_input(&f, Some(roles), "game")).unwrap()),
            "ROLE-07.role-input/primary-input"
        );
        ids.insert(result.manifest.packages[0].build_id.clone());
    }
    assert_eq!(ids.len(), 3, "ROLE-07.selected-hash");
    f.1 = None;
    f.put("ond.toml", "[project]\nname = \"main\"\n");
    let result = f.compile();
    let input = cache::encode(&empty_input(&f, None, "main")).unwrap();
    assert_eq!(input.len(), 540, "ROLE-07.unconfigured-hash");
    assert_eq!(
        cache::hash(&input),
        "6ab6bd11899fd905bb33b9127f979f963d96e035338d3b3b7ba37ac57d65752a"
    );
    assert_eq!(result.manifest.packages[0].build_id, cache::hash(&input));
}
#[test]
fn role_07_configuration_raw_source_and_failure_cache_invalidate() {
    let mut f = Fixture::new("Release");
    f.put(
        "main.ond",
        "package main\nrole Debug { func f(){} }\nfunc main(){}\n",
    );
    let cold = f.compile();
    let warm = f.compile();
    assert!(!warm.frontend_ran);
    assert_eq!(
        cold.manifest.packages[0].build_id,
        warm.manifest.packages[0].build_id
    );
    f.put(
        "main.ond",
        "package main\nrole Debug { func  f(){} }\nfunc main(){}\n",
    );
    let edit = f.compile();
    assert!(edit.frontend_ran);
    assert_ne!(
        edit.manifest.packages[0].build_id,
        cold.manifest.packages[0].build_id
    );
    let manifest = fs::read_to_string(f.0.join("ond.toml")).unwrap();
    f.put(
        "ond.toml",
        &manifest.replace("\"Development\"]", "\"Development\",\"Shipping\"]"),
    );
    let configuration = f.compile();
    assert!(configuration.frontend_ran);
    assert_ne!(
        configuration.manifest.packages[0].build_id,
        edit.manifest.packages[0].build_id
    );
    f.put(
        "main.ond",
        "package main\nrole Debug {func f(){missing()}}\nfunc main(){}\n",
    );
    f.1 = Some("Debug".into());
    assert!(compile(f.request(), &"0".repeat(64)).is_err());
    f.1 = None;
    f.compile();
}
#[test]
fn role_02_generation_and_role_06_original_positions() {
    let source = "package main\nvar Result:i32\nrole Debug, Development {\n    func cheat()->i32{return 99}\n    const Payload=[3]u8{17,23,31}\n    var CheatData:i32=44\n}\nfunc probe()->i32 {\n    role Debug, Development {\n        role Debug {\n            return cheat()\n        }\n        role Development {\n            return 2\n        }\n    }\n    role Release {\n        return 3\n    }\n    return 0\n}\nfunc main(){Result=probe()}\n";
    for (selected, line, hidden) in [
        ("", 20, vec![4, 11, 14, 18]),
        ("Debug", 11, vec![14, 18]),
        ("Development", 14, vec![11, 18]),
        ("Release", 18, vec![4, 11, 14]),
    ] {
        let f = Fixture::new(selected);
        f.put("main.ond", source);
        let result = f.compile();
        let artifact = f.artifact(&result);
        let extra = matches!(selected, "Debug" | "Development");
        assert_eq!(
            artifact
                .object
                .symbols
                .iter()
                .any(|s| s.name == "main.cheat"),
            extra
        );
        assert_eq!(
            artifact
                .object
                .symbols
                .iter()
                .any(|s| s.name == "main.CheatData" && s.size == 4),
            extra
        );
        assert_eq!(artifact.interface.exports.contains_key("CheatData"), extra);
        assert_eq!(artifact.interface.exports.contains_key("Payload"), extra);
        assert!(
            artifact
                .debug
                .ranges
                .iter()
                .any(|r| r.function == "main.probe"
                    && r.source.as_ref().is_some_and(|s| s.line == line)),
            "ROLE-06.positions {selected}: {:?}",
            artifact.debug
        );
        for range in &artifact.debug.ranges {
            if let Some(location) = &range.source {
                assert_eq!(location.path, "main.ond");
                let raw_line = source.lines().nth(location.line as usize - 1).unwrap();
                assert!(
                    location.column > 0 && location.column as usize <= raw_line.len() + 1,
                    "ROLE-06 original source column: {location:?}"
                );
            }
        }
        assert!(
            artifact
                .object
                .symbols
                .iter()
                .any(|symbol| symbol.name == "main.Result" && symbol.size == 4)
        );
        assert!(
            !artifact
                .debug
                .ranges
                .iter()
                .any(|r| r.source.as_ref().is_some_and(|s| hidden.contains(&s.line))),
            "ROLE-02 nonselected debug ranges {selected}"
        );
        if !extra {
            assert!(
                !artifact
                    .object
                    .relocations
                    .iter()
                    .any(|r| r.target == "main.cheat" || r.target == "main.CheatData")
            );
            assert!(
                !artifact
                    .debug
                    .functions
                    .iter()
                    .any(|f| f.symbol == "main.cheat")
            );
        }
        let loaded = crate::input::load(&f.request()).unwrap();
        let frontend = ond_compiler_core::compile_loaded_project(loaded).unwrap();
        let expected = match selected {
            "Debug" => 99,
            "Development" => 2,
            "Release" => 3,
            _ => 0,
        };
        assert_eq!(
            memory_vm::execute(&frontend.mir, "main.probe", &[]).unwrap(),
            [expected]
        );
        assert_eq!(
            memory_vm::execute_with_global(&frontend.mir, "main.main", "main.Result").unwrap(),
            [expected],
            "ROLE-02 main Result {selected}"
        );
    }
}
