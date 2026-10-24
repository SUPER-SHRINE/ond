use ond_protocol::{CompilationManifest, debug::DebugInfo, executable::Executable};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
struct Fixture(PathBuf);

#[test]
fn large_exact_literals_do_not_exhaust_the_compiler_stack() {
    let f = Fixture::new();
    for zeros in [256, 512, 1000] {
        let number = format!("1{}", "0".repeat(zeros));
        f.put(
            "game/main.ond",
            &format!("package main\nconst X:i32={number}-{number}\nfunc main(){{}}\n"),
        );
        let output = run(&f.0, &["compile", "game", "-o", "-"], None);
        assert!(
            output.status.success(),
            "{zeros}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(manifest.packages.len(), 1);
    }
}

#[test]
fn public_json_outputs_use_canonical_bytes() {
    let f = Fixture::new();
    f.put("game/main.ond", "package main\nfunc main(){}\n");
    f.put("game/main_test.ond", "package main\nfunc TestRoot(){}\n");
    let compile = run(&f.0, &["compile", "game", "--test", "-o", "-"], None);
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let canonical = |bytes: &[u8]| {
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&value).unwrap());
    };
    canonical(&compile.stdout);
    let link = run(
        &f.0,
        &["link", "-", "-o", "image.bin"],
        Some(&compile.stdout),
    );
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    Executable::read(&fs::read(f.0.join("image.bin")).unwrap()).unwrap();
    for path in ["image.onddebug", "image.ondtest"] {
        canonical(&fs::read(f.0.join(path)).unwrap());
    }
}
#[test]
fn no_arguments_prints_help_and_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_ond"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.starts_with("Usage:"));
    assert!(help.contains("ond compile") && help.contains("ond link"));
}

impl Fixture {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "ond-cli process {} {} {}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        let f = Self(p);
        f.put("ond.toml", LINK_SETTINGS);

        f.put(
            "game/main.ond",
            "package main\nimport \"math\"\nfunc main(){var _=math.Value()}",
        );
        f.put(
            "libraries/math/value.ond",
            "package math\nfunc Value()->i32{return 7}",
        );
        f.put("libraries/math/ond.toml", "[project]\nname = \"math\"\n");
        f.put(
            "game/ond.toml",
            "[project]\nname = \"game\"\n[dependencies]\nmath={path=\"../libraries/math\"}\n",
        );
        f
    }
    fn put(&self, p: &str, s: &str) {
        let p = self.0.join(p);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, s).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(root: &Path, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ond"))
        .current_dir(root)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(bytes) = input {
        child.stdin.take().unwrap().write_all(bytes).unwrap();
    }
    child.wait_with_output().unwrap()
}

#[test]
fn ond_path_does_not_supply_dependencies() {
    let f = Fixture::new();
    f.put(
        "game/main.ond",
        "package main\nimport \"math\"\nimport \"text\"\nfunc main(){var _=math.Value(); text.Use()}",
    );
    f.put("more-libraries/text/text.ond", "package text\nfunc Use(){}");
    let ond_path =
        std::env::join_paths([f.0.join("libraries"), f.0.join("more-libraries")]).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ond"))
        .current_dir(&f.0)
        .args(["compile", "game", "-o", "-"])
        .env("OND_PATH", ond_path)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown imported package"));
}

#[test]
fn manifest_path_dependencies_include_subpackages_and_import_aliases() {
    let f = Fixture::new();
    f.put(
        "game/ond.toml",
        "[project]\nname = \"game\"\n[dependencies]\nstandard_alias = { path = \"../standard\" }\nmy_alias = { path = \"../my\" }\n",
    );
    f.put(
        "game/main.ond",
        "package main\nimport std_json \"standard/json\"\nimport \"my/json\"\nfunc main(){var _=std_json.Value()+json.Value()}",
    );
    f.put("standard/ond.toml", "[project]\nname = \"standard\"\n");
    f.put(
        "standard/json/value.ond",
        "package json\nfunc Value()->i32{return 3}",
    );
    f.put("my/ond.toml", "[project]\nname = \"my\"\n");
    f.put(
        "my/json/value.ond",
        "package json\nfunc Value()->i32{return 4}",
    );

    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(manifest.packages.len(), 3);
}

