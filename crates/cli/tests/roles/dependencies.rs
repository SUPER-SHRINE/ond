//! ROLE-08〜10: 全登録edgeの設定検査と経路診断を公開CLIで観測する。
#[path = "../../src/role_tests/fixtures.rs"]
mod fixtures;
use fixtures::*;
use ond_protocol::CompilationManifest;
use std::{
    fs,
    process::{Command, Output},
};
fn run(f: &ProjectFixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ond"))
        .current_dir(&f.root)
        .args(["compile", ".", "-o", "result.ondbuild"])
        .args(args)
        .output()
        .unwrap()
}
fn accept(f: &ProjectFixture, args: &[&str]) -> CompilationManifest {
    let output = run(f, args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&fs::read(f.root.join("result.ondbuild")).unwrap()).unwrap()
}
fn control(f: &ProjectFixture) {
    f.runner("[project]\nname=\"runner\"\n[dependencies]\nchild=\"../child\"\n");
    f.put("child/ond.toml", "[project]\nname=\"child\"\n");
    f.put(
        "child/lib.ond",
        "package child\nfunc Value[T](x:T)->i32{return 10}\n",
    );
    accept(f, &[]);
    f.reset();
}
fn reject(f: &ProjectFixture, id: &str, args: &[&str], causes: &[&str]) {
    let before = fs::read(f.root.join("result.ondbuild")).unwrap();
    let output = run(f, args);
    assert_eq!(output.status.code(), Some(1), "{id}: {output:?}");
    let diagnostic = String::from_utf8_lossy(&output.stderr).to_lowercase();
    for cause in causes {
        assert!(
            diagnostic.contains(&cause.to_lowercase()),
            "{id}: missing cause {cause}: {diagnostic}"
        );
    }
    assert_eq!(
        fs::read(f.root.join("result.ondbuild")).unwrap(),
        before,
        "{id}: preserve successful output"
    );
}

