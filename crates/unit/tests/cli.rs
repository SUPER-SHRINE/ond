use std::{fs, path::PathBuf, process::Command};

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("ond-unit-tests")
            .join(format!("cli-tests-{}-{}", std::process::id(), {
                static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            }));
        fs::create_dir_all(root.join("checks/cases")).unwrap();
        fs::write(
            root.join("checks/ond.toml"),
            include_str!("fixtures/checks/ond.toml"),
        )
        .unwrap();
        fs::write(
            root.join("checks/cases/api.ond"),
            include_str!("fixtures/checks/cases/api.ond"),
        )
        .unwrap();
        Self { root }
    }
    fn config(&self, cases: &str) {
        self.config_role(cases, None);
    }
    fn config_role(&self, cases: &str, role: Option<&str>) {
        let role = role
            .map(|value| format!("role={value}\n"))
            .unwrap_or_default();
        fs::write(self.root.join("ond-unit.toml"),format!("version=1\n[project]\npath='checks'\n{role}[vm]\nram_size=65536\nstack_size=4096\n[[mmio]]\nname='port'\nbase=0x10000000\nsize=4\n{cases}")).unwrap();
    }
    fn run(&self, args: &[&str], code: i32) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_ond-unit"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.status.code(), Some(code), "{text}");
        text
    }
}
fn case(name: &str, function: &str) -> String {
    format!("\n[[cases]]\nname='{name}'\npackage='checks/cases'\nfunction='{function}'\n")
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn normal_compiler_cli_contract() {
    let f = Fixture::new();
    let first = case("pass", "Pass");
    let fault = case("fault", "Fault")
        + "expect_fault={cpu_kind='InvalidInstruction',ond_kind='ExplicitTrap',function='checks/cases.Fault',reason='expected',source={path='checks/cases/api.ond',line=5}}\n";
    let stub = case("stub", "Read")
        + "[[cases.mmio]]\ndevice='port'\nread_responses=[{width=4,value=7},{width=1,value=9}]\n";
    let mock = case("mock", "Write")
        + "[[cases.mmio]]\ndevice='port'\nexpected_accesses=[{operation='write',address=0x10000000,width=4,value=9}]\n";
    let ram = case("ram", "Ram")
        + "ram_initial=[{address=0x8000,bytes=[0x78,0x56,0x34,0x12]}]\nram_expected=[{address=0x8000,bytes=[0xef,0xcd,0xab,0x90]}]\n";
    f.config(&(first.clone() + &fault + &stub + &mock + &ram));
    let generated = f.root.join(".ond/target/ond-unit");
    let assert_generated = || {
        for name in ["ond.toml", "main.ond", "ond.lock"] {
            assert!(generated.join(name).is_file(), "missing generated {name}");
        }
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(generated.join("ond.toml")).unwrap()).unwrap();
        for (table, expected) in [
            ("compile", ".ond/target/build.ondbuild"),
            ("link", ".ond/target/a.bin"),
        ] {
            assert_eq!(manifest[table]["output"].as_str(), Some(expected));
            assert!(generated.join(expected).is_file());
        }
        assert!(generated.join(".ond/target/a.onddebug").is_file());
        assert!(generated.join(".ond/target/ond-cache").is_dir());
        for name in ["program.ondbuild", "program.bin", "program.onddebug"] {
            assert!(!generated.join(name).exists());
        }
        assert!(!f.root.join("checks/target").exists());
    };
    let text = f.run(&[], 0);
    assert_generated();
    assert!(
        text.contains("PASS=5 FAIL=0 STEP_LIMIT=0 ERROR=0"),
        "{text}"
    );
    // 生成project/cacheを保持した二回目も同じ入力で通る。
    assert!(f.run(&[], 0).contains("PASS=5"));
    assert_generated();
    let before = fs::read(f.root.join("checks/cases/api.ond")).unwrap();
    assert_eq!(
        before,
        include_str!("fixtures/checks/cases/api.ond").as_bytes()
    );
    f.config(&(first.clone() + &case("second", "SecondIndex")));
    let text = f.run(&["--case", "second", "--case", "second"], 0);
    assert!(!text.contains("pass: PASS"));
    assert!(text.contains("second: PASS"));
    assert!(text.contains("PASS=1"));
    f.run(&["--case", "unknown"], 2);
    f.config(&case("unexpected", "Fault"));
    let text = f.run(&[], 1);
    assert!(text.contains("unexpected: FAIL"));
    assert!(text.contains("reason=\"expected\""));
    assert!(text.contains("checks/cases.Fault"));
    assert!(text.contains("checks/cases/api.ond"));
    assert!(text.contains("ond_kind=ExplicitTrap"));
    let unmet = case("unmet", "Pass")
        + "expect_fault={cpu_kind='InvalidInstruction',function='checks/cases.Pass'}\n";
    f.config(&(unmet + &case("after", "Pass")));
    let text = f.run(&[], 1);
    assert!(text.contains("unmet: FAIL"));
    assert!(text.contains("after: PASS"));
    let text = f.run(&["--fail-fast"], 1);
    assert!(!text.contains("after: PASS"));
    f.config(&(case("loop", "Loop") + "max_steps=1000\n" + &case("after", "Pass")));
    let text = f.run(&[], 1);
    assert!(text.contains("loop: STEP_LIMIT"));
    assert!(text.contains("steps=1000"));
    assert!(text.contains("after: PASS"));
    f.config(
        &(case("exhausted", "Read")
            + "expect_fault={cpu_kind='BusFault',function='checks/cases.Read'}\n"),
    );
    let text = f.run(&[], 1);
    assert!(text.contains("exhausted: FAIL"));
    assert!(text.contains("応答枯渇"));
    f.config(
        &(case("width", "Read")
            + "[[cases.mmio]]\ndevice='port'\nread_responses=[{width=2,value=7}]\n"),
    );
    assert!(f.run(&[], 1).contains("応答幅不一致"));
    f.config(&(first + &case("bad", "WrongSignature")));
    // filterで選ばれない関数の型も全登録一覧から通常compileが検証する。
    assert!(f.run(&["--case", "pass"], 2).contains("ondビルド失敗"));
    f.config(&(case("pass", "Pass") + &case("not-a-function", "Callback")));
    let text = f.run(&["--case", "pass"], 2);
    assert!(text.contains("公開トップレベル関数宣言"));
    assert!(!text.contains("steps="));
    f.config(&case("type-is-not-a-function", "FunctionType"));
    f.run(&[], 2);
    f.config(&case("private", "helper"));
    f.run(&[], 2);
    // 通常のpath依存namespaceが生成名候補と一致しても、その依存を保持する。
    fs::create_dir_all(f.root.join("collision")).unwrap();
    fs::write(
        f.root.join("collision/ond.toml"),
        "[project]\nname='ond_unit_generated'\n",
    )
    .unwrap();
    fs::write(
        f.root.join("collision/api.ond"),
        "package ond_unit_generated\nfunc Check() {}\n",
    )
    .unwrap();
    fs::write(
        f.root.join("checks/ond.toml"),
        "[project]\nname='checks'\n[dependencies]\ncollision={path='../collision'}\n",
    )
    .unwrap();
    f.config(&case("namespace", "Pass"));
    assert!(f.run(&[], 0).contains("namespace: PASS"));
    f.config(&case("missing", "Missing"));
    f.run(&[], 2);
    f.config(&(case("bad-range", "Pass") + "ram_expected=[{address=65528,bytes=[0]}]\n"));
    f.run(&[], 2);
    f.config(&(case("loaded-image", "Pass") + "ram_initial=[{address=0,bytes=[0]}]\n"));
    assert!(f.run(&[], 2).contains("ロード契約"));
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn role_selection_and_final_target_contract() {
    let f = Fixture::new();
    let manifest = "[project]\nname='checks'\nfinal=false\n[roles]\nnames=['Test','Other']\n";
    let source = include_str!("fixtures/checks/cases/api.ond").to_owned()
        + "\nfunc RoleSwitch() {\n    var data = 0x8000 as *u32\n    *data = 1\n    role Test { *data = 2 }\n    role Other { *data = 3 }\n}\nrole Test {\n    func RoleOnly() {}\n}\n";
    let target_manifest = f.root.join("checks/ond.toml");
    let target_source = f.root.join("checks/cases/api.ond");
    fs::write(&target_manifest, manifest).unwrap();
    fs::write(&target_source, &source).unwrap();
    let generated = f.root.join(".ond/target/ond-unit");
    // 同じ生成project/cacheで名前付きroleと未指定構成を切り替える。
    for (role, value) in [
        (Some("'Test'"), 2),
        (Some("'Other'"), 3),
        (None, 1),
        (Some("'Test'"), 2),
    ] {
        let mut cases = case("switch", "RoleSwitch")
            + &format!("ram_expected=[{{address=0x8000,bytes=[{value},0,0,0]}}]\n");
        if role == Some("'Test'") {
            cases += &case("role-only", "RoleOnly");
        }
        f.config_role(&cases, role);
        let text = f.run(&[], 0);
        assert!(text.contains("switch: PASS"), "{text}");
        if role == Some("'Test'") {
            assert!(text.contains("role-only: PASS"), "{text}");
        }
        let generated_manifest: toml::Value =
            toml::from_str(&fs::read_to_string(generated.join("ond.toml")).unwrap()).unwrap();
        assert_eq!(generated_manifest["project"]["final"].as_bool(), Some(true));
        assert!(generated_manifest.get("roles").is_none());
        let dependency = generated_manifest["dependencies"]["subject"]
            .as_table()
            .unwrap();
        assert_eq!(dependency.len(), if role.is_some() { 2 } else { 1 });
        assert_eq!(
            dependency.get("role").and_then(|v| v.as_str()),
            role.map(|r| r.trim_matches('\''))
        );
        assert_eq!(fs::read_to_string(&target_manifest).unwrap(), manifest);
        assert_eq!(fs::read_to_string(&target_source).unwrap(), source);
    }
    // role限定関数はrole未指定の構成では公開されない。
    f.config(&case("role-only", "RoleOnly"));
    assert!(!f.run(&[], 2).contains("steps="));
    for role in ["'Missing'", "'invalid-role'", "''", "7"] {
        f.config_role(&case("invalid", "Pass"), Some(role));
        let text = f.run(&[], 2);
        assert!(text.contains("ERROR:"), "{text}");
        assert!(!text.contains("steps="), "{text}");
    }
    fs::write(&target_manifest, "[project]\nname='checks'\n").unwrap();
    f.config_role(&case("missing-roles", "Pass"), Some("'Test'"));
    assert!(!f.run(&[], 2).contains("steps="));
    // final対象は子packageのケースでも生成file更新前に拒否する。
    let generated_files = ["ond.toml", "main.ond", ".ond/target/a.bin"]
        .map(|name| (name, fs::read(generated.join(name)).unwrap()));
    let final_manifest = manifest.replace("final=false", "final=true");
    fs::write(&target_manifest, &final_manifest).unwrap();
    for role in [None, Some("'Test'")] {
        f.config_role(&case("final-child", "Pass"), role);
        let text = f.run(&[], 2);
        assert!(text.contains("final=false"), "{text}");
        assert!(!text.contains("steps="), "{text}");
        for (name, bytes) in &generated_files {
            assert_eq!(fs::read(generated.join(name)).unwrap(), *bytes);
        }
        assert_eq!(
            fs::read_to_string(&target_manifest).unwrap(),
            final_manifest
        );
        assert_eq!(fs::read_to_string(&target_source).unwrap(), source);
    }
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn locked_git_dependency_namespace_collisions_use_generated_cache() {
    use sha2::{Digest, Sha256};
    let f = Fixture::new();
    let seed = f.root.join("cache-seed");
    fs::create_dir(&seed).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&seed)
            .args([
                "-c",
                "user.name=super-shrine-share",
                "-c",
                "user.email=337560299+super-shrine-share@users.noreply.github.com",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--quiet"]);
    git(&["commit", "--quiet", "--allow-empty", "-m", "first commit"]);
    fs::write(
        seed.join("ond.toml"),
        "[project]\nname='ond_unit_generated'\n[dependencies]\nnested={path='nested'}\n",
    )
    .unwrap();
    fs::write(
        seed.join("api.ond"),
        "package ond_unit_generated\nfunc Check() {}\n",
    )
    .unwrap();
    fs::create_dir(seed.join("nested")).unwrap();
    fs::write(
        seed.join("nested/ond.toml"),
        "[project]\nname='ond_unit_generated_'\n",
    )
    .unwrap();
    fs::write(
        seed.join("nested/api.ond"),
        "package ond_unit_generated_\nfunc Check() {}\n",
    )
    .unwrap();
    git(&["add", "ond.toml", "api.ond", "nested"]);
    git(&["commit", "--quiet", "-m", "テスト用の依存projectを追加"]);
    let commit = git(&["rev-parse", "HEAD"]);
    let url = "https://example.invalid/ond-unit-namespace-fixture.git";
    // 通常compilerの固定commit cacheを事前配置し、fetch/cloneなしで検査する。
    let key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(url, "HEAD", &commit)).unwrap())
    );
    let generated = f.root.join(".ond/target/ond-unit");
    let cache = generated.join(".ond/dependencies");
    fs::create_dir_all(&cache).unwrap();
    fs::rename(&seed, cache.join(format!("git-{}", &key[..12]))).unwrap();
    let subject_manifest =
        format!("[project]\nname='checks'\n[dependencies]\nhidden={{git='{url}',rev='HEAD'}}\n");
    let lock = format!(
        "version=2\n[[package]]\nowner='checks'\nname='hidden'\ngit='{url}'\nrev='HEAD'\ncommit='{commit}'\n"
    );
    fs::write(f.root.join("checks/ond.toml"), &subject_manifest).unwrap();
    fs::write(f.root.join("checks/ond.lock"), &lock).unwrap();
    f.config(&case("git-cache", "Pass"));
    for _ in 0..2 {
        assert!(f.run(&[], 0).contains("git-cache: PASS"));
        let manifest: toml::Value =
            toml::from_str(&fs::read_to_string(generated.join("ond.toml")).unwrap()).unwrap();
        assert_eq!(
            manifest["project"]["name"].as_str(),
            Some("ond_unit_generated__")
        );
        assert_eq!(
            fs::read_to_string(f.root.join("checks/ond.toml")).unwrap(),
            subject_manifest
        );
        assert_eq!(
            fs::read_to_string(f.root.join("checks/ond.lock")).unwrap(),
            lock
        );
        assert_eq!(
            fs::read_to_string(f.root.join("checks/cases/api.ond")).unwrap(),
            include_str!("fixtures/checks/cases/api.ond")
        );
    }
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn generated_project_back_edge_returns_compiler_error() {
    let f = Fixture::new();
    f.config(&case("back-edge", "Pass"));
    assert!(f.run(&[], 0).contains("back-edge: PASS"));
    let manifest =
        "[project]\nname='checks'\n[dependencies]\nback={path='../.ond/target/ond-unit'}\n";
    fs::write(f.root.join("checks/ond.toml"), manifest).unwrap();
    let text = f.run(&[], 2);
    assert!(text.contains("ondビルド失敗"), "{text}");
    assert!(!text.contains("steps="), "{text}");
    assert_eq!(
        fs::read_to_string(f.root.join("checks/ond.toml")).unwrap(),
        manifest
    );
    let generated: toml::Value =
        toml::from_str(&fs::read_to_string(f.root.join(".ond/target/ond-unit/ond.toml")).unwrap())
            .unwrap();
    assert_eq!(
        generated["project"]["name"].as_str(),
        Some("ond_unit_generated")
    );
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn case_files_share_environment_and_keep_filtered_indices() {
    let f = Fixture::new();
    let root = "version=1\ncase_files=['parts/indices.toml','parts/observations.toml']\n[project]\npath='checks'\n[vm]\nram_size=65536\nstack_size=4096\n[[mmio]]\nname='port'\nbase=0x10000000\nsize=4\n".to_owned() + &case("inline", "Pass");
    fs::create_dir_all(f.root.join("parts")).unwrap();
    let indices = "package='checks/cases'\n[[cases]]\nname='second'\nfunction='SecondIndex'\n";
    fs::write(f.root.join("parts/indices.toml"), indices).unwrap();
    // Child records use exactly the inline case observations and opaque debug strings.
    let observations = "package='checks/cases'\n".to_owned()
        + &case("fault", "Fault").replace("package='checks/cases'\n", "")
        + "expect_fault={ond_kind='ExplicitTrap',source={path='checks/cases/api.ond',line=5}}\n"
        + &case("read", "Read").replace("package='checks/cases'\n", "")
        + "[[cases.mmio]]\ndevice='port'\nread_responses=[{width=4,value=7}]\n"
        + &case("ram", "Ram").replace("package='checks/cases'\n", "")
        + "ram_initial=[{address=0x8000,bytes=[0x78,0x56,0x34,0x12]}]\nram_expected=[{address=0x8000,bytes=[0xef,0xcd,0xab,0x90]}]\n";
    fs::write(f.root.join("parts/observations.toml"), &observations).unwrap();
    fs::write(f.root.join("ond-unit.toml"), root).unwrap();
    // Run outside the config directory; project.path and child paths have different bases.
    let config_path = f.root.join("ond-unit.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_ond-unit"))
        .current_dir(f.root.join("checks"))
        .arg(&config_path)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{text}");
    assert!(
        text.contains("PASS=5 FAIL=0 STEP_LIMIT=0 ERROR=0"),
        "{text}"
    );
    assert!(text.find("inline: PASS").unwrap() < text.find("second: PASS").unwrap());
    assert!(text.find("second: PASS").unwrap() < text.find("fault: PASS").unwrap());
    assert!(f.run(&["--case", "second"], 0).contains("second: PASS"));
    let generated = f.root.join(".ond/target/ond-unit/main.ond");
    let before = fs::read(&generated).unwrap();
    for child in [
        observations.clone() + "\n[[cases]]\nname='invalid'\nfunction='Pass'\nmax_steps=0\n",
        observations.clone() + "\n[[cases]]\nname='wrong-type'\nfunction='WrongSignature'\n",
    ] {
        fs::write(f.root.join("parts/observations.toml"), &child).unwrap();
        let text = f.run(&["--case", "second"], 2);
        assert!(!text.contains("steps="), "{text}");
        if child.contains("max_steps=0") {
            assert_eq!(fs::read(&generated).unwrap(), before);
        } else {
            assert!(text.contains("ondビルド失敗"), "{text}");
        }
    }
    fs::remove_file(f.root.join("parts/observations.toml")).unwrap();
    assert!(!f.run(&["--case", "second"], 2).contains("steps="));
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn work_dir_uses_root_config_directory_from_another_current_directory() {
    let f = Fixture::new();
    let cases = case("pass", "Pass")
        + &case("fault", "Fault")
        + "expect_fault={ond_kind='ExplicitTrap',function='checks/cases.Fault'}\n";
    let absolute = fs::canonicalize(&f.root).unwrap().join("absolute-runner");
    let ordinary_absolute = std::path::absolute(f.root.join("ordinary-absolute-runner")).unwrap();
    #[cfg(windows)]
    assert!(!ordinary_absolute.to_str().unwrap().starts_with(r"\\?\"));
    let parent_relative = format!(
        "../{}-runner",
        f.root.file_name().unwrap().to_str().unwrap()
    );
    for work_dir in [
        None,
        Some(".ond/target/ond-unit"),
        Some("custom/runner"),
        Some(parent_relative.as_str()),
        Some(ordinary_absolute.to_str().unwrap()),
        Some(absolute.to_str().unwrap()),
    ] {
        f.config(&cases);
        let config_path = f.root.join("ond-unit.toml");
        if let Some(work_dir) = work_dir {
            let text = fs::read_to_string(&config_path).unwrap().replace(
                "version=1",
                &format!(
                    "version=1\nwork_dir={} ",
                    toml::Value::String(work_dir.into())
                ),
            );
            fs::write(&config_path, text).unwrap();
        }
        let generated = f.root.join(work_dir.unwrap_or(".ond/target/ond-unit"));
        for _ in 0..2 {
            let output = Command::new(env!("CARGO_BIN_EXE_ond-unit"))
                .current_dir(f.root.join("checks/cases"))
                .arg(&config_path)
                .output()
                .unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.status.success(), "{work_dir:?}: {text}");
            assert!(
                text.contains("PASS=2 FAIL=0 STEP_LIMIT=0 ERROR=0"),
                "{text}"
            );
            for file in [
                "ond.toml",
                "main.ond",
                "ond.lock",
                ".ond/target/build.ondbuild",
                ".ond/target/a.bin",
                ".ond/target/a.onddebug",
            ] {
                assert!(
                    generated.join(file).is_file(),
                    "missing {}",
                    generated.join(file).display()
                );
            }
            assert!(generated.join(".ond/target/ond-cache").is_dir());
            assert!(!f.root.join("checks/.ond/target/ond-unit").exists());
        }
    }
    // 同じ作業領域を別の対象projectへ切り替えても、生成物を再利用できる。
    fs::create_dir_all(f.root.join("other-checks/cases")).unwrap();
    fs::copy(
        f.root.join("checks/ond.toml"),
        f.root.join("other-checks/ond.toml"),
    )
    .unwrap();
    fs::copy(
        f.root.join("checks/cases/api.ond"),
        f.root.join("other-checks/cases/api.ond"),
    )
    .unwrap();
    let config_path = f.root.join("ond-unit.toml");
    let text = fs::read_to_string(&config_path)
        .unwrap()
        .replace("path='checks'", "path='other-checks'");
    fs::write(&config_path, text).unwrap();
    assert!(f.run(&[], 0).contains("PASS=2"));
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(absolute.join("ond.toml")).unwrap()).unwrap();
    assert_eq!(
        manifest["dependencies"]["subject"]["path"].as_str(),
        fs::canonicalize(f.root.join("other-checks"))
            .unwrap()
            .to_str()
    );
    assert_eq!(
        fs::read_to_string(f.root.join("checks/ond.toml")).unwrap(),
        include_str!("fixtures/checks/ond.toml")
    );
    assert_eq!(
        fs::read_to_string(f.root.join("checks/cases/api.ond")).unwrap(),
        include_str!("fixtures/checks/cases/api.ond")
    );
}

#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn work_dir_input_collisions_leave_existing_inputs_unchanged() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("other")).unwrap();
    fs::write(
        f.root.join("other/ond.toml"),
        "[project]\nname='other'\nfinal=true\n",
    )
    .unwrap();
    fs::write(
        f.root.join("other/main.ond"),
        "package other\nfunc main() {}\n",
    )
    .unwrap();
    let inputs = [
        "checks/ond.toml",
        "checks/cases/api.ond",
        "other/ond.toml",
        "other/main.ond",
    ];
    let before: Vec<_> = inputs
        .iter()
        .map(|path| fs::read(f.root.join(path)).unwrap())
        .collect();
    for work_dir in ["checks", "checks/cases", "other"] {
        f.config(&case("pass", "Pass"));
        let config_path = f.root.join("ond-unit.toml");
        let text = fs::read_to_string(&config_path)
            .unwrap()
            .replace("version=1", &format!("version=1\nwork_dir='{work_dir}'"));
        fs::write(&config_path, &text).unwrap();
        let result = f.run(&[], 2);
        assert!(result.contains("入力と重なっています"), "{result}");
        assert!(!result.contains("steps="), "{result}");
        assert_eq!(fs::read_to_string(&config_path).unwrap(), text);
        for (path, bytes) in inputs.iter().zip(&before) {
            assert_eq!(&fs::read(f.root.join(path)).unwrap(), bytes);
        }
        assert!(!f.root.join(work_dir).join(".ond").exists());
    }
    assert!(!f.root.join("checks/cases/ond.toml").exists());
}

#[cfg(windows)]
#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn work_dir_preserves_windows_case_aliases_of_existing_inputs() {
    for (name, bytes) in [
        ("OND.TOML", b"[project]\nname='original'\n".as_slice()),
        ("MAIN.OND", b"package original\nfunc main() {}\n".as_slice()),
        ("OND.LOCK", b"version=2\npackage=[]\n".as_slice()),
    ] {
        let f = Fixture::new();
        let directory = f.root.join("foreign");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join(name), bytes).unwrap();
        let manifest = fs::read(f.root.join("checks/ond.toml")).unwrap();
        let source = fs::read(f.root.join("checks/cases/api.ond")).unwrap();
        f.config(&case("pass", "Pass"));
        let config_path = f.root.join("ond-unit.toml");
        let config = fs::read_to_string(&config_path)
            .unwrap()
            .replace("version=1", "version=1\nwork_dir='foreign'");
        fs::write(&config_path, &config).unwrap();
        let result = f.run(&[], 2);
        assert!(result.contains("入力と重なっています"), "{name}: {result}");
        assert!(!result.contains("steps="), "{result}");
        assert_eq!(fs::read(directory.join(name)).unwrap(), bytes);
        let entries: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, [std::ffi::OsString::from(name)]);
        assert_eq!(fs::read_to_string(&config_path).unwrap(), config);
        assert_eq!(fs::read(f.root.join("checks/ond.toml")).unwrap(), manifest);
        assert_eq!(
            fs::read(f.root.join("checks/cases/api.ond")).unwrap(),
            source
        );
        assert!(!f.root.join(".ond").exists());
        assert!(!f.root.join("checks/.ond").exists());
    }
}

