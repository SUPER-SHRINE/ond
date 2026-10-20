use super::*;

fn test_request(f: &Fixture) -> CompileRequest {
    let mut request = f.request();
    request.test = true;
    request
}

#[test]
fn library_test_build_initializes_globals_with_and_without_cases() {
    for (name, cases) in [("test", false), ("test", true)] {
        let f = Fixture::new();
        f.put(
            "library/ond.toml",
            &format!("[project]\nname = \"{name}\"\n"),
        );
        f.put(
            "library/api.ond",
            &format!("package {name}\nvar Value:i32=1\n"),
        );
        if cases {
            f.put(
                "library/api_test.ond",
                &format!("package {name}\nfunc TestRoot(){{_=Value}}\n"),
            );
        }
        let mut request = test_request(&f);
        request.project_root = f.0.join("library");
        let manifest = compile(request, &compiler_identity().unwrap())
            .unwrap()
            .manifest;
        link_with_debug(request_link(manifest)).expect("library test initialization metadata");
    }
}

#[test]
fn package_main_in_a_main_named_project_requires_executable_entry_files() {
    let f = Fixture::new();
    f.put("library/ond.toml", "[project]\nname = \"main\"\n");
    f.put("library/api.ond", "package main\nvar Value:i32=1\n");
    f.put(
        "library/api_test.ond",
        "package main\nfunc TestRoot(){_=Value}\n",
    );
    let mut request = test_request(&f);
    request.project_root = f.0.join("library");
    let errors = compile(request, &compiler_identity().unwrap()).unwrap_err();
    assert!(
        errors.iter().any(|error| error.contains("main.ond")),
        "{errors:?}"
    );
}

#[test]
fn malformed_cached_test_metadata_is_rebuilt_after_frontend() {
    let f = Fixture::new();
    f.put("main_test.ond", "package main\nfunc TestRoot(){}\n");
    let id = compiler_identity().unwrap();
    let request = test_request(&f);
    let cold = compile(request.clone(), &id).unwrap();
    let reference = cold
        .manifest
        .packages
        .iter()
        .find(|r| r.package == "test")
        .unwrap();
    let mut artifact = cache::read_artifact(reference).unwrap();
    artifact.tests = None;
    cache::publish(reference.path.parent().unwrap(), &artifact).unwrap();
    let recovered = compile(request, &id).unwrap();
    assert!(recovered.rebuilt.contains(&".".into()));
    link_with_debug(request_link(recovered.manifest)).expect("repaired test metadata");
}

#[test]
fn artifact_test_cases_require_ownership_and_uniqueness() {
    let mut rejected = Vec::new();
    for scenario in ["foreign", "duplicate"] {
        let f = Fixture::new();
        f.put("main.ond", "package main\nfunc main(){}\n");
        f.put("main_test.ond", "package main\nfunc TestRoot(){}\n");
        let mut manifest = compile(test_request(&f), &compiler_identity().unwrap())
            .unwrap()
            .manifest;
        let root = manifest
            .packages
            .iter()
            .position(|reference| reference.package == "test")
            .unwrap();
        let mut artifact = cache::read_artifact(&manifest.packages[root]).unwrap();
        let case = artifact.tests.as_ref().unwrap().cases[0].clone();
        if scenario == "foreign" {
            artifact.tests.as_mut().unwrap().cases.clear();
            let other = manifest
                .packages
                .iter()
                .position(|reference| reference.package != "test")
                .unwrap();
            let mut foreign = cache::read_artifact(&manifest.packages[other]).unwrap();
            foreign.tests.as_mut().unwrap().cases.push(case);
            manifest.packages[other] =
                cache::publish(manifest.packages[other].path.parent().unwrap(), &foreign).unwrap();
        } else {
            artifact.tests.as_mut().unwrap().cases.push(case);
        }
        manifest.packages[root] =
            cache::publish(manifest.packages[root].path.parent().unwrap(), &artifact).unwrap();
        rejected.push(link_with_debug(request_link(manifest)).is_err());
    }
    assert_eq!(rejected, [true, true], "foreign owner, duplicate case");
}

