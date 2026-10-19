use crate::{
    ExternalSource, ExternalSources, SemanticCache, compile_loaded_project, compile_project,
    load_library_sources, load_project, load_project_with_inputs, load_project_with_test_inputs,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ond-project-contract-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ond.toml"),
            format!("[project]\nname = \"{name}\"\n"),
        )
        .unwrap();
        Self(root)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn application_imports_its_qualified_subpackage_only() {
    let fixture = Fixture::new("game");
    fixture.write(
        "math/math.ond",
        "package math\nfunc Add() -> i32 { return 42 }\n",
    );
    fixture.write(
        "main.ond",
        "package main\nimport \"game/math\"\nfunc main() { _ = math.Add() }\n",
    );
    let compilation = compile_project(fixture.root()).unwrap();
    assert_eq!(
        compilation
            .loaded
            .packages
            .iter()
            .map(|package| package.logical_path.as_str())
            .collect::<Vec<_>>(),
        vec![".", "game/math"]
    );

    fixture.write(
        "main.ond",
        "package main\nimport \"math\"\nfunc main() { _ = math.Add() }\n",
    );
    let error = compile_project(fixture.root()).unwrap_err();
    assert!(error.to_string().contains("unknown import path `math`"));
}

#[test]
fn every_manifest_needs_a_valid_ond_project_name() {
    let fixture = Fixture::new("game");
    for (manifest, message) in [
        ("", "project must declare `[project] name`"),
        ("[project]\n", "missing field `name`"),
        ("[project]\nname = \"bad-name\"\n", "invalid project name"),
        ("[project]\nname = \"func\"\n", "invalid project name"),
    ] {
        fixture.write("ond.toml", manifest);
        let error = load_project(fixture.root()).unwrap_err();
        assert!(error.to_string().contains(message), "{manifest}: {error}");
    }
}

#[test]
fn named_library_without_main_can_be_validated_for_editing() {
    let fixture = Fixture::new("shared");
    fixture.write(
        "api.ond",
        "package shared\nfunc Value() -> i32 { return 42 }\n",
    );
    fixture.write("collections/list.ond", "package collections\n");
    let loaded = load_project(fixture.root()).unwrap();
    assert_eq!(
        loaded
            .packages
            .iter()
            .map(|package| package.logical_path.as_str())
            .collect::<Vec<_>>(),
        vec!["shared", "shared/collections"]
    );
    SemanticCache::default().validate(&loaded).unwrap();
    assert!(compile_project(fixture.root()).is_err());
}

#[test]
fn required_generated_sources_use_the_project_namespace() {
    let fixture = Fixture::new("game");
    fixture.write(
        "main.ond",
        "package main\nimport \"game/api\"\nfunc main() { _ = api.Value() }\n",
    );
    let required = BTreeMap::from([(
        "api/generated.ond".to_string(),
        "package api\nfunc Value() -> i32 { return 42 }\n".to_string(),
    )]);
    let loaded = load_project_with_inputs(
        fixture.root(),
        &BTreeMap::new(),
        &[],
        &required,
        &BTreeMap::new(),
        &ExternalSources::new(),
    )
    .unwrap();
    assert!(loaded.packages.iter().any(|p| p.logical_path == "game/api"));
    compile_loaded_project(loaded).unwrap();
}

