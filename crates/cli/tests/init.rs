use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/ond-cli init {} {} {}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ond"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn init_generates_a_buildable_project_without_overwriting() {
    let f = Fixture::new();
    let initialized = f.run(&["init"]);
    assert_eq!(initialized.status.code(), Some(0));
    assert!(initialized.stdout.is_empty());
    assert!(initialized.stderr.is_empty());
    let manifest = fs::read_to_string(f.0.join("ond.toml")).unwrap();
    assert!(manifest.contains("name = \"app\""));
    assert!(manifest.contains("startup = \"external\""));
    assert!(manifest.contains("base = 4096"));
    assert!(manifest.contains("mode = \"loop\""));
    assert_eq!(
        fs::read_to_string(f.0.join("main.ond")).unwrap(),
        "package main\n\nfunc main() {\n}\n"
    );

    let compile = f.run(&["compile"]);
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let link = f.run(&["link"]);
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    assert!(f.0.join("target/a.bin").is_file());
    assert!(f.0.join("target/a.onddebug").is_file());

    fs::write(f.0.join("ond.toml"), "preserve this").unwrap();
    let rerun = f.run(&["init"]);
    assert_eq!(rerun.status.code(), Some(1));
    assert_eq!(fs::read(f.0.join("ond.toml")).unwrap(), b"preserve this");
    assert_eq!(
        fs::read_to_string(f.0.join("main.ond")).unwrap(),
        "package main\n\nfunc main() {\n}\n"
    );
}

#[test]
fn optional_settings_examples_keep_their_table_scopes() {
    let f = Fixture::new();
    assert!(f.run(&["init"]).status.success());
    let manifest = fs::read_to_string(f.0.join("ond.toml")).unwrap();
    let enabled = manifest
        .replace(
            "# [roles]\n# names = [\"Debug\", \"Development\"]",
            "[roles]\nnames = [\"Debug\", \"Development\"]",
        )
        .replace(
            "# [resolve]\n# ignore = [\".git\", \".ond\", \"target\"]",
            "[resolve]\nignore = [\".git\", \".ond\", \"target\"]",
        )
        .replace(
            "# [compile]\n# output = \"target/build.ondbuild\"\n# cache-dir = \"target/ond-cache\"",
            "[compile]\noutput = \"target/build.ondbuild\"\ncache-dir = \"target/ond-cache\"",
        )
        .replace(
            "# output = \"target/a.bin\" # 省略時の実行file保存先。target/a.onddebugも生成します。",
            "output = \"target/a.bin\"",
        )
        .replace(
            "# [link.rom]\n# ROM範囲の先頭addressと容量（byte）。self startupではbaseを0にします。\n# base = 268435456\n# size = 65536",
            "[link.rom]\n# ROM範囲の先頭addressと容量（byte）。self startupではbaseを0にします。\nbase = 268435456\nsize = 65536",
        )
        .replace(
            "# final = false",
            "final = false",
        );
    let parsed: toml::Value = toml::from_str(&enabled).unwrap();
    assert_eq!(parsed["project"]["final"].as_bool(), Some(false));
    assert_eq!(
        parsed["roles"]["names"].as_array().unwrap()[0].as_str(),
        Some("Debug")
    );
    assert_eq!(
        parsed["resolve"]["ignore"].as_array().unwrap()[0].as_str(),
        Some(".git")
    );
    assert_eq!(
        parsed["compile"]["cache-dir"].as_str(),
        Some("target/ond-cache")
    );
    assert_eq!(parsed["link"]["output"].as_str(), Some("target/a.bin"));
    assert_eq!(parsed["link"]["rom"]["base"].as_integer(), Some(268435456));
    fs::write(f.0.join("ond.toml"), enabled).unwrap();
    let compile = f.run(&["compile"]);
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let link = f.run(&["link"]);
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );

    let self_startup = fs::read_to_string(f.0.join("ond.toml"))
        .unwrap()
        .replace("startup = \"external\"", "startup = \"self\"")
        .replace("base = 268435456", "base = 0")
        .replace("base = 4096", "base = 65536");
    fs::write(f.0.join("ond.toml"), self_startup).unwrap();
    let compile = f.run(&["compile"]);
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let link = f.run(&["link"]);
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );

    let jump_exit = fs::read_to_string(f.0.join("ond.toml"))
        .unwrap()
        .replace(
            "mode = \"loop\"\n# loopはmainの正常復帰後に終了処理内で反復します。jumpではloopを置き換えてaddressを指定します。\n# mode = \"jump\"\n# address = 0 # jump先のabsolute address（byte、4-byte aligned）。0はsample値です。",
            "mode = \"jump\"\naddress = 0",
        );
    fs::write(f.0.join("ond.toml"), jump_exit).unwrap();
    let compile = f.run(&["compile"]);
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let link = f.run(&["link"]);
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
}

#[test]
fn init_preflights_existing_files_and_directories() {
    for existing in ["ond.toml", "main.ond"] {
        for is_directory in [false, true] {
            let f = Fixture::new();
            if is_directory {
                fs::create_dir(f.0.join(existing)).unwrap();
            } else {
                fs::write(f.0.join(existing), "preserve").unwrap();
            }
            let output = f.run(&["init"]);
            assert_eq!(output.status.code(), Some(1));
            assert!(
                !f.0.join(if existing == "ond.toml" {
                    "main.ond"
                } else {
                    "ond.toml"
                })
                .exists()
            );
            if !is_directory {
                assert_eq!(fs::read(f.0.join(existing)).unwrap(), b"preserve");
            }
        }
    }
}

#[test]
fn init_rejects_arguments_and_help_writes_no_files() {
    let f = Fixture::new();
    for args in [
        &["init", "project"][..],
        &["init", "-o"][..],
        &["init", "-v"][..],
    ] {
        let invalid = f.run(args);
        assert_eq!(invalid.status.code(), Some(2));
        assert!(invalid.stdout.is_empty());
        assert!(!f.0.join("ond.toml").exists());
    }
    for args in [&["init", "-h"][..], &["init", "--help"][..]] {
        let help = f.run(args);
        assert_eq!(help.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&help.stdout).contains("ond init"));
        assert!(!f.0.join("ond.toml").exists());
    }
}