#[test]
fn test_compiler_collects_valid_test_only_functions() {
    let f = Fixture::new();
    f.put(
        "main.ond",
        "package main\nfunc main(){}\nfunc TestOrdinary(x:u32){}\n",
    );
    f.put("main_test.ond", "package main\nfunc TestRoot(){}\n");
    let manifest = compile(test_request(&f), &compiler_identity().unwrap())
        .unwrap()
        .manifest;
    let cases = &manifest.tests.as_ref().unwrap().cases;
    assert_eq!(cases.len(), 1);
    assert_eq!(
        (
            cases[0].package.as_str(),
            cases[0].name.as_str(),
            cases[0].symbol.as_str()
        ),
        ("test", "TestRoot", "main.TestRoot")
    );
    let root = manifest
        .packages
        .iter()
        .find(|reference| reference.package == "test")
        .unwrap();
    let artifact = cache::read_artifact(root).unwrap();
    assert_eq!(artifact.tests.as_ref().unwrap().cases, *cases);
    assert!(!artifact.interface.exports.contains_key("TestRoot"));
    assert!(artifact.interface.exports.contains_key("TestOrdinary"));
    link_with_debug(request_link(manifest))
        .expect("test-only signatures are guaranteed by compilation");
}

#[test]
fn test_build_discovers_unreachable_packages_and_separates_cache_modes() {
    let f = Fixture::new();
    f.put(
        "main_test.ond",
        "package main\nfunc TestRoot(){}\nfunc helper(){}\ntype TestType struct {}\n",
    );
    f.put(
        "isolated/only_test.ond",
        "package isolated\nfunc TestUnreachable(){}\n",
    );
    let normal = f.request();
    crate::dependency_test_support::source(&f.0, "sdk/lib.ond", "package sdk\nfunc Run(){}");
    crate::dependency_test_support::source(&f.0, "sdk/bad_test.ond", "invalid syntax");
    crate::dependency_test_support::source(&f.0, "sdk/test.ond", "invalid syntax");
    let normal = compile(normal, "compiler").unwrap();
    assert!(normal.manifest.tests.is_none());
    assert!(
        serde_json::to_value(&normal.manifest)
            .unwrap()
            .get("tests")
            .is_none()
    );
    assert!(
        !normal
            .manifest
            .packages
            .iter()
            .any(|p| p.package == "test/isolated")
    );
    let request = test_request(&f);
    crate::dependency_test_support::source(&f.0, "sdk/bad_test.ond", "invalid syntax");
    crate::dependency_test_support::source(&f.0, "sdk/test.ond", "invalid syntax");
    let tests = compile(request.clone(), "compiler").unwrap();
    assert_eq!(
        tests
            .manifest
            .tests
            .as_ref()
            .unwrap()
            .cases
            .iter()
            .map(|c| (c.package.as_str(), c.name.as_str(), c.symbol.as_str()))
            .collect::<Vec<_>>(),
        [
            ("test", "TestRoot", "main.TestRoot"),
            (
                "test/isolated",
                "TestUnreachable",
                "test/isolated.TestUnreachable"
            )
        ]
    );
    assert_ne!(
        normal.manifest.packages[0].build_id,
        tests.manifest.packages[0].build_id
    );
    assert!(!compile(request, "compiler").unwrap().frontend_ran);
    let artifact = cache::read_artifact(&tests.manifest.packages[0]).unwrap();
    assert!(!artifact.interface.exports.contains_key("TestRoot"));
    assert!(!artifact.interface.exports.contains_key("TestType"));
}

#[test]
fn test_build_recognizes_exact_test_ond_basename_only() {
    let f = Fixture::new();
    f.put("main.ond", "package main\nfunc main(){}\n");
    f.put("test.ond", "package main\nfunc TestExact(){}\n");
    f.put("contest.ond", "package main\nfunc TestContest(){}\n");
    f.put("testing.ond", "package main\nfunc TestTesting(){}\n");

    let normal = compile(f.request(), "compiler").unwrap();
    assert!(normal.manifest.tests.is_none());

    let tests = compile(test_request(&f), "compiler").unwrap();
    assert_eq!(
        tests
            .manifest
            .tests
            .unwrap()
            .cases
            .iter()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        ["TestExact"]
    );
}

#[test]
fn test_build_visibility_covers_values_types_methods_and_generic_helpers() {
    for source in [
        "func main(){ Helper() }",
        "var X=Helper()\nfunc main(){}",
        "var X:TestType\nfunc main(){}",
        "func main(){ var x=Normal{}; x.Helper() }",
        "func unused[T](x:T){ Helper() }\nfunc main(){}",
        "func unused[T](x:T){var hidden:TestType}\nfunc main(){}",
    ] {
        let f = Fixture::new();
        f.put(
            "main.ond",
            &format!("package main\ntype Normal struct {{}}\n{source}"),
        );
        f.put("main_test.ond", "package main\ntype TestType struct {}\nfunc Helper()->u32{return 1}\nfunc (n:Normal) Helper(){}\nfunc TestOK(){}");
        assert!(compile(test_request(&f), "compiler").is_err(), "{source}");
    }
    let f = Fixture::new();
    f.put(
        "main.ond",
        "package main\nfunc private()->u32{return 7}\nfunc main(){}",
    );
    f.put(
        "main_test.ond",
        "package main\nfunc TestPrivate(){var _=private()}",
    );
    assert!(compile(test_request(&f), "compiler").is_ok());
    f.put("left/left_test.ond", "package left\nfunc Helper(){}\n");
    f.put(
        "main_test.ond",
        "package main\nimport \"test/left\"\nfunc TestExternal(){left.Helper()}",
    );
    assert!(compile(test_request(&f), "compiler").is_err());
}

