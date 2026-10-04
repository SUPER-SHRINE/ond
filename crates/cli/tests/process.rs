use ond_protocol::{CompilationManifest, debug::DebugInfo, link::LinkedImage};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};
struct Fixture(PathBuf);
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
        f.put("game/ond.toml", "");
        f.put(
            "game/main.ond",
            "package main\nimport \"math\"\nfunc main(){var _=math.Value()}",
        );
        f.put(
            "libraries/math/value.ond",
            "package math\nfunc Value()->i32{return 7}",
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
        .env_remove("OND_PATH")
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
fn ond_path_supplies_library_roots() {
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
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: CompilationManifest = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(manifest.packages.len(), 3);
}

#[test]
fn manifest_path_dependencies_include_subpackages_and_import_aliases() {
    let f = Fixture::new();
    f.put(
        "game/ond.toml",
        "[dependencies]\nstandard_alias = { path = \"../standard\" }\nmy_alias = { path = \"../my\" }\n",
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
        &format!("[dependencies]\nlocal = {{ git = {repository_url:?} }}\n"),
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
        "[dependencies]\nrenamed = { path = \"../portable\" }\n",
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
        "[resolve]\nignore = [\"generated/kagura\"]\n",
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
            .any(|package| package.package == "generated/portable")
    );

    f.put(
        "game/main.ond",
        "package main\nimport \"generated/kagura\"\nfunc main(){}\n",
    );
    let ignored_import = run(&f.0, &["compile", "game", "-o", "-"], None);
    assert!(!ignored_import.status.success());
    assert!(
        String::from_utf8_lossy(&ignored_import.stderr)
            .contains("excluded from source discovery by ond.toml resolve.ignore")
    );
}
const LINK: &[&str] = &[
    "link",
    "-",
    "--ram",
    "0x1000:64KiB",
    "--stack",
    "8KiB",
    "--return-to",
    "0",
    "-o",
    "-",
];
#[test]
fn files_default_output_cache_stdout_and_stdin_link() {
    let f = Fixture::new();
    let cold = run(
        &f.0,
        &["compile", "game", "--library-dir", "libraries", "-v"],
        None,
    );
    assert!(
        cold.status.success(),
        "{}",
        String::from_utf8_lossy(&cold.stderr)
    );
    assert!(cold.stdout.is_empty());
    assert!(String::from_utf8_lossy(&cold.stderr).contains("rebuilt: ., math"));
    let manifest = fs::read(f.0.join("game/target/build.ondbuild")).unwrap();
    let warm = run(
        &f.0,
        &[
            "compile",
            "game",
            "--library-dir",
            "libraries",
            "-o",
            "-",
            "-v",
        ],
        None,
    );
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
    let _: LinkedImage = serde_json::from_slice(&linked.stdout).unwrap();
    let file = run(
        &f.0,
        &[
            "link",
            "game/target/build.ondbuild",
            "--ram",
            "4096:65536",
            "--stack",
            "8192",
            "--return-to",
            "0",
            "-o",
            "image.ondimage",
        ],
        None,
    );
    assert!(file.status.success());
    assert!(file.stdout.is_empty());
    assert_eq!(fs::read(f.0.join("image.ondimage")).unwrap(), linked.stdout);
    let image: LinkedImage = serde_json::from_slice(&linked.stdout).unwrap();
    let debug: DebugInfo =
        serde_json::from_slice(&fs::read(f.0.join("image.onddebug")).unwrap()).unwrap();
    assert_eq!(image.build_id, debug.build_id);
    assert!(!debug.functions.is_empty());
    let cwd = run(
        &f.0.join("game"),
        &["compile", "--library-dir", "../libraries", "-o", "-"],
        None,
    );
    assert!(cwd.status.success());
    assert_eq!(cwd.stdout, manifest);
}
#[test]
fn pipe_connects_real_processes_without_wrapping_requests() {
    let f = Fixture::new();
    let mut compile = Command::new(env!("CARGO_BIN_EXE_ond"))
        .current_dir(&f.0)
        .args(["compile", "game", "--library-dir", "libraries", "-o", "-"])
        .env_remove("OND_PATH")
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
    serde_json::from_slice::<LinkedImage>(&linked.stdout).unwrap();
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
    f.put("previous.ondimage", "previous");
    let bad = run(
        &f.0,
        &[
            "link",
            "-",
            "--ram",
            "0:64KiB",
            "--stack",
            "8KiB",
            "--return-to",
            "0",
            "-o",
            "previous.ondimage",
        ],
        Some(b"not a manifest"),
    );
    assert_eq!(bad.status.code(), Some(1));
    assert!(bad.stdout.is_empty());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("invalid compilation manifest"));
    assert_eq!(
        fs::read(f.0.join("previous.ondimage")).unwrap(),
        b"previous"
    );
    for args in [
        vec!["link", "-"],
        vec!["compile", "--bogus"],
        vec!["compile", "-o"],
        vec!["compile", "-o", "x", "-o", "y"],
        vec![
            "link",
            "-",
            "--ram",
            "0:4294967296",
            "--stack",
            "8KiB",
            "--return-to",
            "0",
        ],
    ] {
        let out = run(&f.0, &args, None);
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
    }
    f.put("game/main.ond", "package main\nfunc main(){ unknown() }");
    let bad = run(&f.0, &["compile", "game", "-o", "previous.ondimage"], None);
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("unknown"));
    assert_eq!(
        fs::read(f.0.join("previous.ondimage")).unwrap(),
        b"previous"
    );
}
#[test]
fn library_package_collisions_and_unused_packages() {
    let f = Fixture::new();
    f.put("game/math/other.ond", "package math\nfunc Other(){}");
    let bad = run(
        &f.0,
        &["compile", "game", "--library-dir", "libraries", "-o", "-"],
        None,
    );
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("conflicts"));
    fs::remove_file(f.0.join("game/math/other.ond")).unwrap();
    f.put(
        "libraries/unused/unused.ond",
        "package unused\nfunc Unused(){}",
    );
    let out = run(
        &f.0,
        &["compile", "game", "--library-dir", "libraries", "-o", "-"],
        None,
    );
    assert!(out.status.success());
    let m: CompilationManifest = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(m.packages.len(), 2);
    f.put(
        "game/generated/unused/unused.ond",
        "package unused\nfunc Unused(){}",
    );
    let nested = run(
        &f.0,
        &[
            "compile",
            "game",
            "--library-dir",
            "libraries",
            "--library-dir",
            "game/generated",
            "-o",
            "-",
        ],
        None,
    );
    assert!(
        nested.status.success(),
        "{}",
        String::from_utf8_lossy(&nested.stderr)
    );
    let nested: CompilationManifest = serde_json::from_slice(&nested.stdout).unwrap();
    assert_eq!(nested.packages.len(), 2);
    f.put(
        "libraries2/math/another.ond",
        "package math\nfunc Another(){}",
    );
    let bad = run(
        &f.0,
        &[
            "compile",
            "game",
            "--library-dir",
            "libraries",
            "--library-dir",
            "libraries2",
            "-o",
            "-",
        ],
        None,
    );
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("conflicting library"));
}
