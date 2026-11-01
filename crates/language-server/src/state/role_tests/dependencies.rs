//! ROLE-09、07: rootの登録edgeが選んだ依存APIと設定変更時の失効。
use super::super::ServerState;
use crate::util::uri_from_path;
use std::collections::BTreeSet;
#[path = "../../../../cli/src/role_tests/fixtures.rs"]
mod fixtures;
use fixtures::*;
fn labels(state: &mut ServerState, uri: &str) -> Result<BTreeSet<String>, String> {
    let column = MAIN.lines().nth(3).unwrap().find("child.Value").unwrap() as u32 + 6;
    Ok(state
        .completions(uri, 3, column)?
        .iter()
        .filter_map(|item| item["label"].as_str())
        .filter(|label| ["Value", "CommonApi", "TestApi"].contains(label))
        .map(str::to_owned)
        .collect())
}
fn expected(selected: bool) -> BTreeSet<String> {
    if selected {
        ["Value", "CommonApi", "TestApi"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    } else {
        ["Value", "CommonApi"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}
#[test]
fn role_09_selected_api_and_role_07_edge_role_invalidation() {
    let f = ProjectFixture::new();
    let path = f.root.join("main.ond").canonicalize().unwrap();
    let uri = uri_from_path(&path).unwrap();
    let mut state = ServerState::default();
    state
        .open_document(uri.clone(), MAIN.into(), &mut Vec::new())
        .unwrap();
    assert_eq!(
        labels(&mut state, &uri).unwrap(),
        expected(true),
        "ROLE-09.selected-api"
    );
    assert_eq!(
        labels(&mut state, &uri).unwrap(),
        expected(true),
        "warm LSP index"
    );
    f.runner(&RUNNER.replace(",role=\"Test\"", ""));
    state.validate_uri(&uri, &mut Vec::new()).unwrap();
    assert_eq!(
        labels(&mut state, &uri).unwrap(),
        expected(false),
        "ROLE-07.edge-role-invalidation"
    );
    f.runner(RUNNER);
    state.validate_uri(&uri, &mut Vec::new()).unwrap();
    assert_eq!(
        labels(&mut state, &uri).unwrap(),
        expected(true),
        "named selection restored"
    );
}
#[test]
fn role_07_lsp_graph_failure_does_not_reuse_success_or_poison_repair() {
    for (case, runner, child, cause) in [
        (
            "ROLE-07.final-invalidation",
            RUNNER.replace("final=true", "final=false"),
            CHILD.to_string(),
            "role",
        ),
        (
            "ROLE-07.target-final-invalidation",
            RUNNER.to_string(),
            CHILD.replace("name=\"child\"", "name=\"child\"\nfinal=true"),
            "final",
        ),
        (
            "ROLE-07.selected-failure-invalidation",
            RUNNER.replace("role=\"Test\"", "role=\"Unknown\""),
            CHILD.to_string(),
            "Unknown",
        ),
    ] {
        let f = ProjectFixture::new();
        let uri = uri_from_path(&f.root.join("main.ond").canonicalize().unwrap()).unwrap();
        let mut state = ServerState::default();
        state
            .open_document(uri.clone(), MAIN.into(), &mut Vec::new())
            .unwrap();
        assert_eq!(labels(&mut state, &uri).unwrap(), expected(true));
        f.runner(&runner);
        f.put("child/ond.toml", &child);
        let error = labels(&mut state, &uri).unwrap_err();
        assert!(
            error.to_lowercase().contains(&cause.to_lowercase()),
            "{case}: {error}"
        );
        f.runner(&RUNNER.replace(",role=\"Test\"", ""));
        f.put("child/ond.toml", CHILD);
        state.validate_uri(&uri, &mut Vec::new()).unwrap();
        assert_eq!(
            labels(&mut state, &uri).unwrap(),
            expected(false),
            "{case}: corrected common API"
        );
    }
}
#[test]
fn role_10_lsp_checks_unimported_conflicting_paths() {
    let f = ProjectFixture::new();
    f.diamond();
    f.put("runner/main.ond", NO_IMPORT);
    let uri = uri_from_path(&f.root.join("main.ond").canonicalize().unwrap()).unwrap();
    let mut state = ServerState::default();
    let mut output = Vec::new();
    state
        .open_document(uri, NO_IMPORT.into(), &mut output)
        .unwrap();
    let diagnostic = String::from_utf8(output).unwrap();
    for cause in [
        "runner", "child", "direct", "middle", "bridge", "leaf", "Test",
    ] {
        assert!(
            diagnostic.contains(cause),
            "ROLE-10.unimported-conflict: {diagnostic}"
        );
    }
}

#[test]
fn role_07_graph_failure_discards_cached_semantic_token_bindings() {
    let f = ProjectFixture::new();
    let source = "package main\nimport \"child\"\nfunc main(){_=child.TestApi()}\n";
    f.put("runner/main.ond", source);
    let uri = uri_from_path(&f.root.join("main.ond").canonicalize().unwrap()).unwrap();
    let mut state = ServerState::default();
    state
        .open_document(uri.clone(), source.into(), &mut Vec::new())
        .unwrap();
    let column = source.lines().nth(2).unwrap().find("TestApi").unwrap() as u32;
    let has_binding = |tokens: Vec<u32>| {
        let (mut line, mut start) = (0, 0);
        tokens.chunks_exact(5).any(|token| {
            line += token[0];
            start = if token[0] == 0 {
                start + token[1]
            } else {
                token[1]
            };
            line == 2 && start == column
        })
    };
    assert!(has_binding(state.semantic_tokens(&uri).unwrap()));
    f.runner(&RUNNER.replace("final=true", "final=false"));
    state.watched_files_changed(&mut Vec::new()).unwrap();
    assert!(
        !has_binding(state.semantic_tokens(&uri).unwrap()),
        "ROLE-07 cannot reuse bindings after invalid graph settings"
    );
}

#[test]
fn role_09_dependency_source_adoption_uses_its_selected_role() {
    let f = ProjectFixture::new();
    f.put(
        "child/ond.toml",
        &format!(
            "{}[dependencies]\nunused={{path=\"../unused\"}}\n",
            CHILD.replace("[\"Test\"]", "[\"Test\",\"Other\"]")
        ),
    );
    f.put(
        "child/lib.ond",
        &format!(
            "package child\nrole Other {{import \"unused\"}}\n{}",
            CHILD_SOURCE.strip_prefix("package child\n").unwrap()
        ),
    );
    f.put("unused/ond.toml", "[project]\nname=\"unused\"\n");
    f.put("unused/lib.ond", "package unused\nfunc broken(\n");
    let state = ServerState::default();
    let loaded = state.load_current_project(&f.root).unwrap();
    assert!(
        !loaded
            .packages
            .iter()
            .any(|package| package.logical_path == "unused"),
        "ROLE-09 foreign nonselected import must not adopt source"
    );
}

#[test]
fn role_07_import_path_completion_failure_discards_cached_bindings() {
    let f = ProjectFixture::new();
    let source = "package main\nimport \"child\"\nfunc main(){_=child.TestApi()}\n";
    f.put("runner/main.ond", source);
    let uri = uri_from_path(&f.root.join("main.ond").canonicalize().unwrap()).unwrap();
    let mut state = ServerState::default();
    state
        .open_document(uri.clone(), source.into(), &mut Vec::new())
        .unwrap();
    let column = source.lines().nth(2).unwrap().find("TestApi").unwrap() as u32;
    let has_binding = |tokens: Vec<u32>| {
        let (mut line, mut start) = (0, 0);
        tokens.chunks_exact(5).any(|token| {
            line += token[0];
            start = if token[0] == 0 {
                start + token[1]
            } else {
                token[1]
            };
            line == 2 && start == column
        })
    };
    assert!(has_binding(state.semantic_tokens(&uri).unwrap()));
    state
        .completions(&uri, 1, 10)
        .expect("valid graph import-path completion control");
    f.runner(&RUNNER.replace("final=true", "final=false"));
    let error = state
        .completions(&uri, 1, 10)
        .expect_err("import-path completion must observe the invalid dependency graph");
    assert!(error.contains("role"), "{error}");
    assert!(
        !has_binding(state.semantic_tokens(&uri).unwrap()),
        "ROLE-07 import-path errors must invalidate the old role environment"
    );
    f.runner(RUNNER);
    state
        .completions(&uri, 1, 10)
        .expect("import-path completion recovers after manifest repair");
    state.validate_uri(&uri, &mut Vec::new()).unwrap();
    assert!(has_binding(state.semantic_tokens(&uri).unwrap()));
}

#[test]
fn role_07_successful_import_path_completion_refreshes_role_bindings() {
    let f = ProjectFixture::new();
    let source = "package main\nimport \"child\"\nfunc main(){_=child.TestApi()}\n";
    f.put("runner/main.ond", source);
    let uri = uri_from_path(&f.root.join("main.ond").canonicalize().unwrap()).unwrap();
    let mut state = ServerState::default();
    state
        .open_document(uri.clone(), source.into(), &mut Vec::new())
        .unwrap();
    let column = source.lines().nth(2).unwrap().find("TestApi").unwrap() as u32;
    let has_binding = |tokens: Vec<u32>| {
        let (mut line, mut start) = (0, 0);
        tokens.chunks_exact(5).any(|token| {
            line += token[0];
            start = if token[0] == 0 {
                start + token[1]
            } else {
                token[1]
            };
            line == 2 && start == column
        })
    };
    assert!(has_binding(state.semantic_tokens(&uri).unwrap()));
    state
        .completions(&uri, 1, 10)
        .expect("warm named-role import completion");
    f.runner(&RUNNER.replace(",role=\"Test\"", ""));
    state
        .completions(&uri, 1, 10)
        .expect("common dependency selection remains a valid graph");
    assert!(
        !has_binding(state.semantic_tokens(&uri).unwrap()),
        "ROLE-07 successful import-path completion must refresh to the common dependency API"
    );
    f.runner(RUNNER);
    state
        .completions(&uri, 1, 10)
        .expect("named-role dependency selection restored");
    assert!(
        has_binding(state.semantic_tokens(&uri).unwrap()),
        "ROLE-07 successful import-path completion must refresh the restored named-role API"
    );
}
