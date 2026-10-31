# 最初のプログラム

**到達目標:** 最小projectをcompileし、実行fileとdebug sidecarを生成します。

## projectを作る

空のdirectoryで初期設定を作ります。

```sh
ond init
```

commandは`ond.toml`と空の`main`関数を持つ`main.ond`を作成します。\
設定のoptional fieldはcomment例として記載され、既定の出力先は`.ond/target/`です。\
このtutorialでは、生成された`main.ond`を次のprogramに置き換えます。

```ond title="main.ond"
package main

func main() {
    var answer: u32 = add(20, 22)
    _ = answer
}

func add(left: u32, right: u32) -> u32 {
    return left + right
}
```

manifestの各fieldについては[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。

## compileとlink

project directoryで次を実行します。

```sh
ond compile .
ond link --project .
```

両commandがエラーなく終了し、`.ond/target/a.bin`と`.ond/target/a.onddebug`が生成されます。

`.bin`はKagura規格を満たすVM上で実行するためのfileで、host OSから直接起動するファイルではありません。

entryへのload状態と実行環境の準備は[実行file仕様](../../../specs/ond/artifacts.md#ロード完了時の状態)を参照してください。

CLIの入力と出力は[CLI仕様](../../../specs/ond/build/commands.md)、配置設定は[`ond.toml`リファレンス](../reference/manifest.md)を参照してください。
