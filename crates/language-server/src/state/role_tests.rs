//! ROLE-04、05、07: カーソルの実効対象集合とtest文脈を使う補完・名前解決。
use super::ServerState;
use crate::util::uri_from_path;
mod dependencies;
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ond-role-lsp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let f = Self(root.canonicalize().unwrap());
        f.manifest();
        f
    }
    fn put(&self, name: &str, text: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn manifest(&self) {
        self.put(
            "ond.toml",
            "[project]\nname=\"game\"\n[roles]\nnames=[\"Debug\",\"Development\",\"Release\"]\n",
        );
    }
    fn open(&self, state: &mut ServerState, name: &str, source: &str) -> String {
        self.put(name, source);
        let uri = uri_from_path(&self.0.join(name)).unwrap();
        state
            .open_document(uri.clone(), source.into(), &mut Vec::new())
            .unwrap();
        uri
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SOURCE: &str = "package main\nfunc shared() {}\nrole Debug { func debugOnly() {} }\nrole Debug, Development { func commonDev() {} }\nrole Debug, Development, Release { func allRoles() {} }\nfunc main() {\n    // ordinary\n    ;\n    role Debug {\n        // debug\n        ;\n    }\n    role Debug, Development {\n        // multi\n        ;\n    }\n    role Release {\n        // release\n        ;\n    }\n}\n";

fn cursor(source: &str, marker: &str) -> (u32, u32) {
    let mut lines = source.lines().enumerate();
    lines.find(|(_, line)| line.contains(marker)).unwrap();
    let (line, text) = lines.next().unwrap();
    (line as u32, text.find(';').unwrap() as u32)
}

#[test]
fn role_05_candidates_use_entire_cursor_context_independently_of_optional_selection() {
    let fixture = Fixture::new();
    let mut state = ServerState::default();
    let uri = fixture.open(&mut state, "main.ond", SOURCE);
    let observed = ["shared", "debugOnly", "commonDev", "allRoles"];
    for selected in [None, Some("Debug"), Some("Release")] {
        state.validate_uri(&uri, &mut Vec::new()).unwrap();
        let mut snapshot = state.load_current_project(&fixture.0).unwrap();
        if let Some(selected) = selected {
            snapshot
                .sources
                .select_role(&snapshot.root.clone(), selected)
                .unwrap();
        }
        // 順序を固定し、直前の候補cacheが異なる文脈へ流用されないことも確認する。
        for (id, marker, expected) in [
            (
                "ROLE-05.debug/ROLE-07.lsp-context",
                "// debug",
                vec!["shared", "debugOnly", "commonDev", "allRoles"],
            ),
            (
                "ROLE-05.multi",
                "// multi",
                vec!["shared", "commonDev", "allRoles"],
            ),
            ("ROLE-05.ordinary", "// ordinary", vec!["shared"]),
            ("ROLE-05.release", "// release", vec!["shared", "allRoles"]),
        ] {
            let (line, column) = cursor(SOURCE, marker);
            let actual = state
                .completions(&uri, line, column)
                .unwrap()
                .iter()
                .filter_map(|item| item["label"].as_str())
                .filter(|name| observed.contains(name))
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                actual,
                expected.iter().map(|name| (*name).to_owned()).collect(),
                "{id}, selected={selected:?}"
            );
            let index = crate::symbol_index::ProjectSymbolIndex::refresh(snapshot.clone(), None)
                .for_position(&fixture.0.join("main.ond"), line, column);
            let indexed = index
                .package_symbols(".")
                .iter()
                .filter(|symbol| observed.contains(&symbol.name.as_str()))
                .map(|symbol| symbol.name.clone())
                .collect::<BTreeSet<_>>();
            assert_eq!(
                indexed,
                expected.into_iter().map(str::to_owned).collect(),
                "{id}: snapshot selected={selected:?}"
            );
        }
    }
}

#[test]
fn role_02_empty_intersection_has_no_declaration_candidates() {
    let fixture = Fixture::new();
    let mut state = ServerState::default();
    let source = "package main\nfunc shared(){}\nrole Debug {\n    role Release {\n        func impossible(){\n            // empty\n            ;\n        }\n    }\n}\nfunc main(){}\n";
    let uri = fixture.open(&mut state, "main.ond", source);
    let (line, column) = cursor(source, "// empty");
    let completions = state.completions(&uri, line, column).unwrap();
    for name in ["shared", "impossible", "main"] {
        assert!(
            !completions.iter().any(|item| item["label"] == name),
            "ROLE-02.empty-intersection: {name}: {completions:?}"
        );
    }
}

#[test]
fn role_04_conditional_import_alias_and_definition_obey_cursor_context() {
    let fixture = Fixture::new();
    fixture.put(
        "tools/t.ond",
        "package tools\nfunc Value()->i32{return 7}\n",
    );
    let source = "package main\nrole Debug { import \"game/tools\" }\nrole Debug { func invoke()->i32 { return tools.Value() } }\nfunc ordinary(){\n    // ordinary\n    ;\n}\nfunc main(){}\n";
    let mut state = ServerState::default();
    let uri = fixture.open(&mut state, "main.ond", source);
    let line = source.lines().nth(2).unwrap();
    let column = line.find("tools.Value").unwrap() as u32;
    assert!(
        state
            .completions(&uri, 2, column)
            .unwrap()
            .iter()
            .any(|item| item["label"] == "tools"),
        "ROLE-04.import-visibility"
    );
    let definition = state.definition(&uri, 2, column + 6).unwrap();
    assert!(
        definition[0]["targetUri"]
            .as_str()
            .is_some_and(|uri| uri.ends_with("/t.ond")),
        "ROLE-04.import-visibility: {definition}"
    );
    let (line, column) = cursor(source, "// ordinary");
    assert!(
        !state
            .completions(&uri, line, column)
            .unwrap()
            .iter()
            .any(|item| item["label"] == "tools"),
        "ROLE-04.import-hidden"
    );
}

#[test]
fn role_05_common_names_are_visible_across_all_source_filenames() {
    let fixture = Fixture::new();
    fixture.put(
        "ond.toml",
        "[project]\nname=\"library\"\n[roles]\nnames=[\"Internal\",\"Public\"]\n",
    );
    let normal = "package library\nfunc ordinary(){\n    // normal\n    ;\n}\n";
    let tests = "package library\nfunc testHelper(){}\nfunc TestCommon(){\n    // common-testing\n    ;\n}\nrole Public {func TestPublic(){\n    // role-testing\n    ;\n}}\n";
    fixture.put("lib_test.ond", tests);
    let mut state = ServerState::default();
    let normal_uri = fixture.open(&mut state, "lib.ond", normal);
    let test_uri = fixture.open(&mut state, "lib_test.ond", tests);
    for (uri, source, marker, visible) in [
        (&normal_uri, normal, "// normal", true),
        (&test_uri, tests, "// common-testing", true),
        (&test_uri, tests, "// role-testing", true),
    ] {
        let (line, column) = cursor(source, marker);
        let completions = state.completions(uri, line, column).unwrap();
        assert_eq!(
            completions.iter().any(|item| item["label"] == "testHelper"),
            visible,
            "ROLE-05 common source visibility: {completions:?}"
        );
    }
}

#[test]
fn role_05_method_members_follow_role_and_subset_visibility() {
    let fixture = Fixture::new();
    let source = "package main\ntype S struct {}\nfunc (s:S) Shared(){}\nrole Debug {func (s:S) DebugOnly(){}}\nrole Debug, Development {func (s:S) CommonDev(){}}\nfunc ordinary(s:S){s.Shared()}\nrole Debug {func debug(s:S){s.Shared()}}\nrole Debug, Development {func multi(s:S){s.Shared()}}\nfunc main(){}\n";
    let mut state = ServerState::default();
    let uri = fixture.open(&mut state, "main.ond", source);
    {
        state.validate_uri(&uri, &mut Vec::new()).unwrap();
        for (line, expected) in [
            (5, vec!["Shared"]),
            (6, vec!["Shared", "DebugOnly", "CommonDev"]),
            (7, vec!["Shared", "CommonDev"]),
        ] {
            let text = source.lines().nth(line).unwrap();
            let column = text.rfind('.').unwrap() as u32 + 1;
            let actual = state
                .completions(&uri, line as u32, column)
                .unwrap()
                .iter()
                .filter_map(|item| item["label"].as_str())
                .filter(|label| ["Shared", "DebugOnly", "CommonDev"].contains(label))
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            assert_eq!(
                actual,
                expected.into_iter().map(str::to_owned).collect(),
                "ROLE-05 method members, line={line}, common build"
            );
        }
    }
}

#[test]
fn role_04_references_and_rename_keep_disjoint_bindings_separate() {
    let fixture = Fixture::new();
    let source = "package main\nrole Debug {\n    func chosen(){}\n    func debugCall(){chosen()}\n}\nrole Release {\n    func chosen(){}\n    func releaseCall(){chosen()}\n}\nfunc main(){}\n";
    let mut state = ServerState::default();
    let uri = fixture.open(&mut state, "main.ond", source);
    for (line, declaration_line) in [(3, 2), (7, 6)] {
        let text = source.lines().nth(line).unwrap();
        let column = text.find("chosen()").unwrap() as u32;
        let references = state.references(&uri, line as u32, column, true).unwrap();
        assert_eq!(
            references.as_array().unwrap().len(),
            2,
            "ROLE-04 disjoint references: {references}"
        );
        let lines = references
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["range"]["start"]["line"].as_u64().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(lines, [line as u64, declaration_line].into_iter().collect());
        let definition = state.definition(&uri, line as u32, column).unwrap();
        assert_eq!(
            definition[0]["targetSelectionRange"]["start"]["line"],
            declaration_line
        );
        let rename = state.rename(&uri, line as u32, column, "picked").unwrap();
        assert_eq!(
            rename["changes"][&uri].as_array().unwrap().len(),
            2,
            "ROLE-04 disjoint rename: {rename}"
        );
        if line == 3 {
            assert!(
                state
                    .rename(&uri, line as u32, column, "releaseCall")
                    .is_ok(),
                "ROLE-04 disjoint declarations may share a renamed binding name"
            );
        }
    }
}

#[test]
fn role_04_roleless_caller_resolves_only_explicit_dependency_api() {
    let fixture = Fixture::new();
    fixture.put(
        "ond.toml",
        "[project]\nname=\"game\"\nfinal=true\n[dependencies]\nsupport={path=\"library\",role=\"Public\"}\n",
    );
    fixture.put(
        "library/ond.toml",
        "[project]\nname=\"library\"\n[roles]\nnames=[\"Internal\",\"Public\"]\n",
    );
    fixture.put("library/lib.ond", "package library\nrole Internal {\nfunc Value()->i32{return 11}\n}\nrole Public {\nfunc Value()->i32{return 22}\n}\n");
    let source = "package main\nimport \"library\"\nfunc main(){_=library.Value()}\n";
    let mut state = ServerState::default();
    let uri = fixture.open(&mut state, "main.ond", source);
    let column = source.lines().nth(2).unwrap().find("Value").unwrap() as u32;
    let definition = state.definition(&uri, 2, column).unwrap();
    assert_eq!(definition[0]["targetSelectionRange"]["start"]["line"], 5);
    let references = state.references(&uri, 2, column, true).unwrap();
    assert_eq!(
        references.as_array().unwrap().len(),
        2,
        "ROLE-04 selected dependency: {references}"
    );
    assert!(
        references
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["uri"] == uri)
    );
}

#[test]
fn role_07_invalid_role_edit_does_not_reuse_a_different_context_index() {
    let fixture = Fixture::new();
    let mut state = ServerState::default();
    let original = "package main\nfunc stale(){}\nfunc main(){}\n";
    let uri = fixture.open(&mut state, "main.ond", original);
    let invalid = "package main\nrole Debgu {func stale(){}}\nfunc main(){}\n";
    state
        .change_document(
            uri.clone(),
            vec![crate::protocol::TextDocumentContentChangeEvent {
                text: invalid.into(),
                range: None,
            }],
            &mut Vec::new(),
        )
        .unwrap();
    let completions = state.completions(&uri, 2, 12).unwrap();
    assert!(
        !completions.iter().any(|item| item["label"] == "stale"),
        "ROLE-07 stale role index: {completions:?}"
    );
}