#[cfg(windows)]
#[test]
fn git_dependency_checkout_accepts_a_canonical_project_root() {
    let f = Fixture::new();
    let repository = f.0.join("repository");
    f.put("repository/ond.toml", "[project]\nname = \"portable\"\n");
    f.put(
        "repository/library/value.ond",
        "package library\nfunc Value()->i32{return 7}\n",
    );
    git(&repository, &["init"]);
    git(
        &repository,
        &["config", "user.email", "ond@example.invalid"],
    );
    git(&repository, &["config", "user.name", "Ond Test"]);
    git(&repository, &["add", "."]);
    git(&repository, &["commit", "-m", "initial"]);
    let repository_url = repository.to_string_lossy().replace('\\', "/");
    f.put(
        "game/ond.toml",
        &format!(
            "[project]\nname = \"game\"\n[dependencies]\nlocal = {{ git = {repository_url:?} }}\n"
        ),
    );
    f.put(
        "game/main.ond",
        "package main\nimport \"portable/library\"\nfunc main(){var _=library.Value()}\n",
    );

    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_dir(f.0.join("game/.ond/dependencies"))
            .unwrap()
            .any(|entry| entry.unwrap().path().join(".git").is_dir())
    );
}

#[test]
fn dependency_can_import_its_sibling_by_its_own_project_namespace() {
    let f = Fixture::new();
    f.put(
        "game/ond.toml",
        "[project]\nname = \"game\"\n[dependencies]\nrenamed = { path = \"../portable\" }\n",
    );
    f.put(
        "game/main.ond",
        "package main\nimport \"ond_pure_packages/zlib\"\nfunc main(){zlib.Use()}\n",
    );
    f.put(
        "portable/ond.toml",
        "[project]\nname = \"ond_pure_packages\"\n",
    );
    f.put(
        "portable/blob/blob.ond",
        "package blob\nfunc Value()->i32{return 1}\n",
    );
    f.put(
        "portable/zlib/zlib.ond",
        "package zlib\nimport \"ond_pure_packages/blob\"\nfunc Use()->i32{return blob.Value()}\n",
    );

    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn manifest_resolve_ignore_excludes_generated_source_trees() {
    let f = Fixture::new();
    f.put(
        "game/ond.toml",
        "[project]\nname = \"game\"\n[resolve]\nignore = [\"generated/kagura\"]\n",
    );
    f.put("game/main.ond", "package main\nfunc main(){}\n");
    f.put("game/generated/kagura/invalid.ond", "this is not Ond");
    f.put("game/generated/portable/value.ond", "package portable\n");

    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        manifest
            .packages
            .iter()
            .any(|package| package.package == "game/generated/portable")
    );

    f.put(
        "game/main.ond",
        "package main\nimport \"game/generated/kagura\"\nfunc main(){}\n",
    );
    let ignored_import = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(!ignored_import.status.success());
    assert!(
        String::from_utf8_lossy(&ignored_import.stderr)
            .contains("excluded from source discovery by ond.toml resolve.ignore")
    );
}
const LINK: &[&str] = &["link", "-", "-o", "-"];
#[test]
fn files_default_output_cache_stdout_and_stdin_link() {
    let f = Fixture::new();
    let cold = run(&f.0, &["compile", "game", "-v"], None);
    assert!(
        cold.status.success(),
        "{}",
        String::from_utf8_lossy(&cold.stderr)
    );
    assert!(cold.stdout.is_empty());
    assert!(String::from_utf8_lossy(&cold.stderr).contains("rebuilt: ., math"));
    let manifest = fs::read(f.0.join("game/target/build.ondbuild")).unwrap();
    let warm = run(&f.0, &["compile", "game", "-o", "-", "-v"], None);
    assert!(warm.status.success());
    assert!(String::from_utf8_lossy(&warm.stderr).contains("reused: ., math"));
    assert_eq!(warm.stdout, manifest);
    let _: CompilationManifest = serde_json::from_slice(&manifest).unwrap();
    let linked = run(&f.0, LINK, Some(&manifest));
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let _: Executable = Executable::read(&linked.stdout).unwrap();
    let file = run(
        &f.0,
        &["link", "game/target/build.ondbuild", "-o", "image.bin"],
        None,
    );
    assert!(file.status.success());
    assert!(file.stdout.is_empty());
    assert_eq!(fs::read(f.0.join("image.bin")).unwrap(), linked.stdout);
    Executable::read(&linked.stdout).unwrap();
    let debug: DebugInfo =
        serde_json::from_slice(&fs::read(f.0.join("image.onddebug")).unwrap()).unwrap();
    assert_eq!(binary_hash(&linked.stdout), debug.build_id);
    assert!(!debug.functions.is_empty());
    let cwd = run(&f.0.join("game"), &["compile", "-o", "-"], None);
    assert!(cwd.status.success());
    assert_eq!(cwd.stdout, manifest);
}
#[test]
fn pipe_connects_real_processes_without_wrapping_requests() {
    let f = Fixture::new();
    let mut compile = Command::new(env!("CARGO_BIN_EXE_ond"))
        .current_dir(&f.0)
        .args(["compile", "game", "-o", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let linked = Command::new(env!("CARGO_BIN_EXE_ond"))
        .current_dir(&f.0)
        .args(LINK)
        .stdin(Stdio::from(compile.stdout.take().unwrap()))
        .output()
        .unwrap();
    let status = compile.wait().unwrap();
    let mut compile_stderr = String::new();
    compile
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut compile_stderr)
        .unwrap();
    assert!(status.success(), "{compile_stderr}");
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    Executable::read(&linked.stdout).unwrap();
}

#[cfg(windows)]
fn git(directory: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn errors_are_stderr_only_and_preserve_previous_outputs() {
    let f = Fixture::new();
    f.put("previous.bin", "previous");
    let bad = run(
        &f.0,
        &["link", "-", "-o", "previous.bin"],
        Some(b"not a manifest"),
    );
    assert_eq!(bad.status.code(), Some(1));
    assert!(bad.stdout.is_empty());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("invalid compilation manifest"));
    assert_eq!(fs::read(f.0.join("previous.bin")).unwrap(), b"previous");
    for args in [
        vec!["link", "--ram", "0:16"],
        vec!["compile", "--bogus"],
        vec!["compile", "-o"],
        vec!["compile", "-o", "x", "-o", "y"],
        vec!["link", "--stack", "4"],
    ] {
        let out = run(&f.0, &args, None);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
    f.put("game/main.ond", "package main\nfunc main(){ unknown() }");
    let bad = run(&f.0, &["compile", "game", "-o", "previous.bin"], None);
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("unknown"));
    assert_eq!(fs::read(f.0.join("previous.bin")).unwrap(), b"previous");
}
#[test]
fn dependency_names_and_unused_packages() {
    let f = Fixture::new();
    f.put("game/math/other.ond", "package math\nfunc Other(){}\n");
    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
    assert!(manifest.packages.iter().any(|p| p.package == "game/math"));
    assert!(manifest.packages.iter().any(|p| p.package == "math"));
    f.put("libraries/unused/ond.toml", "[project]\nname=\"unused\"\n");
    f.put(
        "libraries/unused/api.ond",
        "package unused\nfunc Unused(){}\n",
    );
    f.put("game/ond.toml","[project]\nname=\"game\"\n[dependencies]\na={path=\"../libraries/math\"}\nb={path=\"../libraries/math\"}\nunused={path=\"../libraries/unused\"}\n");
    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(manifest.packages.len(), 3);
    f.put("libraries2/math/ond.toml", "[project]\nname=\"math\"\n");
    f.put(
        "libraries2/math/api.ond",
        "package math\nfunc Value()->i32{return 7}\n",
    );
    f.put("game/ond.toml","[project]\nname=\"game\"\n[dependencies]\na={path=\"../libraries/math\"}\nb={path=\"../libraries2/math\"}\n");
    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("dependency namespace"));
}

