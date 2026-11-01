//! ROLE-01〜03、07: 現行のnames-only設定と公開CLI契約。
use ond_protocol::{CompilationManifest, PackageArtifact};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
#[path = "roles/dependencies.rs"]
mod dependency_roles;
const ROLES: &str = "[roles]\nnames=[\"Debug\",\"Development\",\"Release\"]\n";
struct Fixture(PathBuf);
impl Fixture {
    fn new(roles: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ond-role-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let f = Self(root);
        f.manifest(roles);
        f.put("main.ond", "package main\nfunc main(){}\n");
        f
    }
    fn put(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn manifest(&self, roles: &str) {
        self.put("ond.toml", &format!("[project]\nname=\"game\"\n{roles}"));
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ond"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
    fn compile(&self, args: &[&str]) -> Output {
        let mut command = vec!["compile", ".", "-o", "result.ondbuild"];
        command.extend_from_slice(args);
        self.run(&command)
    }
    fn accept(&self, args: &[&str]) -> CompilationManifest {
        let output = self.compile(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&fs::read(self.0.join("result.ondbuild")).unwrap()).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn role_01_manifest_closed_schema_and_identifiers() {
    for (id, settings) in [
        ("ROLE-01.empty", "names=[]"),
        ("ROLE-01.duplicate", "names=[\"Debug\",\"Debug\"]"),
        ("ROLE-01.no-names", ""),
        ("ROLE-01.invalid-identifier", "names=[\"Debug-build\"]"),
        ("ROLE-01.identifier-whitespace", "names=[\" Debug \"]"),
        ("ROLE-01.keyword", "names=[\"if\"]"),
        (
            "ROLE-01.unknown-key",
            "names=[\"Debug\"]\ndefault=\"Debug\"",
        ),
        ("ROLE-01.names-type", "names=\"Debug\""),
        ("ROLE-01.element-type", "names=[1]"),
        (
            "build/role-configuration.closed-schema",
            "names=[\"Debug\"]\nprimary=\"Debug\"",
        ),
    ] {
        let f = Fixture::new(ROLES);
        f.accept(&[]);
        let before = fs::read(f.0.join("result.ondbuild")).unwrap();
        f.manifest(&format!("[roles]\n{settings}\n"));
        let output = f.compile(&[]);
        assert_eq!(output.status.code(), Some(1), "{id}: {output:?}");
        assert!(!output.stderr.is_empty(), "{id}");
        assert_eq!(
            fs::read(f.0.join("result.ondbuild")).unwrap(),
            before,
            "{id}: preserve successful output"
        );
    }
}

#[test]
fn role_01_primary_key_is_outside_the_closed_roles_schema() {
    let f = Fixture::new("[roles]\nnames=[\"Debug\"]\nprimary=\"Debug\"\n");
    let output = f.compile(&[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "build/role-configuration: {output:?}"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("primary"));
}
#[test]
fn role_01_names_only_and_unconfigured_and_case_sensitive() {
    let f = Fixture::new("");
    f.accept(&[]);
    assert_eq!(
        f.compile(&["--role", "Debug"]).status.code(),
        Some(1),
        "ROLE-01.unconfigured-option"
    );
    f.put("main.ond", "package main\nrole Debug {}\nfunc main(){}\n");
    assert_eq!(
        f.compile(&[]).status.code(),
        Some(1),
        "ROLE-03.unconfigured-syntax"
    );
    for names in ["[\"Debug\"]", "[\"Debug\",\"debug\"]"] {
        f.manifest(&format!("[roles]\nnames={names}\n"));
        f.accept(&[]);
        f.accept(&["--role", "Debug"]);
    }
}
#[test]
fn role_01_option_errors_have_distinct_exit_codes() {
    let f = Fixture::new(ROLES);
    f.accept(&[]);
    let previous = fs::read(f.0.join("result.ondbuild")).unwrap();
    for (id, args, status) in [
        ("ROLE-01.unknown-option-value", vec!["--role", "debug"], 1),
        (
            "ROLE-01.repeated-option",
            vec!["--role", "Debug", "--role", "Release"],
            2,
        ),
        ("ROLE-01.missing-option-value", vec!["--role"], 2),
    ] {
        let output = f.compile(&args);
        assert_eq!(output.status.code(), Some(status), "{id}: {output:?}");
        assert!(!output.stderr.is_empty());
        assert_eq!(fs::read(f.0.join("result.ondbuild")).unwrap(), previous);
    }
    assert_eq!(
        f.run(&["link", "--role", "Debug"]).status.code(),
        Some(2),
        "ROLE-01.link-option"
    );
}
#[test]
fn role_01_default_override_and_role_07_three_selected_hashes() {
    let f = Fixture::new(ROLES);
    let common = f.accept(&[]);
    let debug = f.accept(&["--role", "Debug"]);
    let release = f.accept(&["--role", "Release"]);
    let ids = [
        &common.packages[0].build_id,
        &debug.packages[0].build_id,
        &release.packages[0].build_id,
    ];
    assert_eq!(
        ids.into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3,
        "ROLE-07.selected-hash"
    );
    assert_eq!(
        debug.packages[0].build_id,
        f.accept(&["--role", "Debug"]).packages[0].build_id
    );
    assert_eq!(
        common.packages[0].build_id,
        f.accept(&[]).packages[0].build_id
    );
    f.put(
        "main.ond",
        "package main\nrole Debug {func f(){missing()}}\nfunc main(){}\n",
    );
    assert_eq!(
        f.compile(&["--role", "Debug"]).status.code(),
        Some(1),
        "ROLE-07.failure-cache"
    );
    f.accept(&[]);
    f.accept(&["--role", "Release"]);
}
#[test]
fn role_02_library_option_remains_an_argument_error() {
    let f = Fixture::new("");
    fs::remove_file(f.0.join("main.ond")).unwrap();
    f.put(
        "ond.toml",
        "[project]\nname=\"library\"\n[roles]\nnames=[\"Internal\",\"Public\"]\n",
    );
    f.put(
        "lib_test.ond",
        "package library\nfunc TestPublic(){}\nrole Internal {func TestInternal(){}}\n",
    );
    assert_eq!(
        f.compile(&["--role", "Public"]).status.code(),
        Some(2),
        "ROLE-02.library-option"
    );
}
#[test]
fn role_02_common_source_and_named_additions_have_exact_symbols() {
    let f = Fixture::new(ROLES);
    f.put("main.ond","package main\nfunc CommonMarker(){}\nrole Debug {func DebugMarker(){}}\nrole Release {func ReleaseMarker(){}}\nfunc main(){}\n");
    for (args, debug, release) in [
        (vec![], false, false),
        (vec!["--role", "Debug"], true, false),
        (vec!["--role", "Release"], false, true),
    ] {
        let manifest = f.accept(&args);
        let artifact: PackageArtifact =
            serde_json::from_slice(&fs::read(&manifest.packages[0].path).unwrap()).unwrap();
        let has = |name: &str| {
            artifact
                .object
                .symbols
                .iter()
                .any(|symbol| symbol.name == name)
        };
        assert!(has("main.CommonMarker"));
        assert_eq!(has("main.DebugMarker"), debug);
        assert_eq!(has("main.ReleaseMarker"), release);
    }
}
#[test]
fn role_03_override_controls_external_import_reachability() {
    let f = Fixture::new(ROLES);
    f.manifest(&format!(
        "{ROLES}[dependencies]\nsupport={{path=\"library\"}}\n"
    ));
    f.put("library/ond.toml", "[project]\nname=\"library\"\n");
    f.put(
        "library/lib.ond",
        "package library\nfunc Value()->i32{return 7}\n",
    );
    f.put("main.ond","package main\nrole Debug {import \"library\"}\nfunc main(){role Debug {_=library.Value()}}\n");
    for (args, imported) in [
        (vec![], false),
        (vec!["--role", "Debug"], true),
        (vec!["--role", "Release"], false),
    ] {
        assert_eq!(
            f.accept(&args)
                .packages
                .iter()
                .any(|package| package.package == "library"),
            imported,
            "ROLE-03.import-off-primary/off/on"
        );
    }
}
#[test]
fn role_01_invalid_configuration_without_source_is_not_ignored() {
    let f = Fixture::new("[roles]\nnames=[]\n");
    fs::remove_file(f.0.join("main.ond")).unwrap();
    let output = f.compile(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("role"));
}
