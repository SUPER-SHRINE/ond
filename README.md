# Ond

Ondは、Kagura v1向けのbackendとlinkerを備えたプログラミング言語です。\
独立コンパイラCLI、Language Server、言語仕様、利用者向けマニュアルを収録しています。

## リリースと対応環境

公開版0.1.3は[GitHub Releases](https://github.com/SUPER-SHRINE/ond/releases)から入手できます。\
Windows x64向けZIPまたはLinux x86_64（glibc）向けtar.gzとSHA-256ファイルを提供しています。\
各Rust packageはcrates.ioでは公開していません。

検証対象はWindows x64、Linux x86_64（glibc）、Rust 1.91.1です。\
Linux配布CIはUbuntu 22.04を使用します。\
macOS、ARM64、Alpine/muslは検証対象ではありません。

## はじめに

[Ond言語マニュアル](docs/manual/README.md)では、最小プロジェクトの作成から言語機能、package構成まで順に説明しています。\
[最初のプロジェクト](docs/manual/getting-started.md)から始めてください。\
同じ内容のサンプルは[`examples/hello`](examples/hello)にあります。

公開版のCLIを使う場合は、上記Releaseから対象OSのarchiveを取得して展開し、`ond`をPATHへ追加します。\
Language Serverを直接起動する場合は`ond-lsp stdio`を実行してください。\
VS Codeでは[Ond拡張](https://github.com/SUPER-SHRINE/ond-vscode-extension)を利用できます。\
正式配布VSIXは、検証済みOnd ReleaseのLanguage Server binaryを同梱します。

ソースからCLIをbuildするには、GitとRust 1.91.1を用意してrepository rootで実行します。

```console
cargo build -p ond-cli --bin ond --locked
```

## 開発

buildとtestは次のコマンドで実行できます。

```console
cargo build --workspace --locked
cargo test --workspace --locked
```

変更を完了する前の全体品質検査には次を使います。

```console
pwsh -NoProfile -File tools/quality.ps1 -Profile Full
```

compiler-coreからKagura backend、Object/linkerを経てLinkedImageを生成します。\
Enbuなどの機種SDKが起動、保存形式、asset配置を担当し、Ond workspaceには含まれません。\
Kagura Rust参照実装はCargoの開発用依存として公開repositoryから取得します。\
兄弟directoryへのcheckoutは不要です。

CLIは`ond compile [project]`と`ond link FILE`を提供します。\
通常の依存は`ond.toml`の`[dependencies]`で指定し、追加libraryは`--library-dir DIR`または`OND_PATH`で渡せます。\
Git依存はproject内の`.ond/dependencies`へ取得し、解決commitを`ond.lock`へ固定します。

CLIはpackage別成果物とproject manifestをcacheします。\
全packageのcacheが一致すれば型検査・HIR/MIR生成・Object生成を省略し、差分があれば型検査とHIR/MIR生成をproject全体で行って変更packageのObjectを生成します。\
古いcacheの自動回収は未実装です。\
厳密なCLI、依存解決、成果物、cacheの契約は[パッケージビルド仕様](specs/ond/package-build.md)を参照してください。

## 仕様と制約

- [Ond言語マニュアル](docs/manual/README.md) — 学習順と実用例
- [規範仕様](specs/README.md) — 構文、意味論、target契約
- [Kagura v1 backend仕様](specs/ond/targets/kagura-v1/mir-backend.md) — ABI、memory layout、実行形式
- [変更履歴](CHANGELOG.md) — versionごとの主な変更

Ond単体には機種固有の起動、ROM生成、asset配置、device APIは含まれません。\
これらはEnbuなどのSDKから提供されます。\
`ond link`には、機種SDKが決めるRAM範囲、stack size、正常終了後の戻り先を指定する必要があります。\
言語、ABI、MIR、Object／LinkedImage形式、Rust公開APIの後方互換性はまだ保証していません。

## License

[MIT License](LICENSE.md)
