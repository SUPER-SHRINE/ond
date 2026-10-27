//! ROLE-07〜10の実行・API・公開hash・cache。期待値は現行roles caseから導く。
use super::{fixtures::*, memory_vm};
use crate::{CompileRequest, CompileResult, cache, compile};
use ond_protocol::Contract;
use std::{collections::BTreeMap, fs};
fn request(f: &ProjectFixture, selected: Option<&str>) -> CompileRequest {
    CompileRequest {
        role: selected.map(str::to_string),
        test: false,
        excluded_dirs: vec![],
        contract: Contract::default(),
        project_root: f.root.clone(),
        sources: BTreeMap::new(),
        cache_dir: None,
    }
}
fn build(f: &ProjectFixture, selected: Option<&str>) -> CompileResult {
    compile(request(f, selected), &"0".repeat(64)).unwrap()
}
fn result(f: &ProjectFixture, selected: Option<&str>) -> u64 {
    let loaded = crate::input::load(&request(f, selected)).unwrap();
    let compiled = ond_compiler_core::compile_loaded_project(loaded).unwrap();
    memory_vm::execute_with_global(&compiled.mir, "main.main", "main.Result").unwrap()[0]
}
fn id(result: &CompileResult, package: &str) -> String {
    result
        .manifest
        .packages
        .iter()
        .find(|p| p.package == package)
        .unwrap()
        .build_id
        .clone()
}
#[test]
fn role_08_09_final_edge_selection_executes_generic_definition_context() {
    for (case, manifest, selected, expected) in [
        (
            "ROLE-08.final-bool / ROLE-09.selected-api",
            RUNNER.to_string(),
            None,
            20,
        ),
        (
            "ROLE-08.final-false / ROLE-09.omitted-role",
            RUNNER
                .replace("final=true", "final=false")
                .replace(",role=\"Test\"", ""),
            None,
            10,
        ),
        (
            "ROLE-08.final-default",
            RUNNER
                .replace("final=true\n", "")
                .replace(",role=\"Test\"", ""),
            None,
            10,
        ),
        (
            "ROLE-09.string-dependency",
            "[project]\nname=\"runner\"\n[dependencies]\nchild=\"../child\"\n".into(),
            None,
            10,
        ),
        (
            "ROLE-09.roleless-default",
            RUNNER.replace(",role=\"Test\"", ""),
            None,
            10,
        ),
        (
            "ROLE-09.root-primary",
            format!("{RUNNER}[roles]\nnames=[\"Debug\",\"Release\"]\n"),
            None,
            20,
        ),
        (
            "ROLE-09.root-nonprimary",
            format!("{RUNNER}[roles]\nnames=[\"Debug\",\"Release\"]\n"),
            Some("Debug"),
            20,
        ),
    ] {
        let f = ProjectFixture::new();
        f.runner(&manifest);
        if case == "ROLE-09.roleless-default" {
            f.put("child/ond.toml", "[project]\nname=\"child\"\n");
            f.put("child/lib.ond","package child\nfunc Value[T](x:T)->i32{return 10}\nfunc CommonApi()->i32{return 1}\n");
        }
        let compiled = build(&f, selected);
        assert_eq!(result(&f, selected), expected, "{case}");
        let child = cache::read_artifact(
            compiled
                .manifest
                .packages
                .iter()
                .find(|p| p.package == "child")
                .unwrap(),
        )
        .unwrap();
        assert!(child.interface.exports.contains_key("CommonApi"));
        assert_eq!(
            child.interface.exports.contains_key("TestApi"),
            expected == 20,
            "{case}"
        );
    }
}
#[test]
fn role_02_dependency_omission_does_not_inherit_root_named_role() {
    let f = ProjectFixture::new();
    f.runner("[project]\nname=\"runner\"\n[roles]\nnames=[\"Debug\",\"Release\"]\n[dependencies]\nchild={path=\"../child\"}\n");
    f.put(
        "child/ond.toml",
        "[project]\nname=\"child\"\n[roles]\nnames=[\"Internal\",\"Public\"]\n",
    );
    f.put(
        "child/lib.ond",
        "package child\nfunc Value[T](x:T)->i32{role Internal{return 11};return 22}\n",
    );
    for selected in [None, Some("Debug"), Some("Release")] {
        build(&f, selected);
        assert_eq!(
            result(&f, selected),
            22,
            "ROLE-02.dependency-primary/independent"
        );
    }
    f.runner("[project]\nname=\"runner\"\nfinal=true\n[roles]\nnames=[\"Debug\",\"Release\"]\n[dependencies]\nchild={path=\"../child\",role=\"Internal\"}\n");
    build(&f, Some("Debug"));
    assert_eq!(
        result(&f, Some("Debug")),
        11,
        "ROLE-02.dependency-added-role"
    );
    assert!(!build(&f, Some("Debug")).frontend_ran);
    let manifest = fs::read_to_string(f.root.join("ond.toml")).unwrap();
    f.runner(&manifest.replace(",role=\"Internal\"", ""));
    build(&f, Some("Debug"));
    assert_eq!(
        result(&f, Some("Debug")),
        22,
        "ROLE-07.dependency-role-reset"
    );
}
#[test]
fn role_08_same_project_import_and_role_wrapper_private_api_execute() {
    let f = ProjectFixture::new();
    f.put(
        "runner/local/l.ond",
        "package local\nfunc Read()->i32{return 5}\n",
    );
    f.put("runner/main.ond","package main\nimport \"child\"\nimport \"runner/local\"\nvar Result:i32\nfunc main(){Result=child.Value[i32](7)+local.Read()}\n");
    build(&f, None);
    assert_eq!(result(&f, None), 25, "ROLE-08.same-project-import");
    f.put("child/lib.ond","package child\ntype Counter struct{value:i32}\nfunc privateValue()->i32{return 7}\nfunc (c:Counter) privateRead()->i32{return c.value}\nrole Test {func VerifyPrivate()->i32{var c=Counter{value:13};return privateValue()+c.privateRead()}}\n");
    f.put("runner/main.ond","package main\nimport \"child\"\nvar Result:i32\nfunc main(){Result=child.VerifyPrivate()}\n");
    build(&f, None);
    assert_eq!(
        result(&f, None),
        20,
        "ROLE-09.selected-api common private function and method through Test wrapper"
    );
}
#[test]
fn role_10_agreeing_paths_execute_once_per_shared_project_identity() {
    let f = ProjectFixture::new();
    f.diamond();
    let root = fs::read_to_string(f.root.join("ond.toml")).unwrap();
    f.runner(&root.replace(",role=\"Test\"", ""));
    let common = build(&f, None);
    assert_eq!(result(&f, None), 20, "ROLE-10.resolved-agreement");
    assert_eq!(
        common
            .manifest
            .packages
            .iter()
            .filter(|p| p.package == "child")
            .count(),
        1
    );
    f.runner(&root.replace(
        "middle={path=\"../bridge\"}",
        "middle={path=\"../child\",role=\"Test\"}",
    ));
    f.put("runner/main.ond", MAIN);
    build(&f, None);
    assert_eq!(result(&f, None), 20, "ROLE-10.named-agreement");
    f.runner("[project]\nname=\"runner\"\nfinal=true\n[dependencies]\nleft={path=\"../bridge\"}\nright={path=\"../other\"}\n");
    f.put("runner/main.ond","package main\nimport \"bridge\"\nimport \"other\"\nvar Result:i32\nfunc main(){Result=bridge.Read()+other.Read()}\n");
    f.put(
        "other/ond.toml",
        "[project]\nname=\"other\"\n[dependencies]\nleaf={path=\"../child\"}\n",
    );
    f.put(
        "other/lib.ond",
        "package other\nimport \"child\"\nfunc Read()->i32{return child.Value[i32](2)}\n",
    );
    let diamond = build(&f, None);
    assert_eq!(result(&f, None), 20, "ROLE-10.diamond-agreement");
    assert_eq!(
        diamond
            .manifest
            .packages
            .iter()
            .filter(|p| p.package == "child")
            .count(),
        1
    );
}
#[test]
fn role_07_dependency_selected_input_and_edge_reset_invalidate() {
    let f = ProjectFixture::new();
    let cold = build(&f, None);
    assert!(!build(&f, None).frontend_ran);
    assert_eq!(result(&f, None), 20);
    let raw = fs::read(f.base.join("child/lib.ond")).unwrap();
    for (selected, expected) in [(Some("Test"), 20), (None, 10)] {
        if selected.is_none() {
            f.runner(&RUNNER.replace(",role=\"Test\"", ""));
        }
        let compiled = build(&f, None);
        assert_eq!(result(&f, None), expected);
        let loaded = crate::input::load(&request(&f, None)).unwrap();
        let (_, generation) =
            ond_compiler_core::compile_loaded_project_with_generation(loaded).unwrap();
        let input = serde_json::json!({"compiler_id":"0".repeat(64),"contract":Contract::default(),"dependencies":{},"generation":serde_json::from_str::<serde_json::Value>(&generation.packages["child"]).unwrap(),"manifest_hash":cache::hash(&fs::read(f.root.join("ond.toml")).unwrap()),"mode":"normal","package":"child","roles":{"names":["Test"],"selected":selected},"sources":[{"content_hash":cache::hash(&raw),"path":"lib.ond"}]});
        assert_eq!(
            id(&compiled, "child"),
            cache::hash(&cache::encode(&input).unwrap()),
            "ROLE-07.dependency-selected-input"
        );
        assert_eq!(fs::read(f.base.join("child/lib.ond")).unwrap(), raw);
        if selected.is_none() {
            for name in ["runner", "child"] {
                assert_ne!(
                    id(&cold, name),
                    id(&compiled, name),
                    "ROLE-07.edge-role-invalidation {name}"
                );
            }
        }
    }
}
#[test]
fn role_07_current_manifest_graph_is_checked_on_warm_cache_and_after_failure() {
    for (case, runner, child, causes) in [
        (
            "ROLE-07.final-invalidation",
            RUNNER.replace("final=true", "final=false"),
            CHILD.to_string(),
            vec!["runner", "child", "role"],
        ),
        (
            "ROLE-07.target-final-invalidation",
            RUNNER.to_string(),
            CHILD.replace("name=\"child\"", "name=\"child\"\nfinal=true"),
            vec!["runner", "child", "final"],
        ),
        (
            "ROLE-07.selected-failure-invalidation",
            RUNNER.replace("role=\"Test\"", "role=\"Unknown\""),
            CHILD.to_string(),
            vec!["runner", "child", "Unknown"],
        ),
    ] {
        let f = ProjectFixture::new();
        let cold = build(&f, None);
        assert!(!build(&f, None).frontend_ran);
        let artifacts = cold
            .manifest
            .packages
            .iter()
            .map(|p| (p.path.clone(), fs::read(&p.path).unwrap()))
            .collect::<Vec<_>>();
        f.runner(&runner);
        f.put("child/ond.toml", &child);
        let error = compile(request(&f, None), &"0".repeat(64))
            .unwrap_err()
            .join("\n");
        for cause in causes {
            assert!(
                error.to_lowercase().contains(&cause.to_lowercase()),
                "{case}: {error}"
            );
        }
        for (path, bytes) in artifacts {
            assert_eq!(
                fs::read(path).unwrap(),
                bytes,
                "{case}: preserve successful artifacts"
            );
        }
        f.put("child/ond.toml", CHILD);
        f.runner(&RUNNER.replace(",role=\"Test\"", ""));
        build(&f, None);
        assert_eq!(
            result(&f, None),
            10,
            "{case}: repaired common configuration"
        );
    }
}

