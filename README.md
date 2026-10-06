# Ond

Ond言語仕様、compiler-core、Kagura backend／リンカ、独立コンパイラCLI、Language Serverを収録します。

## リリース状態

Ond 0.1.3は未リリースです。GitHub Releaseと配布物はまだありません。配布manifestと同梱ライセンス情報は
[0.1.3のリリースノート](docs/releases/0.1.3-notes.md)を参照してください。

0.1.0〜0.1.2の変更記録は移行前の履歴から引き継いだものです。対応するtagとReleaseはこのrepositoryへ移行していません。

言語、ABI、MIRの意味論、
Object／LinkedImage形式、Rust公開APIはまだ安定しておらず、0.2.0までに互換性を損なう変更が
入る可能性があります。0.1系列では、文書化された標準経路の再現性と重大な不具合の修正を
優先します。

検証対象はWindows x64とLinux x86_64（glibc）、Rust 1.91.1です。
Linuxの配布CIはUbuntu 22.04を使用します。ローカルビルドの実行要件はビルド環境に依存します。
macOS、ARM64、Alpine/muslは対象外です。Kagura Rust参照実装は0.1.0を使用します。

Ondを使ってプログラムを書く場合は、[Ond言語マニュアル](docs/manual/README.md)から読み始めてください。

厳密な構文・意味論・target契約は[規範仕様](specs/README.md)にあります。

## 入手方法

Release公開後はWindows x64向けの`ond.exe`と`ond-lsp.exe`をZIPで、
Linux x86_64向けの`ond`と`ond-lsp`をtar.gzで配布します。
同時に公開するSHA-256ファイルで内容を確認できます。Linuxでは`tar -xzf`で実行権限を保持して展開します。
0.1系列の各Rust packageはcrates.ioへ公開しません。

`ond-lsp.exe`は標準入出力を使うLanguage Serverです。直接起動する場合は`ond-lsp stdio`を使用します。
VS Codeでは別repositoryで配布するOnd拡張がLanguage Serverを起動します。

ソースからbuildする場合は、GitとRust 1.91.1を用意し、このrepositoryのrootで実行します。

```console
cargo build --workspace --locked
```

最小プロジェクトの作成からcompileまでの手順は
[マニュアルの「はじめに」](docs/manual/getting-started.md)を参照してください。
同じ内容のサンプルを[`examples/hello`](examples/hello)に収録しています。

## Development layout

Ond自体の開発・テストでは、公開済みの`kagura`リポジトリからCargo経由でRust参照実装を取得します。
参照するrelease tagは[`crates/compiler/Cargo.toml`](./crates/compiler/Cargo.toml)で固定します。
兄弟directoryへのcheckoutは不要です。EnbuはOndの依存先ではありません。

## Compiler and language server

```text
cargo build --workspace --locked
cargo test --workspace --locked
```

標準経路はsource → compiler-core（HIR/MIR・validator）→ Kagura backend → Object/linker → LinkedImageです。機種SDKが起動・保存形式・アセット配置を担当します。言語サーバーはcompiler-coreに直接依存し、Kaguraのコード生成制限を言語診断へ混ぜません。

VS Code拡張は兄弟の`ond-vscode-extension` repositoryで管理します。local開発時は、そのbuild scriptが
このrepositoryの`ond-lsp`をbuildしてVSIXへ同梱します。

`cargo build -p ond-cli --bin ond`で独立コマンドを生成できます。
`ond compile [project]`／`ond link FILE`は引数・ファイルで操作し、パッケージ別成果物を扱います。
追加ライブラリは`--library-dir DIR`、またはOS標準のパス区切りで複数指定できる
`OND_PATH`環境変数から探索できます。
通常の依存は`ond.toml`の`[dependencies]`へ相対／絶対pathまたはGit URLを記述します。
依存先は`[project] name = "package_name"`を宣言し、この名前がimport pathの先頭になります。
`[dependencies]`側の登録名を変更しても、依存先自身と利用側のimport pathは変わりません。
Git依存はproject内の`.ond/dependencies`へ取得し、解決commitを`ond.lock`へ固定します。
`-o -`で標準出力、`ond link -`で標準入力を使用できます。操作用JSONは不要です。
依存先変更時は依存元のObjectも再生成します。全件キャッシュ一致時は型検査以降を省略しますが、
不一致がある場合の型検査・MIR生成はまだプロジェクト全体が対象です。
詳細と残りの移行工程は[パッケージビルド契約](specs/ond/package-build.md)を参照してください。

## Enbu cartridges

別途配布されるEnbu CLIを使用します。

```text
enbu build <project-dir> <cartridge.bin>
enbu assets <project-dir> <output-enbu-dir>
enbu_windows_host.exe cartridge.bin
```

Enbu CLI・SDKの実装とテストはEnbu workspaceにあります。`ond build`／`ond assets`の互換aliasは設けません。sourceとmanifestは引き続きOnd／`ond.toml`です。

Enbu SDKは独立した`ond compile`／`ond link`プロセスを利用し、実行用Rust依存は`ond-protocol`だけです。
`ond`をPATHに置くか、SDKの`ONDC`環境変数で実行ファイルを指定します。
Ond側のCargo workspace・lockfileにEnbu依存はありません。

## 既知の制約

- 機種固有の起動、ROM生成、asset配置、device APIはOnd単体には含まれず、EnbuなどのSDKが必要です。
- `ond link`には機種SDKが決めるRAM範囲、stack size、正常終了後の戻り先を指定する必要があります。
- 差分buildでcache不一致がある場合、型検査とMIR生成はプロジェクト全体を対象にします。
- 古い依存cacheの自動回収とgarbage collectionは未実装です。
- macOS、ARM64、Alpine/muslは検証対象ではありません。
- 言語、ABI、MIR、Object／LinkedImage、Rust公開APIの後方互換性はまだ保証しません。

詳しい制約と実装状況は[パッケージビルド契約](specs/ond/package-build.md)および
[Kagura v1 backend仕様](specs/ond/targets/kagura-v1/mir-backend.md)を参照してください。

## リリース情報

- [変更履歴](CHANGELOG.md)
- [MIT License](LICENSE.md)
