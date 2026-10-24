use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
struct Project(PathBuf);
const SETTINGS: &str = "[project]\nname='app'\n[compile]\noutput='meta/build.ondbuild'\ntest-output='meta/test.ondbuild'\ncache-dir='cache'\n[link]\noutput='out/game.bin'\ntest-output='out/tests.bin'\nstartup='external'\nstack-size=256\n[link.ram]\nbase=0x10000000\nsize=65536\n[link.exit]\nmode='loop'\n";
impl Project {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let p = Self(std::env::temp_dir().join(format!(
            "ond-binary-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir_all(p.0.join("app")).unwrap();
        fs::create_dir_all(p.0.join("run")).unwrap();
        p.put("ond.toml", SETTINGS);
        p.put("main.ond", "package main\nfunc main(){}\n");
        p
    }
    fn put(&self, path: &str, text: &str) {
        let path = self.0.join("app").join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ond"))
            .current_dir(self.0.join("run"))
            .args(args)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> Output {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        o
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn cli_01_configured_project_paths_and_binary_output() {
    let p = Project::new();
    p.ok(&["compile", "../app"]);
    assert!(p.0.join("app/meta/build.ondbuild").exists());
    p.ok(&["link", "--project", "../app"]);
    let bytes = fs::read(p.0.join("app/out/game.bin")).unwrap();
    assert_eq!(&bytes[..4], b"ONDX");
    assert!(p.0.join("app/out/game.onddebug").exists());
    let stdout = p.ok(&["link", "--project", "../app", "-o", "-"]);
    assert_eq!(stdout.stdout, bytes);
    assert_eq!(
        p.run(&["link", "--project", "../app", "--project", "../app"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        p.run(&["link", "--project", "../app", "--ram", "0:16"])
            .status
            .code(),
        Some(2)
    );
}
#[test]
fn cli_01_closed_settings_reject_unknown_and_invalid_values() {
    let p = Project::new();
    p.ok(&["compile", "../app"]);
    for text in [
        SETTINGS.replace("startup='external'", "startup='external'\nunknown=1"),
        SETTINGS.replace("size=65536", ""),
        SETTINGS.replace("stack-size=256", "stack-size='256'"),
        SETTINGS.replace("size=65536", "size=4294967296"),
        format!("{SETTINGS}\n[link.rom]\nbase=0x10000000\nsize=65536\n"),
        SETTINGS.replace("mode='loop'", "mode='loop'\naddress=4"),
        SETTINGS.replace("mode='loop'", "mode='loop'\nunknown=4"),
        SETTINGS.replace("output='out/game.bin'", "output=5"),
    ] {
        p.put("ond.toml", &text);
        let result = p.run(&["link", "--project", "../app"]);
        assert_eq!(result.status.code(), Some(1), "accepted {text}");
        assert!(!p.0.join("app/out/game.bin").exists());
    }
}
#[test]
fn cli_01_selection_argument_errors_and_no_automatic_compile() {
    let p = Project::new();
    p.ok(&["compile", "../app"]);
    fs::remove_file(p.0.join("app/main.ond")).unwrap();
    p.ok(&["link", "--project", "../app"]);
    fs::remove_file(p.0.join("app/meta/build.ondbuild")).unwrap();
    assert_eq!(
        p.run(&["link", "--project", "../app"]).status.code(),
        Some(1)
    );
    assert!(!p.0.join("app/meta/build.ondbuild").exists());
    fs::write(p.0.join("ond.toml"), SETTINGS).unwrap();
    assert_eq!(p.run(&["link"]).status.code(), Some(1));
    for args in [
        vec!["compile", "../app", "-v", "-v"],
        vec!["compile", "../app", "--test", "--test"],
        vec!["compile", "../app", "--cache-dir", "other"],
        vec!["link", "--project", "../app", "-o", "one", "-o", "two"],
        vec!["link", "--project", "-o", "x"],
        vec!["link", "--project"],
        vec!["link", "one", "two"],
        vec!["link", "--stack", "4"],
        vec!["link", "--read-only", "0:16"],
        vec!["link", "--return-to", "0"],
    ] {
        assert_eq!(p.run(&args).status.code(), Some(2), "{args:?}");
    }
    let o = p.run(&["link", "--project", "../app", "--", "--ram"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("--ram"));
}
fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn decode(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .flat_map(|s| {
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}
#[test]
fn hash_01_fixed_binary_vectors() {
    let w = decode(
        "4f4e4458 01000000 01000000 01000000 54000000 50000010 00010010 02000000 01000000 50000000 04000000 50000010 04000000 04000000 04000000 00000000 00000000 54000010 04000000 04000000 000000c0",
    );
    assert_eq!(
        hash(&w),
        "72d1a90b13286b8c57c4e1576be01d319d4070f963981067b9e55b2b73d4fc43"
    );
    let mut prefixed = vec![0; 4];
    prefixed.extend(w);
    assert_eq!(
        hash(&prefixed),
        "718f1c5a7693158554c0a490403ecf5ca2c56ac0bc15e3f833a37c7ad1e4166e"
    );
}
#[test]
fn bin_03_and_test_01_all_startups_sidecars_and_final_addresses() {
    use ond_protocol::{executable::Executable, object::SectionKind};
    for startup in ["ram", "rom", "self"] {
        let p = Project::new();
        let mut settings = SETTINGS.to_string();
        if startup != "ram" {
            settings += &format!(
                "\n[link.rom]\nbase={}\nsize=65536\n",
                if startup == "self" { 0 } else { 4096 }
            );
        }
        if startup == "self" {
            settings = settings.replace("startup='external'", "startup='self'");
        }
        p.put("ond.toml", &settings);
        p.put(
            "main.ond",
            "package main\nvar Counter:u32=3\nfunc main(){}\n",
        );
        p.put("main_test.ond","package main\nimport \"app/dep\"\nfunc TestOne(){*(536871168 as u32 as *u32)=Counter+dep.Value()}\nfunc TestTwo(){*(536871168 as u32 as *u32)=Counter+1}\n");
        p.put(
            "dep/dep.ond",
            "package dep\nvar Seed:u32=7\nfunc Value()->u32{return Seed}\n",
        );
        p.ok(&["compile", "../app", "--test"]);
        let manifest: ond_protocol::CompilationManifest =
            serde_json::from_slice(&fs::read(p.0.join("app/meta/test.ondbuild")).unwrap()).unwrap();
        assert_eq!(manifest.tests.as_ref().unwrap().cases.len(), 2);
        assert_eq!(manifest.program.initializers.len(), 2);
        p.ok(&["link", "../app/meta/test.ondbuild", "--project", "../app"]);
        let bytes = fs::read(p.0.join("app/out/tests.bin")).unwrap();
        let prefix = if startup == "self" { 172 } else { 0 };
        let wire = Executable::read(&bytes[prefix..]).unwrap();
        let text = wire
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::Text)
            .unwrap();
        let debug: ond_protocol::debug::DebugInfo =
            serde_json::from_slice(&fs::read(p.0.join("app/out/tests.onddebug")).unwrap()).unwrap();
        let tests: ond_protocol::TestSidecar =
            serde_json::from_slice(&fs::read(p.0.join("app/out/tests.ondtest")).unwrap()).unwrap();
        assert_eq!(debug.build_id, hash(&bytes));
        assert_eq!(tests.build_id, hash(&bytes));
        assert_eq!(tests.cases.len(), 2);
        for case in tests.cases {
            assert!(
                case.entry_point >= text.memory_address
                    && case.entry_point < text.memory_address + text.memory_size
            );
            assert!(case.entry_point.is_multiple_of(4));
        }
        for function in debug.functions {
            assert!(
                function.address >= text.memory_address
                    && function.address + function.size <= text.memory_address + text.memory_size
            );
            for row in function.unwind {
                assert!(
                    row.address >= text.memory_address
                        && row.address < text.memory_address + text.memory_size
                );
            }
        }
        for range in debug.ranges {
            assert!(
                range.start >= text.memory_address
                    && range.end <= text.memory_address + text.memory_size
            );
        }
        for s in &wire.sections {
            if startup == "ram" && !s.data.is_empty() {
                assert_eq!(s.memory_address, 0x10000000 + s.file_offset);
            }
            if startup != "ram" && matches!(s.kind, SectionKind::Text | SectionKind::Rodata) {
                assert_eq!(
                    s.memory_address,
                    (if startup == "self" {
                        prefix as u32
                    } else {
                        4096
                    }) + s.file_offset
                );
            }
        }
        let stdout = p.ok(&[
            "link",
            "../app/meta/test.ondbuild",
            "--project",
            "../app",
            "-o",
            "-",
        ]);
        assert_eq!(stdout.stdout, bytes);
        p.put("main_test.ond", "package main\n");
        p.ok(&["compile", "../app", "--test"]);
        p.ok(&["link", "../app/meta/test.ondbuild", "--project", "../app"]);
        let tests: ond_protocol::TestSidecar =
            serde_json::from_slice(&fs::read(p.0.join("app/out/tests.ondtest")).unwrap()).unwrap();
        assert!(tests.cases.is_empty());
    }
}
#[test]
fn cli_01_compile_only_and_test_fallback_and_library() {
    let p = Project::new();
    p.put("ond.toml", "[project]\nname='app'\n");
    p.ok(&["compile", "../app"]);
    assert_eq!(
        p.run(&["link", "--project", "../app"]).status.code(),
        Some(1)
    );
    p.put("ond.toml", SETTINGS);
    p.ok(&["compile", "../app", "--test"]);
    assert_eq!(
        p.run(&["link", "--project", "../app"]).status.code(),
        Some(1)
    );
    fs::remove_file(p.0.join("app/main.ond")).unwrap();
    p.put("lib.ond", "package app\nvar Value:u32=3\n");
    p.put(
        "lib_test.ond",
        "package app\nfunc TestValue(){*(536871168 as u32 as *u32)=Value}\n",
    );
    p.ok(&["compile", "../app", "--test"]);
    p.ok(&["link", "../app/meta/test.ondbuild", "--project", "../app"]);
}
#[test]
fn cli_01_failures_preserve_binary_and_sidecars() {
    let p = Project::new();
    p.put("main_test.ond", "package main\nfunc TestOne(){}\n");
    p.ok(&["compile", "../app", "--test"]);
    p.ok(&["link", "../app/meta/test.ondbuild", "--project", "../app"]);
    let paths = [
        "app/out/tests.bin",
        "app/out/tests.onddebug",
        "app/out/tests.ondtest",
    ];
    let previous = paths.map(|path| fs::read(p.0.join(path)).unwrap());
    for output in ["../app/out/tests.onddebug", "../app/out/tests.ondtest"] {
        assert_eq!(
            p.run(&[
                "link",
                "../app/meta/test.ondbuild",
                "--project",
                "../app",
                "-o",
                output
            ])
            .status
            .code(),
            Some(1)
        );
    }
    p.put("ond.toml", &SETTINGS.replace("size=65536", "size=256"));
    assert_eq!(
        p.run(&["link", "../app/meta/test.ondbuild", "--project", "../app"])
            .status
            .code(),
        Some(1)
    );
    for (path, bytes) in paths.iter().zip(&previous) {
        assert_eq!(fs::read(p.0.join(path)).unwrap(), *bytes);
    }
    p.put("ond.toml", SETTINGS);
    fs::remove_file(p.0.join(paths[2])).unwrap();
    fs::create_dir(p.0.join(paths[2])).unwrap();
    assert_eq!(
        p.run(&["link", "../app/meta/test.ondbuild", "--project", "../app"])
            .status
            .code(),
        Some(1)
    );
    for i in 0..2 {
        assert_eq!(fs::read(p.0.join(paths[i])).unwrap(), previous[i]);
    }
}

#[cfg(windows)]
#[test]
fn cli_01_publish_io_failure_restores_already_replaced_binary() {
    use std::os::windows::fs::OpenOptionsExt;
    let p = Project::new();
    p.ok(&["compile", "../app"]);
    p.ok(&["link", "--project", "../app"]);
    let binary = p.0.join("app/out/game.bin");
    let debug = p.0.join("app/out/game.onddebug");
    let before_binary = fs::read(&binary).unwrap();
    let before_debug = fs::read(&debug).unwrap();
    p.put("main.ond", "package main\nfunc main(){var _=42}\n");
    p.ok(&["compile", "../app"]);
    // Allow snapshot reads but deny rename/delete of the sidecar during publication.
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&debug)
        .unwrap();
    let result = p.run(&["link", "--project", "../app"]);
    assert_eq!(result.status.code(), Some(1));
    drop(lock);
    assert_eq!(fs::read(&binary).unwrap(), before_binary);
    assert_eq!(fs::read(&debug).unwrap(), before_debug);
}