#[test]
fn role_08_09_final_bool_and_edge_input_matrix() {
    for (id, final_setting, dependency, target_roles, source, args, accepts, causes) in [
        (
            "ROLE-08.final-bool",
            "final=true",
            "{path=\"../child\",role=\"Test\"}",
            true,
            MAIN,
            vec![],
            true,
            vec![],
        ),
        (
            "ROLE-08.final-false",
            "final=false",
            "{path=\"../child\"}",
            true,
            MAIN,
            vec![],
            true,
            vec![],
        ),
        (
            "ROLE-08.final-default",
            "",
            "{path=\"../child\"}",
            true,
            MAIN,
            vec![],
            true,
            vec![],
        ),
        (
            "ROLE-08.final-type",
            "final=\"true\"",
            "{path=\"../child\",role=\"Test\"}",
            true,
            MAIN,
            vec![],
            false,
            vec!["runner", "final"],
        ),
        (
            "ROLE-09.nonfinal-default-role",
            "final=false",
            "{path=\"../child\",role=\"Test\"}",
            true,
            MAIN,
            vec![],
            false,
            vec!["runner", "child", "role"],
        ),
        (
            "ROLE-09.string-dependency",
            "final=false",
            "\"../child\"",
            true,
            MAIN,
            vec![],
            true,
            vec![],
        ),
        (
            "ROLE-09.unknown-role",
            "final=true",
            "{path=\"../child\",role=\"test\"}",
            true,
            MAIN,
            vec![],
            false,
            vec!["runner", "child", "test"],
        ),
        (
            "ROLE-09.role-type",
            "final=true",
            "{path=\"../child\",role=1}",
            true,
            MAIN,
            vec![],
            false,
            vec!["child", "role"],
        ),
        (
            "ROLE-09.roleless-target",
            "final=true",
            "{path=\"../child\",role=\"Test\"}",
            false,
            MAIN,
            vec![],
            false,
            vec!["runner", "child", "role"],
        ),
        (
            "ROLE-09.roleless-default",
            "final=true",
            "{path=\"../child\"}",
            false,
            MAIN,
            vec![],
            true,
            vec![],
        ),
        (
            "ROLE-09.unimported-edge",
            "final=false",
            "{path=\"../child\",role=\"Test\"}",
            true,
            NO_IMPORT,
            vec![],
            false,
            vec!["runner", "child", "role"],
        ),
    ] {
        let f = ProjectFixture::new();
        control(&f);
        f.runner(&format!(
            "[project]\nname=\"runner\"\n{final_setting}\n[dependencies]\nchild={dependency}\n"
        ));
        f.put("runner/main.ond", source);
        if !target_roles {
            f.put("child/ond.toml", "[project]\nname=\"child\"\n");
            f.put(
                "child/lib.ond",
                "package child\nfunc Value[T](x:T)->i32{return 10}\n",
            );
        }
        if accepts {
            accept(&f, &args);
        } else {
            reject(&f, id, &args, &causes);
        }
    }
}
#[test]
fn role_09_root_selection_does_not_change_edge_authority() {
    for (id, is_final, args, accepts) in [
        ("ROLE-09.root-primary", true, vec![], true),
        (
            "ROLE-09.root-nonprimary",
            true,
            vec!["--role", "Debug"],
            true,
        ),
        ("ROLE-09.nonfinal-own-primary", false, vec![], false),
        (
            "ROLE-09.nonfinal-own-nonprimary",
            false,
            vec!["--role", "Debug"],
            false,
        ),
    ] {
        let f = ProjectFixture::new();
        control(&f);
        f.runner(&format!(
            "{}[roles]\nnames=[\"Debug\",\"Release\"]\n",
            RUNNER.replace("final=true", &format!("final={is_final}"))
        ));
        if accepts {
            accept(&f, &args);
        } else {
            reject(&f, id, &args, &["runner", "child", "role"]);
        }
    }
    let f = ProjectFixture::new();
    control(&f);
    f.runner(&format!(
        "{}[roles]\nnames=[\"Debug\",\"Release\"]\n",
        RUNNER.replace("final=true", "final=false")
    ));
    f.put(
        "runner/main.ond",
        "package main\nrole Debug {import \"child\"}\nfunc main(){}\n",
    );
    reject(
        &f,
        "ROLE-09.inactive-import",
        &[],
        &["runner", "child", "role"],
    );
}
#[test]
fn role_08_final_target_is_rejected_at_every_registration() {
    for (id, alias, source) in [
        ("ROLE-08.final-root-reference", "child", MAIN),
        (
            "ROLE-08.final-subpackage-reference",
            "child",
            "package main\nimport \"child/sub\"\nvar Result:i32\nfunc main(){Result=sub.Read()}\n",
        ),
        ("ROLE-08.final-unimported", "child", NO_IMPORT),
        ("ROLE-08.final-alias", "alternate", MAIN),
    ] {
        let f = ProjectFixture::new();
        control(&f);
        f.put(
            "child/ond.toml",
            &CHILD.replace("name=\"child\"", "name=\"child\"\nfinal=true"),
        );
        f.put(
            "child/sub/s.ond",
            "package sub\nfunc Read()->i32{return 30}\n",
        );
        f.runner(&RUNNER.replace("child={", &format!("{alias}={{")));
        f.put("runner/main.ond", source);
        reject(&f, id, &[], &["runner", alias, "child", "final"]);
    }
}
#[test]
fn role_10_conflicting_paths_and_aliases_are_both_diagnosed() {
    for (id, unimported, alias) in [
        ("ROLE-10.direct-transitive-conflict", false, false),
        ("ROLE-10.unimported-conflict", true, false),
        ("ROLE-10.same-directory-alias", true, true),
    ] {
        let f = ProjectFixture::new();
        control(&f);
        f.diamond();
        if unimported {
            f.put("runner/main.ond", NO_IMPORT);
        }
        if alias {
            let text = fs::read_to_string(f.root.join("ond.toml")).unwrap();
            f.runner(&text.replace("middle={path=\"../bridge\"}", "middle={path=\"../child\"}"));
        }
        let mut causes = vec!["runner", "child", "direct", "middle", "Test", "role"];
        if !alias {
            causes.extend(["bridge", "leaf"]);
        }
        reject(&f, id, &[], &causes);
    }
}
#[test]
fn role_10_agreement_diamond_and_final_intermediate() {
    let f = ProjectFixture::new();
    control(&f);
    f.diamond();
    let manifest = fs::read_to_string(f.root.join("ond.toml")).unwrap();
    f.runner(&manifest.replace(",role=\"Test\"", ""));
    accept(&f, &[]);
    f.diamond();
    f.runner(&manifest.replace(
        "middle={path=\"../bridge\"}",
        "middle={path=\"../child\",role=\"Test\"}",
    ));
    f.put("runner/main.ond", MAIN);
    accept(&f, &[]);
    f.diamond();
    f.put("bridge/ond.toml","[project]\nname=\"bridge\"\nfinal=true\n[dependencies]\nleaf={path=\"../child\",role=\"Test\"}\n");
    reject(
        &f,
        "ROLE-08.final-intermediate",
        &[],
        &["runner", "middle", "bridge", "final"],
    );
}
#[test]
fn role_08_final_git_target_uses_a_local_fixture() {
    let f = ProjectFixture::new();
    control(&f);
    f.put(
        "child/ond.toml",
        &CHILD.replace("name=\"child\"", "name=\"child\"\nfinal=true"),
    );
    f.put(
        "child/sub/s.ond",
        "package sub\nfunc Read()->i32{return 30}\n",
    );
    let root = f.base.join("child");
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.name", "Ond role fixture"],
        vec!["config", "user.email", "ond-role-fixture@example.invalid"],
        vec!["add", "."],
        vec!["commit", "--quiet", "-m", "fixture"],
        vec!["tag", "fixture"],
    ] {
        let status = Command::new("git")
            .current_dir(&root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }
    let url = format!(
        "file:///{}",
        root.canonicalize()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches("\\\\?\\")
            .replace('\\', "/")
    );
    f.runner(&format!("[project]\nname=\"runner\"\nfinal=true\n[dependencies]\nchild={{git={url:?},rev=\"fixture\",role=\"Test\"}}\n"));
    // Path-only control writes an empty lock; the Git case requires its specified no-lock state.
    fs::remove_file(f.root.join("ond.lock")).unwrap();
    reject(&f, "ROLE-08.final-git", &[], &["runner", "child", "final"]);
}
