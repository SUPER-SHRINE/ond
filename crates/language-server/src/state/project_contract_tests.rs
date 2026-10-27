use super::ServerState;
use crate::protocol::TextDocumentContentChangeEvent;
use crate::util::uri_from_path;
use std::fs;
use std::path::PathBuf;

fn project(label: &str, name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ond-lsp-project-contract-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    fs::write(
        root.join("ond.toml"),
        format!("[project]\nname = \"{name}\"\n"),
    )
    .unwrap();
    root
}

#[test]
fn app_navigation_uses_qualified_import_and_rejects_relative_import() {
    let root = project("app", "game");
    fs::create_dir(root.join("math")).unwrap();
    fs::write(
        root.join("math/math.ond"),
        "package math\nfunc Add() -> i32 { return 42 }\n",
    )
    .unwrap();
    let source = "package main\nimport \"game/math\"\nfunc main() { _ = math.Add() }\n";
    let path = root.join("main.ond");
    fs::write(&path, source).unwrap();
    let uri = uri_from_path(&path).unwrap();
    let mut state = ServerState::default();
    let mut output = Vec::new();
    state
        .open_document(uri.clone(), source.to_string(), &mut output)
        .unwrap();
    let diagnostics = super::tests::published_diagnostics(&output, &uri);
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|items| items.as_array().is_some_and(Vec::is_empty)),
        "{diagnostics:?}"
    );
    let line = source.lines().nth(2).unwrap();
    let call = line.find("math.Add").unwrap() as u32;
    let completions = state.completions(&uri, 2, call + 5).unwrap();
    assert!(completions.iter().any(|item| item["label"] == "Add"));
    let definition = state.definition(&uri, 2, call + 5).unwrap();
    assert!(
        definition[0]["targetUri"]
            .as_str()
            .unwrap()
            .ends_with("math.ond"),
        "{definition}"
    );

    let relative = source.replace("game/math", "math");
    let mut output = Vec::new();
    state
        .change_document(
            uri,
            vec![TextDocumentContentChangeEvent {
                text: relative,
                range: None,
            }],
            &mut output,
        )
        .unwrap();
    let diagnostics = String::from_utf8(output).unwrap();
    assert!(
        diagnostics.contains("unknown import path `math`"),
        "{diagnostics}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn library_without_main_resolves_qualified_sibling_in_editor() {
    let root = project("library", "shared");
    fs::create_dir(root.join("sibling")).unwrap();
    fs::write(
        root.join("sibling/value.ond"),
        "package sibling\nfunc Value() -> i32 { return 42 }\n",
    )
    .unwrap();
    let source =
        "package shared\nimport \"shared/sibling\"\nfunc Use() -> i32 { return sibling.Value() }\n";
    let path = root.join("api.ond");
    fs::write(&path, source).unwrap();
    let uri = uri_from_path(&path).unwrap();
    let mut state = ServerState::default();
    let mut output = Vec::new();
    state
        .open_document(uri.clone(), source.to_string(), &mut output)
        .unwrap();
    let diagnostics = super::tests::published_diagnostics(&output, &uri);
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|items| items.as_array().is_some_and(Vec::is_empty)),
        "{diagnostics:?}"
    );
    let line = source.lines().nth(2).unwrap();
    let call = line.find("sibling.Value").unwrap() as u32;
    let completions = state.completions(&uri, 2, call + 8).unwrap();
    assert!(completions.iter().any(|item| item["label"] == "Value"));
    let definition = state.definition(&uri, 2, call + 8).unwrap();
    assert!(
        definition[0]["targetUri"]
            .as_str()
            .unwrap()
            .ends_with("value.ond"),
        "{definition}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn identical_dependency_names_from_distinct_directories_are_rejected() {
    let root = project("duplicate-dependency", "shared");
    let sdk = root.join("target/sdk");
    fs::create_dir_all(sdk.join("shared")).unwrap();
    let source = "package shared\nfunc Own() {}\n";
    fs::write(root.join("api.ond"), source).unwrap();
    fs::write(sdk.join("shared/api.ond"), source).unwrap();
    let state = ServerState::with_dependencies(&root, vec![sdk]);
    assert!(
        state
            .load_current_project(&root)
            .unwrap_err()
            .contains("dependency namespace")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pkg01_source_file_lsp_completes_and_resolves_every_filename() {
    let root = project("source-file-lsp", "p");
    let main = "package main\nvar Result:u32\nfunc main(){var value:Device;_=value.secret;Result=TestArgument(TestResult())+TestGeneric[u32](2)+value.TestMethod()+Collect()}\n";
    let test = "package main\ntype Device struct{secret:u32}\nfunc TestArgument(x:u32)->u32{return x}\nfunc TestResult()->u32{return 1}\nfunc TestGeneric[T](x:T)->T{return x}\nfunc(x:Device) TestMethod()->u32{return x.secret+3}\nfunc privateHelper()->u32{return 4}\n";
    let feature = "package main\nfunc Collect()->u32{return privateHelper()}\n";
    let mut state = ServerState::default();
    let mut output = Vec::new();
    let mut uris = Vec::new();
    for (name, source) in [
        ("main.ond", main),
        ("test.ond", test),
        ("feature_test.ond", feature),
    ] {
        let path = root.join(name);
        fs::write(&path, source).unwrap();
        uris.push(uri_from_path(&path).unwrap());
    }
    for (uri, source) in uris.iter().zip([main, test, feature]) {
        state
            .open_document(uri.clone(), source.into(), &mut output)
            .unwrap();
    }
    for (uri, line, text, needle) in [
        (&uris[0], 2, main.lines().nth(2).unwrap(), "TestArgument"),
        (&uris[1], 2, test.lines().nth(2).unwrap(), "return x"),
        (
            &uris[2],
            1,
            feature.lines().nth(1).unwrap(),
            "return privateHelper",
        ),
    ] {
        let at =
            text.find(needle).unwrap() as u32 + if needle.starts_with("return") { 7 } else { 0 };
        let items = state.completions(uri, line, at).unwrap();
        for expected in [
            "privateHelper",
            "Collect",
            "TestArgument",
            "TestResult",
            "TestGeneric",
        ] {
            assert!(
                items.iter().any(|item| item["label"] == expected),
                "PKG-01.source-file-lsp: {expected}: {items:?}"
            );
        }
    }
    let line = main.lines().nth(2).unwrap();
    let definition = state
        .definition(&uris[0], 2, line.find("secret").unwrap() as u32)
        .unwrap();
    assert!(
        definition[0]["targetUri"]
            .as_str()
            .unwrap()
            .ends_with("test.ond")
    );
    let at = line.find("TestArgument").unwrap() as u32;
    let definition = state.definition(&uris[0], 2, at).unwrap();
    assert_eq!(definition[0]["targetUri"], uris[1]);
    let references = state.references(&uris[0], 2, at, true).unwrap();
    assert_eq!(references.as_array().unwrap().len(), 2);
    let edit = state.rename(&uris[0], 2, at, "Argument").unwrap();
    assert_eq!(edit["changes"].as_object().unwrap().len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_filename_rename_keeps_private_and_public_occurrences() {
    for name in ["Helper", "helper"] {
        let root = project(name, "p");
        let normal = format!("package main\nfunc main(){{{name}()}}\n");
        let suffix = format!("package main\nfunc {name}(){{}}\nfunc TestLocal(){{{name}()}}\n");
        fs::write(root.join("main.ond"), &normal).unwrap();
        fs::write(root.join("feature_test.ond"), &suffix).unwrap();
        let uri = uri_from_path(&root.join("main.ond")).unwrap();
        let suffix_uri = uri_from_path(&root.join("feature_test.ond")).unwrap();
        let mut state = ServerState::default();
        state
            .open_document(uri.clone(), normal.clone(), &mut Vec::new())
            .unwrap();
        let at = normal.lines().nth(1).unwrap().find(name).unwrap() as u32;
        assert!(!state.prepare_rename(&uri, 1, at).unwrap().is_null());
        assert_eq!(
            state
                .references(&uri, 1, at, true)
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let edits = state.rename(&uri, 1, at, "Changed").unwrap();
        assert_eq!(edits["changes"][&uri].as_array().unwrap().len(), 1);
        assert_eq!(edits["changes"][&suffix_uri].as_array().unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn dependency_suffix_api_completion_and_edits_invalidate_in_one_session() {
    let root = project("dependency-suffix", "p");
    fs::create_dir_all(root.join("lib")).unwrap();
    fs::write(
        root.join("ond.toml"),
        "[project]\nname=\"p\"\n[dependencies]\nlib={path=\"lib\"}\n",
    )
    .unwrap();
    fs::write(root.join("lib/ond.toml"), "[project]\nname=\"lib\"\n").unwrap();
    let source = "package main\nimport \"lib\"\nfunc main(){_=lib.TestValue()}\n";
    let library =
        "package lib\nfunc private()->u32{return 7}\nfunc TestValue()->u32{return private()}\n";
    fs::write(root.join("main.ond"), source).unwrap();
    fs::write(root.join("lib/api_test.ond"), library).unwrap();
    let uri = uri_from_path(&root.join("main.ond")).unwrap();
    let library_uri = uri_from_path(&root.join("lib/api_test.ond")).unwrap();
    let mut state = ServerState::default();
    state
        .open_document(uri.clone(), source.into(), &mut Vec::new())
        .unwrap();
    let at = source.lines().nth(2).unwrap().find("TestValue").unwrap() as u32;
    let items = state.completions(&uri, 2, at).unwrap();
    assert!(
        items.iter().any(|item| item["label"] == "TestValue"),
        "{items:?}"
    );
    assert!(!items.iter().any(|item| item["label"] == "private"));
    assert_eq!(
        state.definition(&uri, 2, at).unwrap()[0]["targetUri"],
        library_uri
    );
    state
        .open_document(library_uri.clone(), library.into(), &mut Vec::new())
        .unwrap();
    state
        .change_document(
            library_uri,
            vec![TextDocumentContentChangeEvent {
                text: library.replace("TestValue", "Replacement"),
                range: None,
            }],
            &mut Vec::new(),
        )
        .unwrap();
    let items = state.completions(&uri, 2, at).unwrap();
    assert!(!items.iter().any(|item| item["label"] == "TestValue"));
    assert!(items.iter().any(|item| item["label"] == "Replacement"));
    fs::remove_dir_all(root).unwrap();
}
