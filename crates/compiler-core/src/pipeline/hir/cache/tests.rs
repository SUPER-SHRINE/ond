use super::*;

fn project(files: &[(&str, &str)]) -> LoadedProject {
    let root = std::env::temp_dir().join("ond-semantic-cache-tests");
    let mut loaded = LoadedProject {
        manifest_path: root.join("ond.toml"),
        root: root.clone(),
        ignored_paths: Vec::new(),
        sources: SourceDb::default(),
        packages: Vec::new(),
    };
    let mut packages = BTreeMap::<String, Vec<FileId>>::new();
    for (name, text) in files {
        let path = std::path::Path::new(name);
        let package = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| ".".into());
        packages
            .entry(package)
            .or_default()
            .push(loaded.sources.add_file(root.join(path), (*text).into()));
    }
    for (logical_path, files) in packages {
        loaded.packages.push(PackageSources {
            id: PackageId(loaded.packages.len()),
            directory: root.join(&logical_path),
            logical_path,
            files,
        });
    }
    loaded
}

fn changed(cache: &SemanticCache) -> Vec<(&str, bool)> {
    cache
        .recomputed
        .iter()
        .map(|(name, test)| (name.as_str(), *test))
        .collect()
}

#[test]
fn warm_edits_recompute_only_the_affected_source_group() {
    let files = [
        ("main.ond", "package main\nfunc main() {}\n"),
        ("a/api.ond", "package a\nfunc private() {}\n"),
        ("a/test.ond", "package a\nfunc TestOK() { private() }\n"),
        ("b/api.ond", "package b\nfunc Other() {}\n"),
        ("b/test.ond", "package b\nfunc TestOther() {}\n"),
    ];
    let mut cache = SemanticCache::default();
    cache.validate(&project(&files)).unwrap();
    assert_eq!(
        changed(&cache),
        [
            (".", false),
            ("a", false),
            ("a", true),
            ("b", false),
            ("b", true)
        ]
    );
    let (test_hir, _) = cache.entries[&("a".into(), true)].result.as_ref().unwrap();
    let functions = test_hir
        .packages
        .iter()
        .flat_map(|package| &package.items)
        .filter_map(|item| match item {
            hir::Item::Func(function) => Some(function.name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        functions,
        ["TestOK"],
        "ordinary bodies must not be lowered again for a test entry"
    );
    cache.validate(&project(&files)).unwrap();
    assert!(changed(&cache).is_empty());
    let mut edited = files;
    edited[0].1 = "package main\nfunc main() {}\n\n";
    cache.validate(&project(&edited)).unwrap();
    assert_eq!(changed(&cache), [(".", false)]);
    edited[2].1 = "package a\nfunc TestOK() { private(); private() }\n";
    cache.validate(&project(&edited)).unwrap();
    assert_eq!(changed(&cache), [("a", true)]);
    edited[1].1 = "package a\nfunc private() {}\n\n";
    cache.validate(&project(&edited)).unwrap();
    assert_eq!(changed(&cache), [("a", false), ("a", true)]);
}

#[test]
fn source_membership_import_changes_and_parse_recovery_have_content_keys() {
    let mut cache = SemanticCache::default();
    let missing = project(&[(
        "main.ond",
        "package main\nimport \"dep\"\nfunc main() { dep.Run() }\n",
    )]);
    assert!(cache.validate(&missing).is_err());
    let complete = project(&[
        (
            "main.ond",
            "package main\nimport \"dep\"\nfunc main() { dep.Run() }\n",
        ),
        ("dep/api.ond", "package dep\nfunc Run() {}\n"),
    ]);
    cache.validate(&complete).unwrap();
    assert_eq!(changed(&cache), [(".", false), ("dep", false)]);
    let renamed = project(&[
        ("dep/renamed.ond", "package dep\nfunc Run() {}\n"),
        (
            "main.ond",
            "package main\nimport \"dep\"\nfunc main() { dep.Run() }\n",
        ),
    ]);
    cache.validate(&renamed).unwrap();
    assert_eq!(changed(&cache), [(".", false), ("dep", false)]);
    let removed_import = project(&[
        ("main.ond", "package main\nfunc main() {}\n"),
        ("dep/renamed.ond", "package dep\nfunc Run() {}\n"),
    ]);
    cache.validate(&removed_import).unwrap();
    assert_eq!(changed(&cache), [(".", false)]);
    assert!(
        cache
            .validate(&project(&[("main.ond", "package main\nfunc main(")]))
            .is_err()
    );
    cache.validate(&removed_import).unwrap();
    assert!(changed(&cache).is_empty());
}

#[test]
fn test_entries_reuse_private_normal_declarations_and_preserve_main_requirement() {
    let valid = project(&[
        (
            "main.ond",
            "package main\nfunc main() {}\nconst private: u32 = 2\ntype box struct { value: u32 }\nfunc (value: box) read() -> u32 { return value.value }\nfunc normal[T](value: T) -> T { return value }\n",
        ),
        (
            "test.ond",
            "package main\nfunc TestOK() { value := box{value: private}; _ = normal[u32](value.read()) }\n",
        ),
    ]);
    SemanticCache::default().validate(&valid).unwrap();
    crate::compile_loaded_project(valid).unwrap();
    for files in [
        vec![
            ("main.ond", "package main\n"),
            ("test.ond", "package main\nfunc main() {}\n"),
        ],
        vec![("test.ond", "package main\nfunc TestOnly() {}\n")],
        vec![("dep/api.ond", "package dep\nfunc Run() {}\n")],
    ] {
        let loaded = project(&files);
        assert!(SemanticCache::default().validate(&loaded).is_err());
        assert!(crate::compile_loaded_project(loaded).is_err());
    }
}

#[test]
fn dependency_constants_types_and_generic_bodies_invalidate_consumers() {
    let mut files = [
        (
            "main.ond",
            "package main\nimport \"client\"\nfunc main() { client.Use() }\n",
        ),
        (
            "dep/api.ond",
            "package dep\nconst Value = 1\ntype Number u32\nfunc Identity[T](value: T) -> T { return value }\n",
        ),
        ("dep/test.ond", "package dep\nfunc TestDep() {}\n"),
        (
            "client/api.ond",
            "package client\nimport \"dep\"\nfunc Use() { var value: dep.Number; _ = dep.Identity[dep.Number](value); _ = dep.Value }\n",
        ),
        (
            "client/test.ond",
            "package client\nimport \"dep\"\nfunc TestUse() { _ = dep.Value }\n",
        ),
    ];
    let mut cache = SemanticCache::default();
    cache.validate(&project(&files)).unwrap();
    files[2].1 = "package dep\nfunc TestDep() {}\nfunc OnlyTest() {}\n";
    cache.validate(&project(&files)).unwrap();
    assert_eq!(changed(&cache), [("dep", true)]);
    for dependency in [
        "package dep\nconst Value = 2\ntype Number u32\nfunc Identity[T](value: T) -> T { return value }\n",
        "package dep\nconst Value = 2\ntype Number u16\nfunc Identity[T](value: T) -> T { return value }\n",
        "package dep\nconst Value = 2\ntype Number u16\nfunc Identity[T](value: T) -> T { result := value; return result }\n",
    ] {
        files[1].1 = dependency;
        cache.validate(&project(&files)).unwrap();
        assert_eq!(
            changed(&cache),
            [
                (".", false),
                ("client", false),
                ("client", true),
                ("dep", false),
                ("dep", true)
            ]
        );
    }
}

#[test]
fn normal_errors_do_not_resolve_against_new_test_declarations() {
    let mut cache = SemanticCache::default();
    let normal = "package main\nfunc main() { helper() }\n";
    let first = project(&[("main.ond", normal)]);
    assert!(cache.validate(&first).is_err());
    let second = project(&[
        ("main.ond", normal),
        ("test.ond", "package main\nfunc helper() {}\n"),
    ]);
    let error = cache.validate(&second).unwrap_err();
    assert_eq!(changed(&cache), [(".", true)]);
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("helper"))
    );
    assert!(crate::compile_loaded_project(second).is_err());
}

