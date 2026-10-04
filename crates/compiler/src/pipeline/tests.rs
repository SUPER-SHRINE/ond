#[cfg(test)]
mod tests {
    use super::compile_project;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("ond-pipeline-test-{nanos}"))
    }

    fn write_file(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn compile_project_error(files: &[(&str, &str)]) -> String {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        write_file(&root, "ond.toml", "[project]\nname=\"compile-fail\"\n");
        for (path, contents) in files {
            write_file(&root, path, contents);
        }
        compile_project(&root)
            .expect_err("project should fail to compile")
            .to_string()
    }

    #[test]
    fn compile_project_builds_main_object_for_minimal_project() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("ond.toml"), "[project]\nname=\"sample\"\n").unwrap();
        fs::write(root.join("main.ond"), "package main\n\nfunc main() {\n}\n").unwrap();

        let compilation = compile_project(&root).expect("minimal project should compile");
        assert_eq!(compilation.loaded.packages.len(), 1);
        assert!(
            compilation
                .objects
                .iter()
                .any(|object| object.symbols.iter().any(|s| s.name == "main.main"))
        );
    }

    #[test]
    fn public_pipeline_uses_mir_for_byte_literals_and_user_defined_str() {
        for source in [
            "package main\nfunc main() { var text: [3]u8 = \"abc\"; _ = text; }\n",
            "package main\ntype str [2]u8\nfunc main() { var value: str; _ = value; }\n",
        ] {
            let root = temp_dir();
            fs::create_dir_all(&root).unwrap();
            write_file(&root, "ond.toml", "[project]\nname=\"mir\"\n");
            write_file(&root, "main.ond", source);
            let compilation = compile_project(&root).unwrap();
            let expected = crate::test_support::build_image(
                &compilation.mir,
                crate::ProjectConfig::default(),
            )
            .unwrap();
            assert_eq!(
                format!("{:?}", crate::test_support::build_compilation(&compilation).unwrap()),
                format!("{expected:?}")
            );
            assert_eq!(
                format!("{:?}", crate::test_support::compile_image(&root).unwrap()),
                format!("{expected:?}")
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn compile_project_accepts_u16_i8_i16_builtin_types() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("ond.toml"),
            "[project]\nname=\"small-int-types\"\n",
        )
        .unwrap();
        fs::write(
            root.join("main.ond"),
            "package main\n\nfunc bump(value: u16, delta: i8) -> u16 {\n    var signed: i16 = delta as i16\n    var values: [2]u16 = [2]u16{ value, 1 as u16 }\n    if signed < 0 as i16 {\n        return values[0]\n    }\n    return values[0] + values[1]\n}\n\nfunc main() {\n    var base: u16 = 3 as u16\n    var delta: i8 = 1 as i8\n    var total: u16 = bump(base, delta)\n    if total == 0 as u16 {\n        total = 1 as u16\n    }\n}\n",
        )
        .unwrap();

        let compilation = compile_project(&root).expect("small integer built-ins should compile");
        assert!(
            compilation
                .objects
                .iter()
                .any(|object| object.symbols.iter().any(|s| s.name == "main.main"))
        );
    }

    #[test]
    fn compile_project_reports_import_cycles() {
        let root = temp_dir();
        fs::create_dir_all(root.join("foo")).unwrap();
        fs::create_dir_all(root.join("bar")).unwrap();
        fs::write(root.join("ond.toml"), "[project]\nname=\"cycle\"\n").unwrap();
        fs::write(
            root.join("main.ond"),
            "package main\n\nimport \"foo\"\n\nfunc main() {\n    foo.Run()\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("foo").join("foo.ond"),
            "package foo\n\nimport \"bar\"\n\nfunc Run() {\n    bar.Run()\n}\n",
        )
        .unwrap();
        fs::write(
            root.join("bar").join("bar.ond"),
            "package bar\n\nimport \"foo\"\n\nfunc Run() {\n    foo.Run()\n}\n",
        )
        .unwrap();

        let error = compile_project(&root).expect_err("import cycle must be rejected");
        assert!(error.to_string().contains("import cycle detected"));
    }

    #[test]
    fn compile_project_rejects_nil_without_type_context() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\nfunc main() {\n    value := nil\n}\n",
        )]);
        assert!(error.contains("`nil` requires an explicit pointer or function type context"));
    }

    #[test]
    fn compile_project_rejects_addr_of_composite_literal() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\ntype Item struct {\n    Value: u32\n}\n\nfunc main() {\n    var item: *Item = &Item{ Value: 1 as u32 }\n}\n",
        )]);
        assert!(error.contains("`&T{...}` is not supported"));
    }

    #[test]
    fn compile_project_rejects_invalid_struct_composite_field() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\ntype Item struct {\n    Value: u32\n}\n\nfunc main() {\n    var item: Item = Item{ Missing: 1 as u32 }\n}\n",
        )]);
        assert!(error.contains("unknown struct field `Missing`"));
    }

    #[test]
    fn compile_project_rejects_defined_type_and_underlying_type_mismatch() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\ntype UserId u32\n\nfunc main() {\n    var raw: u32 = 1 as u32\n    var id: UserId = raw\n}\n",
        )]);
        assert!(
            error.contains("type mismatch: expected")
        );
    }

    #[test]
    fn compile_project_rejects_direct_recursive_type() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\ntype Node Node\n\nfunc main() {\n}\n",
        )]);
        assert!(error.contains("direct recursive type `Node` is not allowed"));
    }

    #[test]
    fn compile_project_rejects_missing_return_value() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\nfunc value() -> u32 {\n}\n\nfunc main() {\n    value()\n}\n",
        )]);
        assert!(error.contains("missing return") || error.contains("return"));
    }

    #[test]
    fn compile_project_rejects_return_arity_mismatch() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\nfunc pair() -> (u32, u32) {\n    return 1 as u32\n}\n\nfunc main() {\n    pair()\n}\n",
        )]);
        assert!(error.contains("return value count mismatch: expected 2, found 1"));
    }

    #[test]
    fn compile_project_rejects_function_argument_count_mismatch() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\nfunc target(value: u32) {\n}\n\nfunc main() {\n    target()\n}\n",
        )]);
        assert!(error.contains("function expects 1 arguments, found 0"));
    }

    #[test]
    fn compile_project_rejects_function_argument_type_mismatch() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\nfunc target(value: u32) {\n}\n\nfunc main() {\n    target(1 as i32)\n}\n",
        )]);
        assert!(error.contains("argument 1 type mismatch"));
    }

    #[test]
    fn compile_project_does_not_predeclare_asset() {
        let error = compile_project_error(&[(
            "main.ond",
            "package main\n\nfunc main() {\n    asset(0 as u32)\n}\n",
        )]);
        assert!(error.contains("asset"), "{error}");
    }

    #[test]
    fn compile_project_accepts_ordinary_function_named_init() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        write_file(&root, "ond.toml", "[project]\nname=\"init-name\"\n");
        write_file(
            &root,
            "main.ond",
            "package main\n\nfunc init() {\n}\n\nfunc main() {\n}\n",
        );
        compile_project(&root).expect("init is an ordinary function name");
    }

    #[test]
    fn compile_project_rejects_lowercase_package_qualified_access() {
        let error = compile_project_error(&[
            (
                "main.ond",
                "package main\n\nimport \"foo\"\n\nfunc main() {\n    foo.hidden()\n}\n",
            ),
            ("foo/foo.ond", "package foo\n\nfunc hidden() {\n}\n"),
        ]);
        assert!(error.contains("function `foo.hidden` is not exported"));
    }
}
