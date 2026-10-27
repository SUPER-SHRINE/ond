//! ROLE-02〜04、06: specs/ond/conformance/cases/roles.mdから導いた観測検査。
use crate::{
    Compilation,
    memory_test_vm::{execute, execute_with_global},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf, Option<String>);
impl Fixture {
    fn new(selected: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ond-role-core-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let fixture = Self(root, (!selected.is_empty()).then(|| selected.into()));
        fixture.put("ond.toml", "[project]\nname = \"game\"\n[roles]\nnames = [\"Debug\", \"Development\", \"Release\"]\n");
        fixture
    }
    fn put(&self, name: &str, text: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn compile(&self, source: &str) -> Result<Compilation, crate::FrontendError> {
        self.put("main.ond", source);
        let mut loaded = crate::load_project(&self.0)?;
        if let Some(selected) = &self.1 {
            loaded
                .sources
                .select_role(&loaded.root, selected)
                .map_err(|message| {
                    crate::FrontendError::new(vec![crate::Diagnostic::error(
                        crate::source::Span::synthetic(),
                        message,
                    )])
                })?;
        }
        crate::compile_loaded_project(loaded)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn case(id: &str, selected: &str, items: &str, accept: bool, token: Option<&str>) {
    let fixture = Fixture::new(selected);
    // 拒否検査の前提として、同じ設定とrole構文の成功経路を確認する。
    fixture
        .compile("package main\nrole Debug { func available(){} }\nfunc main(){}\n")
        .expect("role syntax control must compile");
    let source = format!("package main\n{items}\nfunc main(){{}}\n");
    let result = fixture.compile(&source);
    if accept {
        result.unwrap_or_else(|error| panic!("{id}: {error:?}"));
    } else {
        let error = result.expect_err(id);
        if let Some(token) = token {
            let offset = source.rfind(token).unwrap();
            assert!(
                error.diagnostics().iter().any(|diagnostic| {
                    let span = diagnostic.primary.span;
                    !span.is_synthetic() && span.start < offset + token.len() && span.end >= offset
                }),
                "{id}: expected original position of {token}: {error:?}"
            );
        }
    }
}

macro_rules! role_case {
    ($name:ident, $id:literal, $selected:literal, $source:literal, $accept:literal, $token:expr) => {
        #[test]
        fn $name() {
            case($id, $selected, $source, $accept, $token);
        }
    };
}
role_case!(
    unknown_name_off,
    "ROLE-03.unknown-name-off",
    "",
    "role Debug { func f(){ missing() } }",
    true,
    None
);
role_case!(
    unknown_name_on,
    "ROLE-03.unknown-name-on",
    "Debug",
    "role Debug { func f(){ missing() } }",
    false,
    Some("missing")
);
role_case!(
    type_off,
    "ROLE-03.type-off",
    "",
    "role Debug { var Wrong: bool = 1 }",
    true,
    None
);
role_case!(
    type_on,
    "ROLE-03.type-on",
    "Debug",
    "role Debug { var Wrong: bool = 1 }",
    false,
    Some("1")
);
role_case!(
    constant_off,
    "ROLE-03.constant-off",
    "",
    "role Debug { const Wide: u8 = 256 }",
    true,
    None
);
role_case!(
    constant_on,
    "ROLE-03.constant-on",
    "Debug",
    "role Debug { const Wide: u8 = 256 }",
    false,
    Some("256")
);
role_case!(
    syntax_off,
    "ROLE-03.syntax-off",
    "",
    "role Debug { var x: = 1 }",
    false,
    Some("=")
);
role_case!(
    lexical_off,
    "ROLE-03.lexical-off",
    "",
    "role Debug { const x = 0xQ }",
    false,
    Some("0xQ")
);
role_case!(
    typo_off,
    "ROLE-03.typo-off",
    "",
    "role Debug { role Debgu { func f(){} } }",
    false,
    Some("Debgu")
);
role_case!(
    typo_empty,
    "ROLE-03.typo-empty",
    "",
    "role Debug { role Release { role Debgu {} } }",
    false,
    Some("Debgu")
);
role_case!(
    empty_list,
    "ROLE-03.empty-list",
    "",
    "role {}",
    false,
    Some("role {}")
);
role_case!(
    repeated_name,
    "ROLE-03.repeated-name",
    "",
    "role Debug, Debug {}",
    false,
    Some("Debug")
);
role_case!(
    trailing_comma,
    "ROLE-03.trailing-comma",
    "",
    "role Debug, {}",
    false,
    Some("role Debug, {}")
);
role_case!(
    package_inside,
    "ROLE-03.package-inside",
    "",
    "role Debug { package main }",
    false,
    Some("package main")
);
role_case!(
    contextual_name,
    "ROLE-03.contextual-name",
    "",
    "func role(){}; func call(){role()}",
    true,
    None
);
role_case!(
    normal_hidden,
    "ROLE-04.normal-hidden",
    "Debug",
    "role Debug {func cheat(){}}; func ordinary(){cheat()}",
    false,
    Some("cheat")
);
role_case!(
    all_hidden,
    "ROLE-04.all-hidden",
    "Debug",
    "role Debug, Development, Release {func cheat(){}}; func ordinary(){cheat()}",
    false,
    Some("cheat")
);
role_case!(
    subset,
    "ROLE-04.subset",
    "Debug",
    "role Debug, Development {func sharedRole(){}}; role Debug {func call(){sharedRole()}}",
    true,
    None
);
role_case!(
    not_subset,
    "ROLE-04.not-subset",
    "Debug",
    "role Debug {func only(){}}; role Debug, Development {func call(){only()}}",
    false,
    Some("only")
);
role_case!(
    disjoint_name,
    "ROLE-04.disjoint-name",
    "Debug",
    "role Debug {func f(){}}; role Development {func f(){}}",
    true,
    None
);
role_case!(
    overlap_name,
    "ROLE-04.overlap-name",
    "Debug",
    "role Debug {func f(){}}; role Debug, Development {func f(){}}",
    false,
    None
);
role_case!(
    no_binding_union,
    "ROLE-04.no-binding-union",
    "Debug",
    "role Debug {func f(){}}; role Development {func f(){}}; role Debug, Development {func call(){f()}}",
    false,
    Some("f()")
);
role_case!(
    inherited_body,
    "ROLE-04.inherited-body",
    "Debug",
    "role Debug {func only(){ };func call(){only()}}",
    true,
    None
);
role_case!(
    definition_context,
    "ROLE-04.definition-context",
    "Debug",
    "role Debug {func only(){}}; func generic[T](v:T){only()}; role Debug {func call(){generic[i32](1)}}",
    false,
    Some("only()")
);
role_case!(
    local_hidden,
    "ROLE-04.local-hidden",
    "Debug",
    "func f(){role Debug {var x:i32=1}; x=2}",
    false,
    Some("x=2")
);
role_case!(
    local_shadow,
    "ROLE-04.local-shadow",
    "Debug",
    "func f(){var x:i32=1; role Debug {var x:i32=2; x=3}; x=4}",
    true,
    None
);

const SELECTION: &str = "package main\nvar Result: i32\nrole Debug, Development {\n    func cheat() -> i32 { return 99 }\n    const Payload = [3]u8{17, 23, 31}\n    var CheatData: i32 = 44\n}\nfunc probe() -> i32 {\n    role Debug, Development {\n        role Debug { return cheat() }\n        role Development { return 2 }\n    }\n    role Release { return 3 }\n    return 0\n}\nfunc main() { Result = probe() }\n";

#[test]
fn non_function_bindings_and_members_obey_role_visibility() {
    // ROLE-04、language/role-visibility: 宣言種別によらず同じ候補条件を適用する。
    for (kind, ordinary, declaration, operation) in [
        ("type", "", "type Hidden struct {}", "var value: Hidden"),
        ("constant", "", "const Secret:i32=7", "_=Secret"),
        ("global", "", "var Secret:i32=7", "Secret=8"),
        (
            "method",
            "type S struct {}",
            "func (s:S) Hidden(){}",
            "var s:S; s.Hidden()",
        ),
        (
            "operator",
            "type S struct {}",
            "operator (s:S) + (other:S)->S{return s}",
            "var s:S; _=s+s",
        ),
    ] {
        let fixture = Fixture::new("Debug");
        let definitions = format!("package main\n{ordinary}\nrole Debug {{{declaration}}}\n");
        fixture
            .compile(&format!(
                "{definitions}role Debug {{func use(){{{operation}}}}}\nfunc main(){{}}\n"
            ))
            .unwrap_or_else(|error| panic!("ROLE-04 {kind} role-visible control: {error:?}"));
        assert!(
            fixture
                .compile(&format!(
                    "{definitions}func use(){{{operation}}}\nfunc main(){{}}\n"
                ))
                .is_err(),
            "ROLE-04 {kind} must be hidden from ordinary context"
        );
        assert!(fixture.compile(&format!("{definitions}role Debug, Development {{func use(){{{operation}}}}}\nfunc main(){{}}\n")).is_err(), "ROLE-04 {kind} must not narrow multi-role context to selected Debug");
    }
}

#[test]
fn signatures_types_and_initializers_inherit_definition_context() {
    // ROLE-04、language/role-visibility: signature/initializerにもC⊆Dを要求する。
    for (kind, declaration, use_item) in [
        (
            "signature",
            "type Hidden struct {}",
            "func identity(value:Hidden)->Hidden{return value}",
        ),
        (
            "type-underlying",
            "type Hidden struct {}",
            "type Holder struct { Value:Hidden }",
        ),
        (
            "global-initializer",
            "const Secret:i32=7",
            "var Value:i32=Secret",
        ),
        (
            "constant-initializer",
            "const Secret:i32=7",
            "const Value:i32=Secret",
        ),
    ] {
        let fixture = Fixture::new("Debug");
        let definitions = format!("package main\nrole Debug {{{declaration}}}\n");
        fixture
            .compile(&format!(
                "{definitions}role Debug {{{use_item}}}\nfunc main(){{}}\n"
            ))
            .unwrap_or_else(|error| panic!("ROLE-04 {kind} role-visible control: {error:?}"));
        for context in ["ordinary", "Debug, Development"] {
            let item = if context == "ordinary" {
                use_item.into()
            } else {
                format!("role {context} {{{use_item}}}")
            };
            assert!(
                fixture
                    .compile(&format!("{definitions}{item}\nfunc main(){{}}\n"))
                    .is_err(),
                "ROLE-04 {kind}: reference context {context}"
            );
        }
    }
}

#[test]
fn role_declaration_blocks_preserve_source_item_separator_grammar() {
    let fixture = Fixture::new("Debug");
    fixture
        .compile("package main\nrole Debug {}\nfunc main(){}\n")
        .unwrap();
    for source in [
        "package main\n;\nfunc main(){}\n",
        "package main\nrole Debug {;}\nfunc main(){}\n",
    ] {
        assert!(
            fixture.compile(source).is_err(),
            "SourceItem requires a declaration or import: {source}"
        );
    }
}

role_case!(
    unused_generic_body_keeps_definition_context,
    "ROLE-04.definition-context.unused-generic",
    "Debug",
    "role Debug {func only(){}}; func unused[T](value:T){only()}",
    false,
    Some("only()")
);

#[test]
fn or_nested_common_and_empty_intersection_execute() {
    for (selected, expected) in [("", 0), ("Debug", 99), ("Development", 2), ("Release", 3)] {
        let fixture = Fixture::new(selected);
        for empty in [false, true] {
            let source = if empty {
                SELECTION.replace(
                    "    func cheat",
                    "    role Release { func impossible(){ missing() } }\n    func cheat",
                )
            } else {
                SELECTION.into()
            };
            let compilation = fixture.compile(&source).unwrap();
            assert_eq!(
                execute_with_global(&compilation.mir, "main.main", "main.Result").unwrap(),
                [expected],
                "ROLE-02 {selected}, empty={empty}"
            );
        }
    }
}

#[test]
fn conditional_import_resolution_and_every_projection_order() {
    for selected in ["", "Debug", "Release"] {
        let fixture = Fixture::new(selected);
        fixture
            .compile("package main\nrole Debug {}\nfunc main(){}\n")
            .unwrap();
        let source = "package main\nrole Debug { import \"missing/tools\" }\nfunc main(){}\n";
        assert_eq!(
            fixture.compile(source).is_ok(),
            selected != "Debug",
            "ROLE-03.import {selected}"
        );
        assert!(
            fixture
                .compile("package main\nfunc main(){}\nrole Debug { import \"missing/tools\" }\n")
                .is_err(),
            "ROLE-03.import-order {selected}"
        );
    }
}

#[test]
fn aliases_and_package_qualified_names_keep_role_context() {
    let fixture = Fixture::new("Debug");
    fixture.put(
        "tools/t.ond",
        "package tools\nfunc Value()->i32{return 7}\nrole Debug { func Hidden(){} }\n",
    );
    let source = "package main\nrole Debug { import \"game/tools\" }\nrole Debug { func invoke()->i32{return tools.Value()} }\nfunc main(){}\n";
    let compilation = fixture.compile(source).unwrap();
    assert_eq!(execute(&compilation.mir, "main.invoke", &[]).unwrap(), [7]);
    assert!(
        fixture
            .compile(&format!("{source}func ordinary(){{tools.Value()}}\n"))
            .is_err(),
        "ROLE-04.import-hidden"
    );
    for (call, accept) in [
        ("tools.Hidden()", false),
        ("role Debug { tools.Hidden() }", true),
    ] {
        assert_eq!(
            fixture
                .compile(&format!(
                    "package main\nimport \"game/tools\"\nfunc main(){{{call}}}\n"
                ))
                .is_ok(),
            accept,
            "ROLE-04.qualified {call}"
        );
    }
    for declaration in [
        "role Debug {func tools(){}}",
        "role Debug, Development {func tools(){}}",
    ] {
        assert!(
            fixture
                .compile(&format!(
                    "package main\nimport \"game/tools\"\n{declaration}\nfunc main(){{}}\n"
                ))
                .is_err(),
            "ROLE-04 import alias and overlapping declaration conflict"
        );
    }
}

#[test]
fn dependency_role_is_not_inherited_from_the_caller() {
    let fixture = Fixture::new("Debug");
    fixture.put(
        "library/ond.toml",
        "[project]\nname=\"library\"\n[roles]\nnames=[\"Internal\",\"Public\"]\n",
    );
    fixture.put(
        "library/lib.ond",
        "package library\nfunc Value[T](x:T)->i32 {role Internal {return 11};return 22}\n",
    );
    fixture.put("main.ond", "package main\nimport \"library\"\nfunc probe()->i32{return library.Value[i32](7)}\nfunc main(){}\n");
    let dependency = fixture.0.join("library").canonicalize().unwrap();
    let external = std::collections::BTreeMap::from([(
        "library/lib.ond".into(),
        crate::ExternalSource {
            path: dependency.join("lib.ond"),
            text: fs::read_to_string(dependency.join("lib.ond")).unwrap(),
        },
    )]);
    for selected in [None, Some("Debug"), Some("Release")] {
        let mut loaded = crate::load_project_with_inputs(
            &fixture.0,
            &Default::default(),
            std::slice::from_ref(&dependency),
            &Default::default(),
            &external,
        )
        .unwrap();
        if let Some(selected) = selected {
            loaded.sources.select_role(&loaded.root, selected).unwrap();
        }
        let compiled = crate::compile_loaded_project(loaded).unwrap();
        assert_eq!(
            execute(&compiled.mir, "main.probe", &[]).unwrap(),
            [22],
            "ROLE-02.dependency-primary/independent: {selected:?}"
        );
    }
}

#[test]
fn role_07_loaded_snapshot_and_semantic_cache_preserve_optional_selection() {
    let fixture = Fixture::new("");
    fixture.put("main.ond", "package main\nfunc probe()->i32{role Debug {return 99};role Release {return 3};return 0}\nfunc main(){}\n");
    let common = crate::load_project(&fixture.0).unwrap();
    let mut debug = common.clone();
    debug.sources.select_role(&debug.root, "Debug").unwrap();
    let mut release = common.clone();
    release
        .sources
        .select_role(&release.root, "Release")
        .unwrap();
    fixture.put(
        "ond.toml",
        "[project]\nname=\"game\"\n[roles]\nnames=[\"Changed\"]\n",
    );
    let mut cache = crate::SemanticCache::default();
    for (snapshot, expected) in [(common, 0), (debug, 99), (release, 3)] {
        cache.validate(&snapshot).unwrap();
        let compiled = crate::compile_loaded_project(snapshot).unwrap();
        assert_eq!(
            execute(&compiled.mir, "main.probe", &[]).unwrap(),
            [expected]
        );
    }
    fixture.put(
        "ond.toml",
        "[project]\nname=\"game\"\n[roles]\nnames=[\"Debug\",\"Release\"]\n",
    );
    fixture.put(
        "main.ond",
        "package main\nrole Debug {func f(){missing()}}\nfunc main(){}\n",
    );
    let common = crate::load_project(&fixture.0).unwrap();
    let mut debug = common.clone();
    debug.sources.select_role(&debug.root, "Debug").unwrap();
    assert!(cache.validate(&debug).is_err());
    cache.validate(&common).unwrap();
}

#[test]
fn role_04_common_bindings_are_not_overridden_and_duplicates_remain_errors() {
    case(
        "ROLE-04.common-no-override",
        "Debug",
        "func f(){};role Debug {func f(){}}",
        false,
        None,
    );
    let fixture = Fixture::new("");
    fixture.put("ond.toml", "[project]\nname=\"game\"\n");
    assert!(
        fixture
            .compile("package main\nfunc f(){};func f(){}\nfunc main(){}\n")
            .is_err(),
        "ROLE-04.common-duplicate-without-roles"
    );
}

#[test]
fn role_03_single_role_block_grammar_preserves_runtime_if_else() {
    let fixture = Fixture::new("Debug");
    fixture
        .compile(
            "package main\nfunc probe()->i32{if true {return 1} else {return 2}}\nfunc main(){}\n",
        )
        .unwrap();
    for source in [
        "package main\nrole Debug {func f(){}} else {func g(){}}\nfunc main(){}\n",
        "package main\nfunc main(){role Debug {} else {}}\n",
    ] {
        assert!(
            fixture.compile(source).is_err(),
            "language/role-grammar requires a single block: {source}"
        );
    }
}

fn unused_qualified_type_alias_case(
    kind: &str,
    targets: &str,
    reference: Option<&str>,
    accepts: bool,
) {
    let fixture = Fixture::new("Debug");
    fixture.put("tools/t.ond", "package tools\ntype Visible struct{}\n");
    let item = match kind {
        "signature" => "func unused[T](value:tools.Visible){}",
        "underlying" => "type Unused[T] struct{value:tools.Visible}",
        _ => unreachable!(),
    };
    fixture
        .compile(&format!(
            "package main\nimport \"game/tools\"\n{item}\nfunc main(){{}}\n"
        ))
        .unwrap();
    let item = reference.map_or_else(
        || item.into(),
        |context| format!("role {context} {{{item}}}"),
    );
    let source = format!(
        "package main\nrole {targets} {{import \"game/tools\"}}\n{item}\nfunc main(){{}}\n"
    );
    let result = fixture.compile(&source);
    if accepts {
        result.unwrap_or_else(|error| {
            panic!("language/role-visibility {kind}: {targets}→{reference:?}: {error:?}")
        });
    } else {
        let error = result
            .expect_err("role-limited alias must be hidden in this unused generic definition");
        let start = source.rfind("tools.Visible").unwrap();
        assert!(
            error.diagnostics().iter().any(|diagnostic| {
                let span = diagnostic.primary.span;
                !span.is_synthetic() && span.start < start + "tools".len() && span.end >= start
            }),
            "language/role-visibility must identify the qualified type alias: {error:?}"
        );
    }
}

macro_rules! unused_alias_case {
    ($name:ident,$kind:literal,$targets:literal,$reference:expr,$accepts:literal) => {
        #[test]
        fn $name() {
            unused_qualified_type_alias_case($kind, $targets, $reference, $accepts);
        }
    };
}
unused_alias_case!(
    unused_signature_alias_hidden_from_common,
    "signature",
    "Debug",
    None,
    false
);
unused_alias_case!(
    unused_signature_alias_hidden_from_multi,
    "signature",
    "Debug",
    Some("Debug, Development"),
    false
);
unused_alias_case!(
    unused_signature_alias_visible_in_same_role,
    "signature",
    "Debug",
    Some("Debug"),
    true
);
unused_alias_case!(
    unused_signature_alias_visible_in_subset,
    "signature",
    "Debug, Development",
    Some("Debug"),
    true
);
unused_alias_case!(
    unused_underlying_alias_hidden_from_common,
    "underlying",
    "Debug",
    None,
    false
);
unused_alias_case!(
    unused_underlying_alias_hidden_from_multi,
    "underlying",
    "Debug",
    Some("Debug, Development"),
    false
);
unused_alias_case!(
    unused_underlying_alias_visible_in_same_role,
    "underlying",
    "Debug",
    Some("Debug"),
    true
);
unused_alias_case!(
    unused_underlying_alias_visible_in_subset,
    "underlying",
    "Debug, Development",
    Some("Debug"),
    true
);