#[test]
fn test_build_rejects_invalid_signatures_and_import_cycles_and_requires_app_main_file() {
    for function in [
        "func TestBad(x:u32){}",
        "func TestBad()->u32{return 1}",
        "func TestBad[T](){}",
        "func (x:Device) TestBad(){}",
    ] {
        let f = Fixture::new();
        f.put(
            "main_test.ond",
            &format!("package main\ntype Device struct {{}}\n{function}"),
        );
        assert!(
            compile(test_request(&f), "compiler")
                .unwrap_err()
                .join(" ")
                .contains("test function")
        );
    }
    let f = Fixture::new();
    f.put(
        "leaf/leaf_test.ond",
        "package leaf\nimport \"test/left\"\nfunc TestCycle(){var _=left.Value()}",
    );
    assert!(
        compile(test_request(&f), "compiler")
            .unwrap_err()
            .join(" ")
            .contains("cyclic")
    );
    fs::remove_file(f.0.join("leaf/leaf_test.ond")).unwrap();
    fs::rename(f.0.join("main.ond"), f.0.join("entry.ond")).unwrap();
    assert!(
        compile(test_request(&f), "compiler")
            .unwrap_err()
            .join(" ")
            .contains("main.ond")
    );
}

#[test]
fn library_test_build_links_without_an_executable_main() {
    for root_main_file in [false, true] {
        let f = Fixture::new();
        fs::remove_file(f.0.join("main.ond")).unwrap();
        f.put(
            "api.ond",
            "package test\nimport \"test/left\"\nfunc private() -> i32 { return left.Value() }\nfunc identity[T](value: T) -> T { return value }\n",
        );
        if root_main_file {
            f.put("main.ond", "package test\nfunc Root() {}\n");
        }
        f.put(
            "api_test.ond",
            "package test\nfunc TestRoot() { _ = private(); _ = identity[i32](1) }\n",
        );
        f.put(
            "left/left_test.ond",
            "package left\nfunc TestLeft() { _ = Value() }\n",
        );

        assert!(compile(f.request(), "compiler").is_err());
        let id = compiler_identity().unwrap();
        let request = test_request(&f);
        let result = compile(request.clone(), &id).unwrap();
        let cases = &result.manifest.tests.as_ref().unwrap().cases;
        assert!(cases.iter().any(|case| {
            case.package == "test" && case.name == "TestRoot" && case.symbol == "test.TestRoot"
        }));
        assert!(cases.iter().any(|case| {
            case.package == "test/left"
                && case.name == "TestLeft"
                && case.symbol == "test/left.TestLeft"
        }));
        assert_eq!(result.manifest.program.entry_symbol, "main.main");
        assert!(!f.0.join(".ond/__test_entry__/main.ond").exists());
        assert_eq!(f.0.join("main.ond").exists(), root_main_file);
        let sidecar = link_with_debug(request_link(result.manifest))
            .unwrap()
            .tests
            .unwrap();
        assert_eq!(sidecar.cases.len(), 2);
        assert!(!compile(request, &id).unwrap().frontend_ran);
    }
}

#[test]
fn library_test_build_links_with_no_test_cases() {
    let f = Fixture::new();
    fs::remove_file(f.0.join("main.ond")).unwrap();
    f.put(
        "api.ond",
        "package test\nfunc Value() -> i32 { return 1 }\n",
    );
    let manifest = compile(test_request(&f), &compiler_identity().unwrap())
        .unwrap()
        .manifest;
    assert!(manifest.tests.as_ref().unwrap().cases.is_empty());
    assert!(
        link_with_debug(request_link(manifest))
            .unwrap()
            .tests
            .unwrap()
            .cases
            .is_empty()
    );
}

