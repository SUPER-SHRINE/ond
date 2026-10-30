use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

const MANIFEST: &str = r#"# Ondのproject設定。必要に応じてproject名を編集してください。
[project]
# project名。依存import pathの先頭にも使います。
name = "app"
# projectをdependencyにできるかを指定します（既定値false）。trueではこのprojectをdependencyにできません。
# dependencyのroleを指定する場合はtrueに変更します。
# final = false

# 名前付きの追加構成。選択時は `ond compile --role NAME` を使います。
# [roles]
# names = ["Debug", "Development"]

# source探索から除外するdirectory（既定値: .git、.ond、target）。
# [resolve]
# ignore = [".git", ".ond", "target"]

# path依存の例。既存のOnd projectへのpathに置き換えてください。
# [dependencies.example]
# path = "../example"
# role = "Development" # 依存先がこのroleを宣言し、このprojectのfinal=trueの場合に指定できます。

# Git依存の例。使用前にURLとrevisionを実際の値へ置き換えてください。
# [dependencies.remote]
# git = "https://github.com/example/library.git"
# rev = "v1.0.0"
# role = "Development" # 依存先がこのroleを宣言し、このprojectのfinal=trueの場合に指定できます。

# compile成果物とpackage cacheの保存先（既定値）。
# [compile]
# output = "target/build.ondbuild"
# cache-dir = "target/ond-cache"

# 初めてのprogram向けの例。外部loaderが実行fileをRAMへcopyする想定です。
# 数値はsampleです。実行環境のmemory mapとstack要件に合わせてください。
[link]
# output = "target/a.bin" # 省略時の実行file保存先。target/a.onddebugも生成します。
# startup形式。externalは外部loaderが起動し、selfはROM上の前置処理から始めます。
startup = "external"
# stack領域の予約量（byte）。
stack-size = 8192

# ROMとRAMが別範囲にある実行環境向けの任意例。下記RAM範囲とは重なりません。
# [link.rom]
# ROM範囲の先頭addressと容量（byte）。self startupではbaseを0にします。
# base = 268435456
# size = 65536

[link.ram]
# RAM範囲の先頭addressと容量（byte）。
base = 4096
size = 65536

[link.exit]
# mainから正常復帰した後の動作をloopまたはjumpで指定します。
mode = "loop"
# loopはmainの正常復帰後に終了処理内で反復します。jumpではloopを置き換えてaddressを指定します。
# mode = "jump"
# address = 0 # jump先のabsolute address（byte、4-byte aligned）。0はsample値です。

# startupをselfへ変える場合はROM baseを0とし、RAMを移動して領域を重ねないでください。
"#;

const SOURCE: &str = "package main\n\nfunc main() {\n}\n";

pub fn run() -> Result<(), Vec<String>> {
    create_pair(
        Path::new("ond.toml"),
        MANIFEST,
        Path::new("main.ond"),
        SOURCE,
    )
}

fn create_pair(
    manifest_path: &Path,
    manifest: &str,
    source_path: &Path,
    source: &str,
) -> Result<(), Vec<String>> {
    for path in [manifest_path, source_path] {
        if fs::symlink_metadata(path).is_ok() {
            return Err(vec![format!("{} already exists", path.display())]);
        }
    }

    let mut created = Vec::new();
    for (path, contents) in [(manifest_path, manifest), (source_path, source)] {
        let file = OpenOptions::new().write(true).create_new(true).open(path);
        let result = match file {
            Ok(mut file) => {
                created.push(path);
                file.write_all(contents.as_bytes())
            }
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            for path in created {
                let _ = fs::remove_file(path);
            }
            return Err(vec![format!("{}: {error}", path.display())]);
        }
    }
    Ok(())
}