#[test]
fn external_logical_paths_coexist_and_real_collisions_are_rejected() {
    let fixture = Fixture::new("game");
    fixture.write("math/math.ond", "package math\nfunc Own() {}\n");
    fixture.write("math/math_test.ond", "package math\nfunc TestOwn() {}\n");
    fixture.write(
        "main.ond",
        "package main\nimport own \"game/math\"\nimport extra \"math\"\nfunc main() { own.Own(); extra.Other() }\n",
    );
    let external = ExternalSources::from([(
        "math/lib.ond".to_string(),
        ExternalSource {
            path: fixture.root().join("external/lib.ond"),
            text: "package math\nfunc Other() {}\n".to_string(),
        },
    )]);
    let loaded = load_project_with_inputs(
        fixture.root(),
        &BTreeMap::new(),
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &external,
    )
    .unwrap();
    assert!(loaded.packages.iter().any(|p| p.logical_path == "math"));
    assert!(
        loaded
            .packages
            .iter()
            .any(|p| p.logical_path == "game/math")
    );
    let paths = loaded
        .sources
        .files()
        .iter()
        .map(|file| {
            (
                file.path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                file.display_path().unwrap().to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(paths["math.ond"], "math/math.ond");
    assert_eq!(paths["lib.ond"], "math/lib.ond");
    compile_loaded_project(loaded).unwrap();

    let conflicting = ExternalSources::from([(
        "game/math/math.ond".to_string(),
        ExternalSource {
            path: fixture.root().join("external/math.ond"),
            text: "package math\nfunc Different() {}\n".to_string(),
        },
    )]);
    let error = load_project_with_inputs(
        fixture.root(),
        &BTreeMap::new(),
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &conflicting,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("library conflicts with project package: game/math")
    );

    let identical = ExternalSources::from([(
        "game/math/math.ond".to_string(),
        ExternalSource {
            path: fixture.root().join("external/math.ond"),
            text: "package math\nfunc Own() {}\n".to_string(),
        },
    )]);
    load_project_with_test_inputs(
        fixture.root(),
        &BTreeMap::new(),
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &identical,
    )
    .unwrap();
}

#[test]
fn optional_files_with_the_same_relative_name_keep_separate_package_identity() {
    for physical in [false, true] {
        for external_text in [
            "package math\nfunc Value() {}\n",
            "package math\nfunc Other() {}\n",
        ] {
            let fixture = Fixture::new("game");
            fixture.write("main.ond", "package main\nimport own \"game/math\"\nimport extra \"math\"\nfunc main() { own.Value() }\n");
            let own_text = "package math\nfunc Value() {}\n";
            let required = if physical {
                fixture.write("math/value.ond", own_text);
                BTreeMap::new()
            } else {
                BTreeMap::from([("math/value.ond".into(), own_text.into())])
            };
            let optional = BTreeMap::from([("math/value.ond".into(), external_text.into())]);
            let loaded = load_project_with_inputs(
                fixture.root(),
                &BTreeMap::new(),
                &[],
                &required,
                &optional,
                &ExternalSources::new(),
            )
            .unwrap();
            for (package, text) in [("game/math", own_text), ("math", external_text)] {
                let package = loaded
                    .packages
                    .iter()
                    .find(|p| p.logical_path == package)
                    .unwrap();
                assert_eq!(package.files.len(), 1);
                let file = loaded.sources.file(package.files[0]);
                assert_eq!(file.text(), text);
                assert_eq!(file.display_path(), Some("math/value.ond"));
            }
            compile_loaded_project(loaded).unwrap();
        }
    }
}

#[test]
fn optional_root_library_packages_require_an_identical_complete_file_set() {
    let fixture = Fixture::new("shared");
    let own = "package shared\nfunc Own() {}\n";
    fixture.write("api.ond", own);
    fixture.write("api_test.ond", "package shared\nfunc TestOwn() { Own() }\n");
    for files in [
        BTreeMap::from([(
            "shared/extra.ond".into(),
            "package shared\nfunc extra() {}\n".into(),
        )]),
        BTreeMap::from([(
            "shared/api.ond".into(),
            "package shared\nfunc Other() {}\n".into(),
        )]),
        BTreeMap::from([
            ("shared/api.ond".into(), own.into()),
            ("shared/extra.ond".into(), "package shared\n".into()),
        ]),
        BTreeMap::from([("shared/api.ond".into(), own.into())]),
    ] {
        for file_backed in [false, true] {
            let external = files
                .iter()
                .map(|(name, text): (&String, &String)| {
                    (
                        name.clone(),
                        ExternalSource {
                            path: fixture.root().join("vendor").join(name),
                            text: text.clone(),
                        },
                    )
                })
                .collect();
            let empty = BTreeMap::new();
            let no_external = ExternalSources::new();
            let result = load_project_with_test_inputs(
                fixture.root(),
                &BTreeMap::new(),
                &[],
                &BTreeMap::new(),
                if file_backed { &empty } else { &files },
                if file_backed { &external } else { &no_external },
            );
            if files == BTreeMap::from([("shared/api.ond".into(), own.into())]) {
                let loaded = result.unwrap();
                let package = loaded
                    .packages
                    .iter()
                    .find(|p| p.logical_path == "shared")
                    .unwrap();
                assert_eq!(package.files.len(), 2, "own test file must be retained");
            } else {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("library conflicts with project package: shared")
                );
            }
        }
    }
}

#[test]
fn file_backed_root_library_package_cannot_mix_with_project_files() {
    let fixture = Fixture::new("shared");
    let own = "package shared\nfunc Own() {}\n";
    fixture.write("api.ond", own);
    fixture.write("api_test.ond", "package shared\nfunc TestOwn() { Own() }\n");
    let sdk = fixture.root().join("target/sdk");
    fixture.write(
        "target/sdk/shared/extra.ond",
        "package shared\nfunc extra() {}\n",
    );
    let external =
        load_library_sources(fixture.root(), std::slice::from_ref(&sdk), &BTreeMap::new()).unwrap();
    let error = load_project_with_test_inputs(
        fixture.root(),
        &BTreeMap::new(),
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &external,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("library conflicts with project package: shared")
    );
    fs::remove_file(sdk.join("shared/extra.ond")).unwrap();
    fixture.write("target/sdk/shared/api.ond", own);
    let external = load_library_sources(fixture.root(), &[sdk], &BTreeMap::new()).unwrap();
    assert_eq!(external.len(), 1);
    let loaded = load_project_with_test_inputs(
        fixture.root(),
        &BTreeMap::new(),
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
        &external,
    )
    .unwrap();
    assert_eq!(loaded.packages[0].files.len(), 2);
}
