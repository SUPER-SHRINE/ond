# Ond

Ondは、Kagura v1向けのbackendとlinkerを備えたプログラミング言語です。\
独立コンパイラCLI、Language Server、言語仕様、利用者向けマニュアルを収録しています。

## 対応環境

CLIの配布targetはWindows x64とLinux x86_64（glibc）です。\
macOS、ARM64、Alpine/muslはtarget契約の対象外です。

## はじめに

[Ond言語マニュアル](docs/manual/README.md)では、最小プロジェクトの作成から言語機能、package構成まで順に説明しています。\
[最初のプロジェクト](docs/manual/getting-started.md)から始めてください。\
同じ内容のサンプルは[`examples/hello`](examples/hello)にあります。

CLIをソースからbuildする手順は「はじめに」を参照してください。\
Language Serverを直接起動する場合は`ond-lsp stdio`を実行します。\
VS Codeでは[Ond拡張の案内](https://github.com/SUPER-SHRINE/ond-vscode-extension/blob/main/README.md)を参照してください。\
ソースからCLIをbuildするには、repository rootで次を実行します。

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

OndはCLI/APIから直接利用できます。\
SDKは特定の実行環境向けにライブラリ・ツール・文書をまとめた開発キットという提供形態であり、機種固有の実行環境・loader・配布ツールの役割は[責務の定義](specs/ond/build/packages.md#責務)を参照してください。\
CLIのcompile、link、依存解決、成果物、cacheの規則は[パッケージビルド仕様](specs/ond/build/commands.md)を参照してください。

## 仕様と制約

- [Ond言語マニュアル](docs/manual/README.md) — 学習順と実用例
- [規範仕様](specs/README.md) — 構文、意味論、target契約
- [Kagura v1 backend仕様](specs/ond/targets/kagura-v1/README.md) — ABI、memory layout、実行形式

`ond link`には、呼出側が決めるRAM範囲、stack size、正常終了後の戻り先を指定する必要があります。\
言語、ABI、MIR、Object／LinkedImage形式、Rust公開APIの後方互換性は保証されません。

## License

[MIT License](LICENSE.md)