#[test]
fn role_07_dependency_names_change_invalidates_both_packages() {
    let f = ProjectFixture::new();
    let cold = build(&f, None);
    assert!(!build(&f, None).frontend_ran);
    f.put(
        "child/ond.toml",
        &CHILD.replace("[\"Test\"]", "[\"Test\",\"Other\"]"),
    );
    let changed = build(&f, None);
    assert!(changed.frontend_ran);
    for package in ["runner", "child"] {
        assert_ne!(id(&cold, package), id(&changed, package));
    }
    assert_eq!(result(&f, None), 20);
}

#[test]
fn role_08_09_10_resolver_checks_all_unimported_registration_edges() {
    for invalid in [
        "nonfinal-role",
        "final-target",
        "conflicting-paths",
        "final-type",
    ] {
        let f = ProjectFixture::new();
        match invalid {
            "nonfinal-role" => f.runner(&RUNNER.replace("final=true", "final=false")),
            "final-target" => f.put(
                "child/ond.toml",
                &CHILD.replace("name=\"child\"", "name=\"child\"\nfinal=true"),
            ),
            "conflicting-paths" => f.diamond(),
            "final-type" => f.runner(&RUNNER.replace("final=true", "final=\"true\"")),
            _ => unreachable!(),
        }
        f.put("runner/main.ond", NO_IMPORT);
        assert!(
            crate::resolve_dependencies(&f.root).is_err(),
            "ROLE-08/09/10 registered graph validation: {invalid}"
        );
    }
}

#[test]
fn role_10_root_override_conflicts_with_an_unimported_return_path() {
    let f = ProjectFixture::new();
    f.runner("[project]\nname=\"runner\"\n[roles]\nnames=[\"Debug\"]\n[dependencies]\nchild={path=\"../child\"}\n");
    f.put("runner/main.ond", NO_IMPORT);
    f.put(
        "child/ond.toml",
        "[project]\nname=\"child\"\n[dependencies]\nback={path=\"../runner\"}\n",
    );
    f.put("child/lib.ond", "package child\n");
    build(&f, None);
    let error = compile(request(&f, Some("Debug")), &"0".repeat(64))
        .unwrap_err()
        .join("\n");
    for cause in ["runner", "child", "back", "Debug", "role"] {
        assert!(error.contains(cause), "ROLE-10 root return path: {error}");
    }
}