#[cfg(windows)]
#[test]
#[ignore = "通常のond executableをPATHへ配置して実行する"]
fn work_dir_rejects_drive_relative_paths_before_creating_files() {
    let f = Fixture::new();
    let root = fs::canonicalize(&f.root).unwrap();
    let drive = match root.components().next().unwrap() {
        std::path::Component::Prefix(prefix) => match prefix.kind() {
            std::path::Prefix::Disk(drive) | std::path::Prefix::VerbatimDisk(drive) => {
                char::from(drive)
            }
            _ => panic!("test fixture requires a drive path"),
        },
        _ => panic!("test fixture requires a drive path"),
    };
    let manifest = fs::read(f.root.join("checks/ond.toml")).unwrap();
    let source = fs::read(f.root.join("checks/cases/api.ond")).unwrap();
    for name in ["cwd-one", "cwd-two"] {
        fs::create_dir(f.root.join(name)).unwrap();
    }
    for work_dir in [format!("{drive}:runner"), format!("{drive}:")] {
        f.config(&case("pass", "Pass"));
        let config_path = f.root.join("ond-unit.toml");
        let config = fs::read_to_string(&config_path)
            .unwrap()
            .replace("version=1", &format!("version=1\nwork_dir='{work_dir}'"));
        fs::write(&config_path, &config).unwrap();
        for name in ["cwd-one", "cwd-two"] {
            let cwd = f.root.join(name);
            let output = Command::new(env!("CARGO_BIN_EXE_ond-unit"))
                .current_dir(&cwd)
                .arg(&config_path)
                .output()
                .unwrap();
            let result = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.status.code(), Some(2), "{work_dir}: {result}");
            assert!(result.contains("work_dirのWindowsドライブ指定"), "{result}");
            assert!(!result.contains("steps="), "{result}");
            assert!(fs::read_dir(&cwd).unwrap().next().is_none());
            assert_eq!(fs::read_to_string(&config_path).unwrap(), config);
            assert_eq!(fs::read(f.root.join("checks/ond.toml")).unwrap(), manifest);
            assert_eq!(
                fs::read(f.root.join("checks/cases/api.ond")).unwrap(),
                source
            );
            assert!(!f.root.join("runner").exists());
            assert!(!f.root.join(".ond").exists());
            assert!(!f.root.join("checks/.ond").exists());
        }
    }
}