#[test]
fn generic_definitions_keep_definition_site_visibility_when_called_from_tests() {
    for normal in [
        "package main\nfunc main() {}\nfunc Normal[T](value: T) { testHelper() }\n",
        "package main\nfunc main() {}\nfunc Normal[T](value: T) { var x: TestType; _ = x }\n",
    ] {
        let loaded = project(&[
            ("main.ond", normal),
            (
                "test.ond",
                "package main\ntype TestType u32\nfunc testHelper() {}\nfunc TestCall() { Normal[u32](1) }\n",
            ),
        ]);
        assert!(SemanticCache::default().validate(&loaded).is_err());
        assert!(crate::compile_loaded_project(loaded).is_err());
    }
}

#[test]
fn diagnostic_labels_rebind_after_file_ids_change_and_clear_on_recovery() {
    let invalid = "package main\nfunc Invalid[T](value: T) -> T { return value + value }\nfunc main() { _ = Invalid[bool](true) }\n";
    let mut cache = SemanticCache::default();
    let first = project(&[("main.ond", invalid)]);
    let error = cache.validate(&first).unwrap_err();
    assert!(
        error
            .diagnostics()
            .iter()
            .any(|diagnostic| !diagnostic.related.is_empty())
    );
    let shifted = project(&[
        ("a/api.ond", "package a\nfunc Unrelated() {}\n"),
        ("main.ond", invalid),
    ]);
    let error = cache.validate(&shifted).unwrap_err();
    assert_eq!(changed(&cache), [("a", false)]);
    for diagnostic in error.diagnostics() {
        for span in std::iter::once(diagnostic.primary.span)
            .chain(diagnostic.related.iter().map(|label| label.span))
        {
            if !span.is_synthetic() {
                assert_eq!(
                    shifted.sources.file(span.file).path(),
                    shifted.root.join("main.ond")
                );
            }
        }
    }
    let fixed = project(&[("main.ond", "package main\nfunc main() {}\n")]);
    cache.validate(&fixed).unwrap();
    assert_eq!(changed(&cache), [(".", false)]);
    assert_eq!(cache.entries.len(), 1);
}

