# 最初のプログラム

**到達目標:** 最小projectをcompileし、実行fileとdebug sidecarを生成します。

## projectを作る

空のdirectoryで初期設定を作成します。

```sh
ond init
```

`ond.toml`と空の`main`関数を持つ`main.ond`が作られます。\
必要に応じてsourceへ処理を追加してください。\
optional fieldはcomment例として記載されています。

manifestの各fieldについては[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。

## compileとlink

project directoryで次を実行します。

```sh
ond compile .
ond link --project .
```

両commandがexit code `0`で終了し、`target/a.bin`と`target/a.onddebug`が生成されます。

`.bin`はKagura規格を満たすVM上で実行するためのfileで、host OSから直接起動するファイルではありません。

entryへのload状態と実行環境の準備は[実行file仕様](../../../specs/ond/artifacts.md#ロード完了時の状態)を参照してください。

CLIの入力と出力は[CLI仕様](../../../specs/ond/build/commands.md)、配置設定は[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。