#[test]
fn virtual_project_sources_build_tests_without_a_disk_entry_file() {
    let f = Fixture::new();
    f.put("virtual/ond.toml", "[project]\nname = \"test\"\n");
    let mut request = test_request(&f);
    request.project_root = f.0.join("virtual");
    request.sources = BTreeMap::from([
        ("api.ond".into(), "package test\nfunc Own() {}\n".into()),
        (
            "api_test.ond".into(),
            "package test\nfunc TestRoot() { Own() }\n".into(),
        ),
    ]);
    let result = compile(request.clone(), &compiler_identity().unwrap()).unwrap();
    assert_eq!(result.manifest.tests.as_ref().unwrap().cases.len(), 1);
    assert_eq!(
        result.manifest.tests.as_ref().unwrap().cases[0].package,
        "test"
    );
    assert_eq!(
        link_with_debug(request_link(result.manifest))
            .unwrap()
            .tests
            .unwrap()
            .cases
            .len(),
        1
    );
    assert!(!request.project_root.join("main.ond").exists());
    request.test = false;
    assert!(compile(request.clone(), "compiler").is_err());

    request.sources = BTreeMap::from([(
        "main.ond".into(),
        "package main\nfunc main() {}\nfunc TestRoot() {}\n".into(),
    )]);
    assert!(compile(request, "compiler").is_ok());
}

#[test]
fn test_build_link_sidecar_entries_and_metadata_boundary() {
    let f = Fixture::new();
    f.put(
        "main_test.ond",
        "package main\nfunc TestOne(){}\nfunc TestTwo(){}",
    );
    let id = compiler_identity().unwrap();
    let manifest = compile(test_request(&f), &id).unwrap().manifest;
    let linked = link_with_debug(request_link(manifest.clone())).unwrap();
    let sidecar = linked.tests.unwrap();
    assert_eq!(sidecar.build_id, linked.image.build_id);
    assert_eq!(sidecar.cases.len(), 2);
    assert_ne!(sidecar.cases[0].entry_point, sidecar.cases[1].entry_point);
    for case in sidecar.cases {
        assert_ne!(case.entry_point, linked.image.entry_point);
        assert_ne!(
            case.entry_point,
            linked.image.symbols[&format!("main.{}", case.name)]
        );
    }
    assert!(linked.debug.ranges.iter().any(|r| {
        r.start == linked.image.entry_point
            && r.message
                .as_ref()
                .is_some_and(|s| s.contains("select a test case"))
    }));
    let mut malformed = manifest.clone();
    malformed.tests.as_mut().unwrap().cases[0].symbol = "main.main".into();
    assert!(link(request_link(malformed)).is_err());
    let mut malformed = manifest.clone();
    malformed
        .tests
        .as_mut()
        .unwrap()
        .cases
        .push(manifest.tests.as_ref().unwrap().cases[0].clone());
    assert!(link(request_link(malformed)).is_err());
    let mut malformed = manifest.clone();
    malformed.tests = None;
    assert!(link(request_link(malformed)).is_err());
    let mut json = serde_json::to_value(manifest).unwrap();
    json["tests"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ond_protocol::CompilationManifest>(json).is_err());
    fs::remove_file(f.0.join("main_test.ond")).unwrap();
    let empty = compile(test_request(&f), &id).unwrap().manifest;
    assert!(empty.tests.as_ref().unwrap().cases.is_empty());
    assert!(
        link_with_debug(request_link(empty))
            .unwrap()
            .tests
            .unwrap()
            .cases
            .is_empty()
    );
}

#[test]
fn test_build_unused_generic_cannot_reference_other_package_tests() {
    for body in ["var hidden:left.TestType", "left.TestHelper()"] {
        let f = Fixture::new();
        f.put(
            "main.ond",
            &format!(
                "package main\nimport \"test/left\"\nfunc unused[T](x:T){{{body}}}\nfunc main(){{}}"
            ),
        );
        f.put(
            "left/left_test.ond",
            "package left\ntype TestType struct {}\nfunc TestHelper(){}",
        );
        assert!(compile(test_request(&f), "compiler").is_err(), "{body}");
    }
    let f = Fixture::new();
    f.put("main.ond", "package main\nimport \"test/left\"\nfunc unused[T](x:T){var value:left.Normal;left.Helper()}\nfunc main(){}");
    f.put(
        "left/left.ond",
        "package left\ntype Normal struct {}\nfunc Helper(){}",
    );
    f.put("left/left_test.ond", "package left\nfunc TestOK(){}");
    assert!(compile(test_request(&f), "compiler").is_ok());
}