#[test]
fn test_visibility_covers_imported_types_constants_methods_and_generics() {
    for body in [
        "_ = dep.TestValue",
        "var value: dep.TestType; _ = value",
        "dep.OnlyGeneric[u32](1)",
        "value := dep.Value{}; value.OnlyMethod()",
    ] {
        let main = format!("package main\nimport \"dep\"\nfunc TestLeak() {{ {body} }}\n");
        let loaded = project(&[
            ("main.ond", "package main\nfunc main() {}\n"),
            ("test.ond", &main),
            ("dep/api.ond", "package dep\ntype Value struct {}\n"),
            (
                "dep/test.ond",
                "package dep\nconst TestValue = 1\ntype TestType u32\nfunc OnlyGeneric[T](x: T) {}\nfunc (x: Value) OnlyMethod() {}\n",
            ),
        ]);
        assert!(
            SemanticCache::default().validate(&loaded).is_err(),
            "{body}"
        );
        assert!(crate::compile_loaded_project(loaded).is_err(), "{body}");
    }
}

#[test]
fn import_alias_conflicts_include_unselected_visible_declarations() {
    for declaration in [
        "const dep = 1",
        "var dep: u32 = 1",
        "type dep u32",
        "func dep() {}",
    ] {
        let normal = format!("package main\nfunc main() {{}}\n{declaration}\n");
        let mut files = [
            ("main.ond", normal.as_str()),
            ("test.ond", "package main\nfunc TestOK() {}\n"),
            ("dep/api.ond", "package dep\nfunc Run() {}\n"),
        ];
        let mut cache = SemanticCache::default();
        cache.validate(&project(&files)).unwrap();
        files[1].1 = "package main\nimport \"dep\"\nfunc TestOK() {}\n";
        let loaded = project(&files);
        let warm = cache.validate(&loaded).unwrap_err();
        assert_eq!(changed(&cache), [(".", true)]);
        let cold = crate::compile_loaded_project(loaded.clone()).unwrap_err();
        assert_eq!(warm.diagnostics(), cold.diagnostics(), "{declaration}");
        assert_eq!(warm.diagnostics().len(), 1);
        assert!(
            warm.diagnostics()[0]
                .message
                .contains("top-level declaration conflicts with imported package `dep`")
        );
        let fresh = SemanticCache::default().validate(&loaded).unwrap_err();
        assert_eq!(fresh.diagnostics(), cold.diagnostics());
    }
    // A test-only name stays hidden from an ordinary file's import environment.
    let loaded = project(&[
        (
            "main.ond",
            "package main\nimport \"dep\"\nfunc main() { dep.Run() }\n",
        ),
        (
            "test.ond",
            "package main\nconst dep = 1\nfunc TestOK() { _ = dep }\n",
        ),
        ("dep/api.ond", "package dep\nfunc Run() {}\n"),
    ]);
    SemanticCache::default().validate(&loaded).unwrap();
    crate::compile_loaded_project(loaded).unwrap();
}

#[test]
fn whole_project_test_import_cycles_are_checked_on_cache_hits_and_recovery() {
    let mut files = [
        ("main.ond", "package main\nfunc main() {}\n"),
        ("a/api.ond", "package a\nfunc A() {}\n"),
        (
            "a/test.ond",
            "package a\nimport \"b\"\nfunc TestA() { b.B() }\n",
        ),
        ("b/api.ond", "package b\nfunc B() {}\n"),
        ("b/test.ond", "package b\nfunc TestB() {}\n"),
    ];
    let mut cache = SemanticCache::default();
    cache.validate(&project(&files)).unwrap();
    files[4].1 = "package b\nimport \"a\"\nfunc TestB() { a.A() }\n";
    let loaded = project(&files);
    let warm = cache.validate(&loaded).unwrap_err();
    assert_eq!(changed(&cache), [("b", true)]);
    let cold = crate::compile_loaded_project(loaded.clone()).unwrap_err();
    assert_eq!(warm.diagnostics(), cold.diagnostics());
    assert_eq!(warm.diagnostics().len(), 1);
    assert_eq!(
        warm.diagnostics()[0].message,
        "import cycle detected: a -> b -> a"
    );
    assert_eq!(
        cache.validate(&loaded).unwrap_err().diagnostics(),
        cold.diagnostics()
    );
    assert!(changed(&cache).is_empty());
    assert_eq!(
        SemanticCache::default()
            .validate(&loaded)
            .unwrap_err()
            .diagnostics(),
        cold.diagnostics()
    );
    files[4].1 = "package b\nfunc TestB() {}\n";
    cache.validate(&project(&files)).unwrap();
    assert_eq!(changed(&cache), [("b", true)]);
}