#[test]
fn libraries_with_matching_physical_names_keep_qualified_package_identity() {
    let f = Fixture::new();
    f.put("game/main.ond", "package main\nimport own \"game/math\"\nimport extra \"math\"\nfunc main() { _ = own.Value() + extra.Value() }\n");
    for own in [
        "package math\nfunc Value()->i32{return 7}",
        "package math\nfunc Value()->i32{return 9}",
    ] {
        f.put("game/math/value.ond", own);
        let output = run(&f.0, &["compile", "game", "-o", "-"], None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            manifest
                .packages
                .iter()
                .map(|p| p.package.as_str())
                .collect::<Vec<_>>(),
            ["game", "game/math", "math"]
        );
    }
}

#[test]
fn ignored_or_nested_project_files_do_not_shadow_dependencies() {
    for directory in ["generated", ".ond", "nested"] {
        let f = Fixture::new();
        f.put(
            "libraries/generated/ond.toml",
            "[project]\nname=\"generated\"\n",
        );
        f.put(
            "libraries/generated/math/value.ond",
            "package math\nfunc Value()->i32{return 7}\n",
        );
        f.put("game/ond.toml",&format!("[project]\nname=\"game\"\n[dependencies]\ngenerated={{path=\"../libraries/generated\"}}\n[resolve]\nignore=[{directory:?}]\n"));
        f.put(
            "game/main.ond",
            "package main\nimport \"generated/math\"\nfunc main(){_=math.Value()}\n",
        );
        f.put(
            &format!("game/{directory}/math/value.ond"),
            "invalid duplicate source",
        );
        if directory == "nested" {
            f.put("game/nested/ond.toml", "[project]\nname=\"nested\"\n");
        }
        let output = run(&f.0, &["compile", "game", "-o", "-"], None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            manifest
                .packages
                .iter()
                .any(|p| p.package == "generated/math")
        );
    }
}