#[test]
fn test_build_generic_specialization_preserves_declaration_source_visibility() {
    for (normal, tests) in [
        (
            "type Normal struct {}\nfunc unused[T](x:Normal){x.Helper()}",
            "func (n:Normal) Helper(){}\nfunc TestOK(){unused[u32](Normal{})}",
        ),
        (
            "type Normal[T] struct { F:TestType }",
            "type TestType struct { X:u32 }\nfunc TestOK(){var x:Normal[u32];var _=x.F.X}",
        ),
        (
            "type Normal struct {}\nfunc unused[T](x:Normal)->u32{return x+x}",
            "operator (x:Normal) +(y:Normal)->u32{return 1}\nfunc TestOK(){var _=unused[u32](Normal{})}",
        ),
    ] {
        let f = Fixture::new();
        f.put(
            "main.ond",
            &format!("package main\n{normal}\nfunc main(){{}}"),
        );
        f.put("main_test.ond", &format!("package main\n{tests}"));
        let error = compile(test_request(&f), "compiler").unwrap_err().join(" ");
        assert!(!error.contains("expected"), "{error}");
        if tests.starts_with("operator") {
            assert!(error.contains("invalid binary operand type"), "{error}");
            let (operator, case) = tests.split_once('\n').unwrap();
            f.put(
                "main.ond",
                &format!("package main\n{normal}\n{operator}\nfunc main(){{}}"),
            );
            f.put("main_test.ond", &format!("package main\n{case}"));
            assert!(compile(test_request(&f), "compiler").is_ok());
        }
    }
    let f = Fixture::new();
    f.put("main.ond", "package main\ntype Normal struct {}\nfunc (x:Normal) Helper(){}\nfunc fixed[T](x:Normal){x.Helper()}\nfunc dispatch[T](x:T){x.Helper()}\nfunc forward[T](x:T)->T{return x}\nfunc main(){}");
    f.put("main_test.ond", "package main\ntype TestType struct {}\nfunc (x:TestType) Helper(){}\nfunc TestOK(){fixed[u32](Normal{});dispatch[TestType](TestType{});var _=forward[TestType](TestType{})}");
    assert!(compile(test_request(&f), "compiler").is_ok());
}

#[test]
fn test_build_hidden_types_do_not_collide_with_normal_receiver_type_parameters() {
    for (declaration, call) in [
        ("func (b:Box[T]) Get()->T{return b.Value}", "var _=b.Get()"),
        ("operator (b:Box[T]) len()->u32{return 1}", "var _=len(b)"),
    ] {
        let f = Fixture::new();
        let normal =
            format!("package main\ntype Box[T] struct {{Value:T}}\n{declaration}\nfunc main(){{}}");
        f.put("main.ond", &normal);
        assert!(compile(f.request(), "compiler").is_ok(), "{declaration}");
        f.put(
            "main_test.ond",
            &format!("package main\ntype T struct {{}}\nfunc TestBox(){{var b:Box[u32];{call}}}"),
        );
        assert!(
            compile(test_request(&f), "compiler").is_ok(),
            "{declaration}"
        );
        // A type visible in the receiver's own normal scope still collides.
        f.put(
            "main.ond",
            &normal.replace("type Box[T]", "type T struct {}\ntype Box[T]"),
        );
        fs::remove_file(f.0.join("main_test.ond")).unwrap();
        assert!(
            compile(f.request(), "compiler")
                .unwrap_err()
                .join(" ")
                .contains("fresh"),
            "{declaration}"
        );
        assert!(
            compile(test_request(&f), "compiler")
                .unwrap_err()
                .join(" ")
                .contains("fresh"),
            "{declaration}"
        );
    }
}

#[test]
fn test_build_hidden_declarations_do_not_shadow_normal_import_aliases() {
    let f = Fixture::new();
    f.put("main.ond", "package main\nimport helper \"test/left\"\nfunc use[T](){helper.Value()}\nfunc main(){helper.Value();use[u32]()}\n");
    f.put(
        "main_test.ond",
        "package main\nfunc helper(){}\nfunc TestHelper(){helper()}\n",
    );
    assert!(compile(f.request(), "compiler").is_ok());
    assert!(compile(test_request(&f), "compiler").is_ok());
    // A test file sees normal declarations, so its alias conflict is real.
    f.put("main.ond", "package main\nfunc helper(){}\nfunc main(){}\n");
    f.put(
        "main_test.ond",
        "package main\nimport helper \"test/left\"\nfunc TestHelper(){helper.Value()}\n",
    );
    assert!(
        compile(test_request(&f), "compiler")
            .unwrap_err()
            .join(" ")
            .contains("conflicts with imported package")
    );
}
