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
fn identical_library_root_files_preserve_project_sources_and_conflicts_fail() {
    let root = project("duplicate-library", "shared");
    let sdk = root.join("target/sdk");
    fs::create_dir_all(sdk.join("shared")).unwrap();
    let source = "package shared\nfunc Own() {}\n";
    fs::write(root.join("api.ond"), source).unwrap();
    fs::write(sdk.join("shared/api.ond"), source).unwrap();
    let state = ServerState::with_library_roots(vec![sdk.clone()]);
    let loaded = state.load_current_project(&root).unwrap();
    let package = loaded
        .packages
        .iter()
        .find(|p| p.logical_path == "shared")
        .unwrap();
    assert_eq!(package.files.len(), 1);
    assert_eq!(
        loaded.sources.file(package.files[0]).path(),
        root.join("api.ond").canonicalize().unwrap()
    );
    assert!(compiler::compile_loaded_project(loaded).is_ok());
    fs::write(
        sdk.join("shared/extra.ond"),
        "package shared\nfunc extra() {}\n",
    )
    .unwrap();
    assert!(
        state
            .load_current_project(&root)
            .unwrap_err()
            .contains("library conflicts with project package: shared")
    );
    fs::remove_dir_all(root).unwrap();
}