#[test]
fn test_build_defaults_sidecars_and_stdout_contract() {
    let f = Fixture::new();
    f.put(
        "game/main_test.ond",
        "package main\nfunc TestOne(){}\nfunc TestTwo(){}\n",
    );
    f.put(
        "game/unused/only_test.ond",
        "package unused\nfunc TestUnreachable(){}\n",
    );
    f.put(
        "libraries/math/invalid_test.ond",
        "invalid source must be ignored",
    );
    for test in [false, true] {
        let mut args = vec!["compile", "game"];
        if test {
            args.push("--test");
        }
        let output = run(&f.0, &args, None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let path = if test {
            "game/target/test.ondbuild"
        } else {
            "game/target/build.ondbuild"
        };
        let bytes = fs::read(f.0.join(path)).unwrap();
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest.get("tests").is_some(), test);
        if test {
            assert_eq!(manifest["tests"]["cases"].as_array().unwrap().len(), 3);
        }
        let image = if test { "test.bin" } else { "normal.bin" };
        let output = run(&f.0, &["link", path, "-o", image], None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let _linked: Executable = Executable::read(&fs::read(f.0.join(image)).unwrap()).unwrap();
        assert!(f.0.join(image).with_extension("onddebug").exists());
        assert_eq!(f.0.join(image).with_extension("ondtest").exists(), test);
        if test {
            let sidecar: serde_json::Value = serde_json::from_slice(
                &fs::read(f.0.join(image).with_extension("ondtest")).unwrap(),
            )
            .unwrap();
            assert_eq!(sidecar.as_object().unwrap().len(), 2);
            assert_eq!(
                sidecar["build_id"],
                binary_hash(&fs::read(f.0.join(image)).unwrap())
            );
            for case in sidecar["cases"].as_array().unwrap() {
                assert_eq!(case.as_object().unwrap().len(), 3);
                assert!(case["entry_point"].is_u64());
            }
            let output = run(&f.0, &["link", path, "-o", "-"], None);
            assert!(output.status.success());
            let _: Executable = Executable::read(&output.stdout).unwrap();
            assert!(!f.0.join("-.ondtest").exists());
            for collision in ["bad.ondtest", "bad.onddebug"] {
                let output = run(&f.0, &["link", path, "-o", collision], None);
                assert!(!output.status.success());
                assert!(!f.0.join(collision).exists());
            }
        }
        let mut args = vec!["compile", "game", "-o", "explicit.ondbuild"];
        if test {
            args.push("--test");
        }
        assert!(run(&f.0, &args, None).status.success());
        let manifest: CompilationManifest =
            serde_json::from_slice(&fs::read(f.0.join("explicit.ondbuild")).unwrap()).unwrap();
        assert_eq!(manifest.tests.is_some(), test);
    }
    fs::remove_file(f.0.join("game/main_test.ond")).unwrap();
    fs::remove_file(f.0.join("game/unused/only_test.ond")).unwrap();
    assert!(
        run(&f.0, &["compile", "game", "--test"], None)
            .status
            .success()
    );
    assert!(
        run(
            &f.0,
            &["link", "game/target/test.ondbuild", "-o", "empty.bin"],
            None
        )
        .status
        .success()
    );
    let empty: ond_protocol::TestSidecar =
        serde_json::from_slice(&fs::read(f.0.join("empty.ondtest")).unwrap()).unwrap();
    assert!(empty.cases.is_empty());
}

fn artifact_path_case(mode: &str) {
    let f = Fixture::new();
    let compiled = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(compiled.status.success(), "{:?}", compiled);
    let mut manifest: CompilationManifest = serde_json::from_slice(&compiled.stdout).unwrap();
    let build = f.0.join("build");
    let cwd = f.0.join("run");
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::write(cwd.join("ond.toml"), LINK_SETTINGS).unwrap();
    for reference in &mut manifest.packages {
        let name = reference.path.file_name().unwrap().to_owned();
        let base = if mode == "stdin" { &cwd } else { &build };
        let destination = if mode == "absolute" {
            f.0.join("shared").join(&name)
        } else {
            base.join("cache").join(&name)
        };
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(&reference.path, &destination).unwrap();
        reference.path = if mode == "absolute" {
            destination
        } else {
            PathBuf::from("cache").join(name)
        };
    }
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let path = build.join("app.ondbuild");
    fs::write(&path, &bytes).unwrap();
    let path = path.to_str().unwrap();
    let mut args = LINK.to_vec();
    args[1] = path;
    let output = if mode == "stdin" {
        run(&cwd, LINK, Some(&bytes))
    } else {
        run(&cwd, &args, None)
    };
    assert!(
        output.status.success(),
        "ART-01.path-{mode}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Executable::read(&output.stdout).unwrap();
    if mode == "absolute" {
        let stdin = run(&cwd, LINK, Some(&bytes));
        assert!(
            stdin.status.success(),
            "ART-01.path-absolute stdin: {}",
            String::from_utf8_lossy(&stdin.stderr)
        );
        Executable::read(&stdin.stdout).unwrap();
    }
}
#[test]
fn art01_path_file() {
    artifact_path_case("file");
}
#[test]
fn art01_path_stdin() {
    artifact_path_case("stdin");
}
#[test]
fn art01_path_absolute() {
    artifact_path_case("absolute");
}

#[test]
fn dependency_root_roles_and_subpackages_are_preserved_in_test_links() {
    let f = Fixture::new();
    f.put(
        "libraries/hogeLib/ond.toml",
        "[project]\nname=\"hogeLib\"\n",
    );
    f.put(
        "libraries/hogeLib/main.ond",
        "package hogeLib\nfunc main(){}\nfunc Value()->u32{return 1}\n",
    );
    f.put(
        "game/ond.toml",
        "[project]\nname=\"game\"\n[dependencies]\nhogeLib={path=\"../libraries/hogeLib\"}\n",
    );
    f.put(
        "game/main.ond",
        "package main\nimport \"hogeLib\"\nfunc main(){_=hogeLib.Value()}\n",
    );
    f.put("game/main_test.ond", "package main\nfunc TestRoot(){}\n");
    for mode in [false, true] {
        let mut args = vec!["compile", "game", "-o", "-"];
        if mode {
            args.push("--test");
        }
        let output = run(&f.0, &args, None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let linked = run(&f.0, &["link", "-", "-o", "-"], Some(&output.stdout));
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
    }
    f.put(
        "libraries/executable/ond.toml",
        "[project]\nname=\"main\"\n",
    );
    f.put(
        "libraries/executable/main.ond",
        "package main\nfunc main(){}\nfunc Value()->u32{return 1}\n",
    );
    f.put(
        "libraries/executable/api/api.ond",
        "package api\nfunc Value()->u32{return 2}\n",
    );
    f.put(
        "game/ond.toml",
        "[project]\nname=\"game\"\n[dependencies]\nother={path=\"../libraries/executable\"}\n",
    );
    f.put(
        "game/main.ond",
        "package main\nimport other \"main\"\nfunc main(){_=other.Value()}\n",
    );
    let output = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot import application root"));
    f.put(
        "game/main.ond",
        "package main\nimport \"main/api\"\nfunc main(){_=api.Value()}\n",
    );
    let output = run(&f.0, &["compile", "game", "--test", "-o", "-"], None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let linked = run(&f.0, &["link", "-", "-o", "-"], Some(&output.stdout));
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
}

#[test]
fn removed_library_option_is_an_argument_error() {
    let f = Fixture::new();
    let output = run(
        &f.0,
        &["compile", "game", "--library-dir", "libraries"],
        None,
    );
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn dependency_main_subpackage_keeps_its_qualified_function_namespace() {
    let f = Fixture::new();
    f.put(
        "libraries/hogeLib/ond.toml",
        "[project]\nname=\"hogeLib\"\n",
    );
    f.put(
        "libraries/hogeLib/main/api.ond",
        "package main\nfunc main(){}\nfunc Value()->u32{return 1}\n",
    );
    f.put(
        "game/ond.toml",
        "[project]\nname=\"game\"\n[dependencies]\nlib={path=\"../libraries/hogeLib\"}\n",
    );
    f.put(
        "game/main.ond",
        "package main\nimport lib \"hogeLib/main\"\nfunc main(){_=lib.Value()}\n",
    );
    f.put("game/main_test.ond", "package main\nfunc TestRoot(){}\n");
    for mode in [false, true] {
        let mut args = vec!["compile", "game", "-o", "-"];
        if mode {
            args.push("--test");
        }
        let output = run(&f.0, &args, None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let linked = run(&f.0, &["link", "-", "-o", "-"], Some(&output.stdout));
        assert!(
            linked.status.success(),
            "{}",
            String::from_utf8_lossy(&linked.stderr)
        );
    }
}

const LINK_SETTINGS: &str = "[project]\nname='run'\n[link]\nstartup='external'\nstack-size=65536\n[link.ram]\nbase=4096\nsize=262144\n[link.exit]\nmode='jump'\naddress=0\n";
fn binary_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
